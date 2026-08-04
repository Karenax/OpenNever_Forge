#!/usr/bin/env python3
"""Compare the synthetic Lot 1 fixture with neverwinter.nim CLI tools."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


def run_tool(executable: Path, *arguments: str, cwd: Path | None = None) -> str:
    completed = subprocess.run(
        [str(executable), *arguments],
        cwd=cwd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=30,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "no diagnostic output"
        raise RuntimeError(f"{executable.name} failed with code {completed.returncode}: {detail}")
    return completed.stdout


def gff_value(document: dict[str, Any], field: str) -> Any:
    value = document.get(field)
    return value.get("value") if isinstance(value, dict) else None


def add_check(checks: list[dict[str, Any]], name: str, expected: Any, actual: Any) -> None:
    checks.append({"name": name, "expected": expected, "actual": actual, "passed": actual == expected})


def parse_oracle_version(output: str) -> str:
    lines = [line.strip() for line in output.splitlines() if line.strip()]
    return next((line for line in lines if line.lower().startswith("neverwinter ")), lines[-1] if lines else "unknown")


def compare_payloads(
    manifest: dict[str, Any],
    module_entries: list[str],
    gff: dict[str, Any],
    tlk: dict[str, Any],
) -> list[dict[str, Any]]:
    expected = manifest["expected"]
    checks: list[dict[str, Any]] = []
    add_check(checks, "module.ifo listed", True, "module.ifo" in module_entries)
    add_check(checks, "GFF file type", "IFO ", gff.get("__data_type"))
    localized_name = gff_value(gff, "Mod_Name")
    module_name = localized_name.get("0") if isinstance(localized_name, dict) else None
    add_check(checks, "module name", expected["moduleName"], module_name)
    add_check(checks, "module tag", expected["moduleTag"], gff_value(gff, "Mod_Tag"))
    add_check(checks, "entry area", expected["entryArea"], gff_value(gff, "Mod_Entry_Area"))
    add_check(checks, "custom TLK", expected["customTlk"], gff_value(gff, "Mod_CustomTlk"))
    hak_list = gff_value(gff, "Mod_HakList") or []
    hak_names = [gff_value(item, "Mod_Hak") for item in hak_list if isinstance(item, dict)]
    add_check(checks, "HAK list", expected["hakFiles"], hak_names)
    entries = tlk.get("entries") if isinstance(tlk, dict) else None
    first_text = entries[0].get("text") if isinstance(entries, list) and entries else None
    add_check(checks, "custom TLK text", expected["customTlkText"], first_text)
    return checks


def compare_fixture(fixture_root: Path, oracle_directory: Path) -> dict[str, Any]:
    fixture_root = fixture_root.resolve()
    oracle_directory = oracle_directory.resolve()
    tools = {
        "erf": oracle_directory / "nwn_erf.exe",
        "gff": oracle_directory / "nwn_gff.exe",
        "tlk": oracle_directory / "nwn_tlk.exe",
    }
    missing = [str(path) for path in tools.values() if not path.is_file()]
    if missing:
        raise FileNotFoundError(f"missing neverwinter.nim tools: {', '.join(missing)}")

    manifest = json.loads((fixture_root / "manifest.json").read_text(encoding="utf-8"))
    module = fixture_root / "module" / "forge_lot1.mod"
    hak = fixture_root / "user" / "hak" / "forge_assets.hak"
    tlk = fixture_root / "user" / "tlk" / "forge_dialog.tlk"
    module_entries = [
        line.strip()
        for line in run_tool(tools["erf"], "--quiet", "-f", str(module), "-t").splitlines()
        if line.strip()
    ]
    run_tool(tools["erf"], "--quiet", "-f", str(hak), "-t")
    tlk_document = json.loads(run_tool(tools["tlk"], "--quiet", "-i", str(tlk), "-k", "json"))

    with tempfile.TemporaryDirectory(prefix="opennever-oracle-") as directory:
        extraction = Path(directory)
        run_tool(
            tools["erf"],
            "--quiet",
            "-f",
            str(module),
            "-x",
            "module.ifo",
            cwd=extraction,
        )
        gff_document = json.loads(
            run_tool(tools["gff"], "--quiet", "-i", str(extraction / "module.ifo"), "-k", "json")
        )

    checks = compare_payloads(manifest, module_entries, gff_document, tlk_document)
    version = parse_oracle_version(run_tool(tools["erf"], "--version"))
    return {
        "schemaVersion": 1,
        "oracle": "neverwinter.nim",
        "oracleVersion": version,
        "fixture": manifest["fixture"],
        "passed": all(check["passed"] for check in checks),
        "checks": checks,
    }


def main() -> int:
    repository = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fixture-root",
        type=Path,
        default=repository / "fixtures" / "synthetic" / "lot1_custom_tlk",
    )
    parser.add_argument("--oracle-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, help="optional JSON report path")
    arguments = parser.parse_args()

    try:
        report = compare_fixture(arguments.fixture_root, arguments.oracle_dir)
    except (FileNotFoundError, RuntimeError, json.JSONDecodeError, OSError) as error:
        print(f"ORACLE_COMPARISON_ERROR: {error}", file=sys.stderr)
        return 2

    rendered = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if arguments.output:
        arguments.output.write_text(rendered, encoding="utf-8", newline="\n")
    print(rendered, end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
