import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class DesktopSecurityTests(unittest.TestCase):
    def test_packaged_webview_allows_in_memory_image_sources(self) -> None:
        config = json.loads(
            (ROOT / "apps" / "desktop" / "src-tauri" / "tauri.conf.json").read_text(
                encoding="utf-8"
            )
        )
        csp = config["app"]["security"]["csp"]
        image_sources = next(
            directive for directive in csp.split(";") if directive.strip().startswith("img-src ")
        ).split()

        self.assertIn("data:", image_sources)
        self.assertIn("blob:", image_sources)


if __name__ == "__main__":
    unittest.main()
