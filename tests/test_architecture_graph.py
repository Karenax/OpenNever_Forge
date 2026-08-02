from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "architecture_graph.py"
SPEC = importlib.util.spec_from_file_location("architecture_graph", SCRIPT)
assert SPEC and SPEC.loader
architecture_graph = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(architecture_graph)


class ArchitectureGraphTests(unittest.TestCase):
    def make_repository(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        (root / "apps/desktop/src/lib").mkdir(parents=True)
        (root / "apps/desktop/src-tauri/src").mkdir(parents=True)
        (root / "crates/aurora-core/src").mkdir(parents=True)
        (root / "crates/aurora-core/Cargo.toml").write_text(
            '[package]\nname = "aurora-core"\n', encoding="utf-8"
        )
        (root / "apps/desktop/src/lib/tauri.ts").write_text(
            'import { invoke } from "@tauri-apps/api/core";\n'
            'export function status() { return invoke("get_app_status"); }\n',
            encoding="utf-8",
        )
        (root / "apps/desktop/src/App.tsx").write_text(
            'import { status } from "./lib/tauri";\n'
            'export function App() { status(); return null; }\n',
            encoding="utf-8",
        )
        (root / "apps/desktop/src/App.test.tsx").write_text(
            'import { App } from "./App";\nexport const covered = App;\n', encoding="utf-8"
        )
        (root / "apps/desktop/src-tauri/src/commands.rs").write_text(
            '#[tauri::command]\npub fn get_app_status() {}\n', encoding="utf-8"
        )
        (root / "apps/desktop/src-tauri/src/lib.rs").write_text(
            'mod commands;\nuse aurora_core::AppError;\n', encoding="utf-8"
        )
        (root / "crates/aurora-core/src/lib.rs").write_text(
            'pub struct AppError;\n#[cfg(test)] mod tests { #[test] fn error_is_stable() {} }\n',
            encoding="utf-8",
        )
        return root

    def test_detects_vertical_path_with_evidence(self) -> None:
        graph = architecture_graph.build_graph(self.make_repository())
        command = next(node for node in graph["nodes"] if node["id"] == "tauri_command:get_app_status")
        self.assertEqual(command["path"], "apps/desktop/src-tauri/src/commands.rs")
        self.assertTrue(
            any(
                edge["kind"] == "invokes_command"
                and edge["target"] == "tauri_command:get_app_status"
                and edge["evidence"]["line"] == 2
                for edge in graph["edges"]
            )
        )

    def test_generation_is_byte_for_byte_deterministic(self) -> None:
        root = self.make_repository()
        first = architecture_graph.graph_bytes(architecture_graph.build_graph(root))
        second = architecture_graph.graph_bytes(architecture_graph.build_graph(root))
        self.assertEqual(first, second)

    def test_check_detects_a_modified_source_without_writing(self) -> None:
        root = self.make_repository()
        output = Path("docs/architecture/graph.json")
        architecture_graph.generate(root, output)
        self.assertEqual(architecture_graph.check(root, output), (True, []))
        graph_before = (root / output).read_bytes()

        app = root / "apps/desktop/src/App.tsx"
        app.write_text(app.read_text(encoding="utf-8") + "\nexport const changed = true;\n", encoding="utf-8")
        fresh, stale = architecture_graph.check(root, output)

        self.assertFalse(fresh)
        self.assertIn("docs/architecture/graph.json", stale)
        self.assertEqual((root / output).read_bytes(), graph_before)

    def test_query_is_bounded_and_supports_all_formats(self) -> None:
        graph = architecture_graph.build_graph(self.make_repository())
        result = architecture_graph.query_graph(graph, "get_app_status", depth=2, max_nodes=3)
        self.assertLessEqual(len(result["nodes"]), 3)
        self.assertIn("commands.rs", architecture_graph.format_query(result, "paths"))
        self.assertIn('"nodes"', architecture_graph.format_query(result, "json"))
        self.assertIn("flowchart LR", architecture_graph.format_query(result, "mermaid"))
        self.assertIn("Architecture query", architecture_graph.format_query(result, "text"))

    def test_excludes_local_data_and_build_outputs(self) -> None:
        root = self.make_repository()
        (root / "apps/desktop/src/target").mkdir()
        (root / "apps/desktop/src/target/secret.rs").write_text("pub fn secret() {}", encoding="utf-8")
        (root / "apps/desktop/src/cache.sqlite").write_bytes(b"database")

        paths = {node["path"] for node in architecture_graph.build_graph(root)["nodes"]}
        self.assertNotIn("apps/desktop/src/target/secret.rs", paths)
        self.assertNotIn("apps/desktop/src/cache.sqlite", paths)


if __name__ == "__main__":
    unittest.main()
