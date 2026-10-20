import importlib.util
from pathlib import Path
import plistlib
import tempfile
import unittest


spec = importlib.util.spec_from_file_location("macos_dmg", Path(__file__).parents[1] / "package-macos-dmg.py")
dmg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dmg)


class DmgTests(unittest.TestCase):
    def fixture(self, root):
        bundle = root / "Chill Center.app"
        (bundle / "Contents").mkdir(parents=True)
        with (bundle / "Contents/Info.plist").open("wb") as stream:
            plistlib.dump({"CFBundleIdentifier": "home.chillcenter.desktop", "CFBundleShortVersionString": "0.7.0", "LSMinimumSystemVersion": "27.0"}, stream)
        return bundle

    def test_valid_normal_bundle(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            info = dmg.validate_paths(self.fixture(root), root / "new.dmg")
            self.assertEqual(info["LSMinimumSystemVersion"], "27.0")

    def test_existing_file_or_dangling_link_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.fixture(root)
            target = root / "existing.dmg"
            target.write_bytes(b"user-owned")
            with self.assertRaises(FileExistsError):
                dmg.validate_paths(bundle, target)
            self.assertEqual(target.read_bytes(), b"user-owned")
            link = root / "dangling.dmg"
            link.symlink_to(root / "missing")
            with self.assertRaises(FileExistsError):
                dmg.validate_paths(bundle, link)
            self.assertTrue(link.is_symlink())

    def test_wrong_identity_and_recursive_output_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.fixture(root)
            with self.assertRaises(ValueError):
                dmg.validate_paths(bundle, bundle / "recursive.dmg")
            with (bundle / "Contents/Info.plist").open("wb") as stream:
                plistlib.dump({"CFBundleIdentifier": "home.chillcenter.qa"}, stream)
            with self.assertRaises(ValueError):
                dmg.validate_paths(bundle, root / "qa.dmg")

    def test_atomic_publication_refuses_racing_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage, target = root / "stage.dmg", root / "target.dmg"
            stage.write_bytes(b"new")
            target.write_bytes(b"keep")
            with self.assertRaises(FileExistsError):
                dmg.publish_image(stage, target)
            self.assertEqual(target.read_bytes(), b"keep")
            self.assertEqual(stage.read_bytes(), b"new")

    def test_successful_publication_does_not_depend_on_staging_link(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage, target = root / "stage.dmg", root / "target.dmg"
            stage.write_bytes(b"new")
            dmg.publish_image(stage, target)
            stage.unlink()
            self.assertEqual(target.read_bytes(), b"new")


if __name__ == "__main__":
    unittest.main()
