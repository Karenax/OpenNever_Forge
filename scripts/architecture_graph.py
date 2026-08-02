#!/usr/bin/env python3
"""Deterministic, local architecture graph for OpenNever Forge."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
from collections import Counter, deque
from pathlib import Path
from typing import Any, Iterable


SCHEMA_VERSION = 1
DEFAULT_GRAPH_PATH = Path("docs/architecture/graph.json")
SOURCE_ROOTS = (
    Path("apps/desktop/src"),
    Path("apps/desktop/src-tauri/src"),
    Path("crates"),
    Path("scripts"),
    Path("tests"),
    Path("fixtures/synthetic"),
)
SOURCE_SUFFIXES = {".py", ".rs", ".sql", ".ts", ".tsx"}
EXCLUDED_PARTS = {
    ".git",
    ".tmp",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "generated",
    "node_modules",
    "target",
    "vendor",
}
SECRET_NAMES = {".env", ".env.local", ".env.production"}
SECRET_SUFFIXES = {".db", ".key", ".model", ".onnx", ".p12", ".pem", ".sqlite", ".sqlite3"}

TS_IMPORT_RE = re.compile(r"(?:from\s+|import\s*)[\"']([^\"']+)[\"']")
TS_FUNCTION_RE = re.compile(r"^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_]\w*)", re.MULTILINE)
TS_CONST_RE = re.compile(r"^\s*export\s+const\s+([A-Za-z_]\w*)\s*=", re.MULTILINE)
TS_INVOKE_RE = re.compile(r"\binvoke(?:<[^>]+>)?\s*\(\s*[\"']([A-Za-z_]\w*)[\"']")
RUST_MOD_RE = re.compile(r"^\s*(?:pub\s+)?mod\s+([A-Za-z_]\w*)\s*;", re.MULTILINE)
RUST_USE_RE = re.compile(r"^\s*use\s+([A-Za-z_]\w*)::", re.MULTILINE)
RUST_TYPE_RE = re.compile(r"^\s*pub\s+(?:struct|enum|trait)\s+([A-Za-z_]\w*)", re.MULTILINE)
RUST_FUNCTION_RE = re.compile(r"^\s*pub\s+(?:async\s+)?fn\s+([A-Za-z_]\w*)", re.MULTILINE)
RUST_COMMAND_RE = re.compile(
    r"#\[tauri::command\]\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_]\w*)",
    re.MULTILINE,
)
RUST_TEST_RE = re.compile(r"#\[test\]\s*(?:pub\s+)?fn\s+([A-Za-z_]\w*)", re.MULTILINE)


def normalize_path(path: Path) -> str:
    return path.as_posix()


def line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def is_excluded(relative: Path) -> bool:
    if any(part in EXCLUDED_PARTS for part in relative.parts):
        return True
    if relative.name in SECRET_NAMES:
        return True
    return relative.suffix.lower() in SECRET_SUFFIXES


def iter_source_files(root: Path) -> list[Path]:
    files: set[Path] = set()
    for source_root in SOURCE_ROOTS:
        absolute = root / source_root
        if not absolute.exists():
            continue
        for path in absolute.rglob("*"):
            if not path.is_file():
                continue
            relative = path.relative_to(root)
            if is_excluded(relative) or path.suffix.lower() not in SOURCE_SUFFIXES:
                continue
            files.add(relative)
    return sorted(files, key=normalize_path)


def source_digest(root: Path, files: Iterable[Path]) -> str:
    digest = hashlib.sha256()
    for relative in files:
        digest.update(normalize_path(relative).encode("utf-8"))
        digest.update(b"\0")
        source = (root / relative).read_bytes()
        digest.update(source.replace(b"\r\n", b"\n").replace(b"\r", b"\n"))
        digest.update(b"\0")
    return f"sha256:{digest.hexdigest()}"


def layer_for(path: str, kind: str) -> str:
    if kind == "test" or ".test." in path or path.startswith("tests/"):
        return "tests"
    if path.startswith("apps/desktop/src-tauri/src/"):
        return "tauri"
    if path.startswith("apps/desktop/src/"):
        return "interface"
    if path.startswith("crates/aurora-core/"):
        return "core"
    if path.startswith("crates/aurora-project/"):
        return "project"
    if path.startswith("crates/aurora-index/"):
        return "index"
    if path.startswith("crates/"):
        return "domain"
    if path.startswith("scripts/"):
        return "tooling"
    return "other"


def file_kind(path: str) -> str:
    if ".test." in path or path.startswith("tests/"):
        return "test"
    if "/migrations/" in path and path.endswith(".sql"):
        return "migration"
    return "file"


def node(node_id: str, kind: str, name: str, path: str, line: int) -> dict[str, Any]:
    return {
        "id": node_id,
        "kind": kind,
        "name": name,
        "path": path,
        "line": line,
        "layer": layer_for(path, kind),
    }


def edge(source: str, target: str, kind: str, path: str, line: int) -> dict[str, Any]:
    return {
        "source": source,
        "target": target,
        "kind": kind,
        "evidence": {"path": path, "line": line},
    }


def add_node(nodes: dict[str, dict[str, Any]], value: dict[str, Any]) -> None:
    existing = nodes.get(value["id"])
    if existing is None or (value["path"], value["line"]) < (existing["path"], existing["line"]):
        nodes[value["id"]] = value


def resolve_ts_import(root: Path, source: Path, specifier: str) -> Path | None:
    if not specifier.startswith("."):
        return None
    candidate = (source.parent / specifier)
    attempts = [
        candidate,
        candidate.with_suffix(".ts"),
        candidate.with_suffix(".tsx"),
        candidate / "index.ts",
        candidate / "index.tsx",
    ]
    for attempt in attempts:
        absolute = root / attempt
        if absolute.is_file():
            return absolute.relative_to(root)
    return None


def rust_crates(root: Path) -> dict[str, Path]:
    result: dict[str, Path] = {}
    crates_root = root / "crates"
    if not crates_root.exists():
        return result
    for manifest in sorted(crates_root.glob("*/Cargo.toml")):
        text = manifest.read_text(encoding="utf-8")
        match = re.search(r'^name\s*=\s*"([^"]+)"', text, re.MULTILINE)
        if match:
            result[match.group(1).replace("-", "_")] = manifest.relative_to(root)
    return result


def resolve_rust_module(root: Path, source: Path, module_name: str) -> Path | None:
    candidates = [source.parent / f"{module_name}.rs", source.parent / module_name / "mod.rs"]
    for candidate in candidates:
        if (root / candidate).is_file():
            return candidate
    return None


def parse_typescript(
    root: Path,
    relative: Path,
    text: str,
    nodes: dict[str, dict[str, Any]],
    edges: list[dict[str, Any]],
) -> None:
    path = normalize_path(relative)
    file_id = f"file:{path}"

    for match in TS_IMPORT_RE.finditer(text):
        target = resolve_ts_import(root, relative, match.group(1))
        if target is None:
            continue
        target_path = normalize_path(target)
        edges.append(edge(file_id, f"file:{target_path}", "imports", path, line_number(text, match.start())))

    for pattern in (TS_FUNCTION_RE, TS_CONST_RE):
        for match in pattern.finditer(text):
            name = match.group(1)
            kind = "component" if name[0].isupper() else "client_operation"
            symbol_id = f"{kind}:{path}:{name}"
            add_node(nodes, node(symbol_id, kind, name, path, line_number(text, match.start())))
            edges.append(edge(symbol_id, file_id, "defined_in", path, line_number(text, match.start())))

    for match in TS_INVOKE_RE.finditer(text):
        command_name = match.group(1)
        command_id = f"tauri_command:{command_name}"
        if command_id not in nodes:
            add_node(
                nodes,
                node(command_id, "tauri_command", command_name, path, line_number(text, match.start())),
            )
        edges.append(edge(file_id, command_id, "invokes_command", path, line_number(text, match.start())))


def parse_rust(
    root: Path,
    relative: Path,
    text: str,
    crates: dict[str, Path],
    nodes: dict[str, dict[str, Any]],
    edges: list[dict[str, Any]],
) -> None:
    path = normalize_path(relative)
    file_id = f"file:{path}"

    for match in RUST_MOD_RE.finditer(text):
        target = resolve_rust_module(root, relative, match.group(1))
        if target is not None:
            edges.append(
                edge(
                    file_id,
                    f"file:{normalize_path(target)}",
                    "imports",
                    path,
                    line_number(text, match.start()),
                )
            )

    for match in RUST_USE_RE.finditer(text):
        crate_name = match.group(1)
        manifest = crates.get(crate_name)
        if manifest is None:
            continue
        crate_id = f"crate:{crate_name.replace('_', '-')}"
        manifest_path = normalize_path(manifest)
        add_node(nodes, node(crate_id, "crate", crate_name.replace("_", "-"), manifest_path, 1))
        edges.append(edge(file_id, crate_id, "imports", path, line_number(text, match.start())))

    command_offsets: set[tuple[str, int]] = set()
    for match in RUST_COMMAND_RE.finditer(text):
        name = match.group(1)
        command_offsets.add((name, match.start()))
        command_id = f"tauri_command:{name}"
        add_node(nodes, node(command_id, "tauri_command", name, path, line_number(text, match.start())))
        edges.append(edge(command_id, file_id, "defined_in", path, line_number(text, match.start())))

    for pattern, kind in ((RUST_TYPE_RE, "domain_type"), (RUST_FUNCTION_RE, "rust_function")):
        for match in pattern.finditer(text):
            name = match.group(1)
            if kind == "rust_function" and any(command == name for command, _ in command_offsets):
                continue
            symbol_id = f"{kind}:{path}:{name}"
            add_node(nodes, node(symbol_id, kind, name, path, line_number(text, match.start())))
            edges.append(edge(symbol_id, file_id, "defined_in", path, line_number(text, match.start())))

    for match in RUST_TEST_RE.finditer(text):
        name = match.group(1)
        test_id = f"test:{path}:{name}"
        add_node(nodes, node(test_id, "test", name, path, line_number(text, match.start())))
        edges.append(edge(test_id, file_id, "tests", path, line_number(text, match.start())))


def build_graph(root: Path) -> dict[str, Any]:
    root = root.resolve()
    files = iter_source_files(root)
    crates = rust_crates(root)
    nodes: dict[str, dict[str, Any]] = {}
    edges: list[dict[str, Any]] = []

    for relative in files:
        path = normalize_path(relative)
        kind = file_kind(path)
        add_node(nodes, node(f"file:{path}", kind, relative.name, path, 1))

    for relative in files:
        absolute = root / relative
        try:
            text = absolute.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        suffix = relative.suffix.lower()
        if suffix in {".ts", ".tsx"}:
            parse_typescript(root, relative, text, nodes, edges)
        elif suffix == ".rs":
            parse_rust(root, relative, text, crates, nodes, edges)

    known_ids = set(nodes)
    edges = [value for value in edges if value["source"] in known_ids and value["target"] in known_ids]
    unique_edges = {
        (
            value["source"],
            value["target"],
            value["kind"],
            value["evidence"]["path"],
            value["evidence"]["line"],
        ): value
        for value in edges
    }

    return {
        "metadata": {
            "schema_version": SCHEMA_VERSION,
            "source_digest": source_digest(root, files),
            "source_file_count": len(files),
        },
        "nodes": sorted(nodes.values(), key=lambda value: value["id"]),
        "edges": sorted(
            unique_edges.values(),
            key=lambda value: (
                value["source"],
                value["target"],
                value["kind"],
                value["evidence"]["path"],
                value["evidence"]["line"],
            ),
        ),
    }


def graph_bytes(graph: dict[str, Any]) -> bytes:
    return (json.dumps(graph, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def mermaid_overview(graph: dict[str, Any]) -> str:
    layer_counts = Counter(value["layer"] for value in graph["nodes"])
    layer_edges: dict[tuple[str, str], set[str]] = {}
    nodes_by_id = {value["id"]: value for value in graph["nodes"]}
    for relation in graph["edges"]:
        source = nodes_by_id[relation["source"]]["layer"]
        target = nodes_by_id[relation["target"]]["layer"]
        if source == target:
            continue
        layer_edges.setdefault((source, target), set()).add(relation["kind"])

    layer_labels = {
        "interface": "Interface React",
        "tauri": "API Tauri",
        "core": "Types et erreurs",
        "project": "Projets et jobs",
        "index": "Index SQLite",
        "domain": "Domaine NWN",
        "tests": "Tests",
        "tooling": "Outillage",
        "other": "Autres",
    }
    layers = sorted(layer_counts)
    identifiers = {layer: f"layer_{index}" for index, layer in enumerate(layers)}
    lines = ["flowchart LR"]
    for layer in layers:
        label = layer_labels.get(layer, layer.title())
        lines.append(f'  {identifiers[layer]}["{label} ({layer_counts[layer]})"]')
    for (source, target), kinds in sorted(layer_edges.items()):
        lines.append(
            f'  {identifiers[source]} -->|"{", ".join(sorted(kinds))}"| {identifiers[target]}'
        )
    return "\n".join(lines) + "\n"


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as temporary:
            temporary.write(content)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_name, path)
    finally:
        if os.path.exists(temporary_name):
            os.unlink(temporary_name)


def artifact_paths(root: Path, output: Path) -> tuple[Path, Path]:
    graph_path = output if output.is_absolute() else root / output
    return graph_path, graph_path.with_name("overview.mmd")


def generate(root: Path, output: Path) -> dict[str, Any]:
    graph = build_graph(root)
    graph_path, overview_path = artifact_paths(root, output)
    write_atomic(graph_path, graph_bytes(graph))
    write_atomic(overview_path, mermaid_overview(graph).encode("utf-8"))
    return graph


def check(root: Path, output: Path) -> tuple[bool, list[str]]:
    graph = build_graph(root)
    graph_path, overview_path = artifact_paths(root, output)
    expected = {
        graph_path: graph_bytes(graph),
        overview_path: mermaid_overview(graph).encode("utf-8"),
    }
    stale = []
    for path, content in expected.items():
        if not path.exists() or path.read_bytes() != content:
            stale.append(normalize_path(path.relative_to(root)))
    return not stale, stale


def query_graph(
    graph: dict[str, Any],
    search: str,
    depth: int = 1,
    max_nodes: int = 40,
) -> dict[str, Any]:
    needle = search.casefold().strip()
    nodes_by_id = {value["id"]: value for value in graph["nodes"]}
    matched = [
        value["id"]
        for value in graph["nodes"]
        if needle
        and needle
        in " ".join(
            str(value.get(field, "")) for field in ("id", "kind", "name", "path", "layer")
        ).casefold()
    ]
    adjacency: dict[str, set[str]] = {node_id: set() for node_id in nodes_by_id}
    for relation in graph["edges"]:
        adjacency[relation["source"]].add(relation["target"])
        adjacency[relation["target"]].add(relation["source"])

    selected: list[str] = []
    seen: set[str] = set()
    queue = deque((node_id, 0) for node_id in sorted(matched))
    while queue and len(selected) < max_nodes:
        node_id, current_depth = queue.popleft()
        if node_id in seen:
            continue
        seen.add(node_id)
        selected.append(node_id)
        if current_depth >= depth:
            continue
        for neighbor in sorted(adjacency.get(node_id, set())):
            if neighbor not in seen:
                queue.append((neighbor, current_depth + 1))

    selected_set = set(selected)
    return {
        "query": search,
        "nodes": [nodes_by_id[node_id] for node_id in selected],
        "edges": [
            relation
            for relation in graph["edges"]
            if relation["source"] in selected_set and relation["target"] in selected_set
        ],
    }


def format_query(result: dict[str, Any], output_format: str) -> str:
    if output_format == "json":
        return json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    if output_format == "paths":
        paths = {value["path"] for value in result["nodes"] if value.get("path")}
        paths.update(relation["evidence"]["path"] for relation in result["edges"])
        return ("\n".join(sorted(paths)) + "\n") if paths else ""
    if output_format == "mermaid":
        identifiers = {value["id"]: f"n{index}" for index, value in enumerate(result["nodes"])}
        lines = ["flowchart LR"]
        for value in result["nodes"]:
            label = f'{value["kind"]}: {value["name"]}'.replace('"', "'")
            lines.append(f'  {identifiers[value["id"]]}["{label}"]')
        for relation in result["edges"]:
            lines.append(
                f'  {identifiers[relation["source"]]} -->|"{relation["kind"]}"| '
                f'{identifiers[relation["target"]]}'
            )
        return "\n".join(lines) + "\n"

    if not result["nodes"]:
        return f'No architecture nodes matched "{result["query"]}".\n'
    lines = [f'Architecture query: {result["query"]}']
    for value in result["nodes"]:
        lines.append(f'- {value["kind"]} {value["name"]} ({value["path"]}:{value["line"]})')
    return "\n".join(lines) + "\n"


def add_common_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="Repository root")
    parser.add_argument("--output", type=Path, default=DEFAULT_GRAPH_PATH, help="Graph JSON path")


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)
    for command in ("generate", "check", "stats"):
        subcommand = commands.add_parser(command)
        add_common_arguments(subcommand)
    query = commands.add_parser("query")
    add_common_arguments(query)
    query.add_argument("search")
    query.add_argument("--depth", type=int, default=1)
    query.add_argument("--max-nodes", type=int, default=40)
    query.add_argument("--format", choices=("text", "paths", "json", "mermaid"), default="text")
    return result


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    root = arguments.root.resolve()
    if arguments.command == "generate":
        graph = generate(root, arguments.output)
        print(
            f'Generated {len(graph["nodes"])} nodes and {len(graph["edges"])} edges '
            f'in {arguments.output.as_posix()}.'
        )
        return 0
    if arguments.command == "check":
        fresh, stale = check(root, arguments.output)
        if fresh:
            print("Architecture graph is fresh.")
            return 0
        print("Architecture graph is missing or stale:", file=sys.stderr)
        for path in stale:
            print(f"- {path}", file=sys.stderr)
        print("Run: python scripts/architecture_graph.py generate", file=sys.stderr)
        return 1

    graph = build_graph(root)
    if arguments.command == "stats":
        node_counts = Counter(value["kind"] for value in graph["nodes"])
        edge_counts = Counter(value["kind"] for value in graph["edges"])
        print(
            json.dumps(
                {"nodes": dict(sorted(node_counts.items())), "edges": dict(sorted(edge_counts.items()))},
                indent=2,
                sort_keys=True,
            )
        )
        return 0

    result = query_graph(
        graph,
        arguments.search,
        depth=max(arguments.depth, 0),
        max_nodes=max(arguments.max_nodes, 1),
    )
    sys.stdout.write(format_query(result, arguments.format))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
