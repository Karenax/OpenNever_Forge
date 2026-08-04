from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.generate_lot1_fixture import write_fixture


class SyntheticLot1FixtureTests(unittest.TestCase):
    def test_checked_in_fixture_matches_the_generator(self) -> None:
        repository_fixture = (
            Path(__file__).parents[1] / "fixtures" / "synthetic" / "lot1_custom_tlk"
        )
        relative_outputs = (
            Path("module/forge_lot1.mod"),
            Path("user/hak/forge_assets.hak"),
            Path("user/tlk/forge_dialog.tlk"),
            Path("manifest.json"),
        )
        with tempfile.TemporaryDirectory() as directory:
            generated = Path(directory)
            write_fixture(generated)

            for relative in relative_outputs:
                self.assertEqual(
                    (repository_fixture / relative).read_bytes(),
                    (generated / relative).read_bytes(),
                    f"stale synthetic fixture: {relative.as_posix()}",
                )

    def test_generation_is_deterministic_and_declares_custom_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as first_directory, tempfile.TemporaryDirectory() as second_directory:
            first = write_fixture(Path(first_directory))
            second = write_fixture(Path(second_directory))

            self.assertEqual(first, second)
            self.assertEqual(first["expected"]["hakFiles"], ["forge_assets"])
            self.assertEqual(first["expected"]["customTlk"], "forge_dialog")
            self.assertEqual(first["expected"]["resolvedDependencies"], 2)
            self.assertEqual(
                (Path(first_directory) / "module" / "forge_lot1.mod").read_bytes()[0:8],
                b"MOD V1.0",
            )
            self.assertEqual(
                (Path(first_directory) / "user" / "tlk" / "forge_dialog.tlk").read_bytes()[0:8],
                b"TLK V3.0",
            )

    def test_generation_refuses_to_overwrite_without_explicit_force(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_fixture(root)

            with self.assertRaises(FileExistsError):
                write_fixture(root)


if __name__ == "__main__":
    unittest.main()
