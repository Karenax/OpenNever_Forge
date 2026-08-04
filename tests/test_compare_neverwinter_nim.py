from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "tools" / "compare-oracles" / "compare_neverwinter_nim.py"
SPEC = importlib.util.spec_from_file_location("compare_neverwinter_nim", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class NeverwinterNimComparisonTests(unittest.TestCase):
    def test_version_is_extracted_after_the_embedded_licence(self) -> None:
        output = "LICENCE\n=======\nMIT text\n\nneverwinter 2.1.2 (/07a475, nim 2.2.4)\n"

        self.assertEqual(
            MODULE.parse_oracle_version(output),
            "neverwinter 2.1.2 (/07a475, nim 2.2.4)",
        )

    def test_matching_oracle_payload_passes_all_checks(self) -> None:
        checks = MODULE.compare_payloads(
            manifest(),
            ["module.ifo"],
            gff_document("OPENNEVER_LOT1"),
            {"entries": [{"id": 0, "text": "Synthetic TLK"}]},
        )

        self.assertTrue(all(check["passed"] for check in checks))
        self.assertEqual(len(checks), 8)

    def test_divergence_is_reported_without_treating_the_oracle_as_truth(self) -> None:
        checks = MODULE.compare_payloads(
            manifest(),
            ["module.ifo"],
            gff_document("DIFFERENT_TAG"),
            {"entries": [{"id": 0, "text": "Synthetic TLK"}]},
        )

        mismatch = next(check for check in checks if check["name"] == "module tag")
        self.assertFalse(mismatch["passed"])
        self.assertEqual(mismatch["expected"], "OPENNEVER_LOT1")
        self.assertEqual(mismatch["actual"], "DIFFERENT_TAG")


def manifest() -> dict[str, object]:
    return {
        "expected": {
            "moduleName": "Synthetic Module",
            "moduleTag": "OPENNEVER_LOT1",
            "entryArea": "startarea",
            "hakFiles": ["forge_assets"],
            "customTlk": "forge_dialog",
            "customTlkText": "Synthetic TLK",
        }
    }


def gff_document(tag: str) -> dict[str, object]:
    return {
        "__data_type": "IFO ",
        "Mod_Name": {"value": {"0": "Synthetic Module"}},
        "Mod_Tag": {"value": tag},
        "Mod_Entry_Area": {"value": "startarea"},
        "Mod_CustomTlk": {"value": "forge_dialog"},
        "Mod_HakList": {"value": [{"Mod_Hak": {"value": "forge_assets"}}]},
    }


if __name__ == "__main__":
    unittest.main()
