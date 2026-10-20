import plistlib
import tempfile
import unittest
from pathlib import Path

from macos_minimum_version import macho_minimum, update_minimum, version


def modern(minimum, sdk="27.0", platform="MACOS"):
    return f"cmd LC_BUILD_VERSION\n platform {platform}\n minos {minimum}\n sdk {sdk}\n"


class MinimumVersionTests(unittest.TestCase):
    def test_sdk_does_not_raise_deployment_target(self):
        self.assertEqual(macho_minimum(modern("13.0")), (13, 0, 0))

    def test_legacy_mac_command(self):
        self.assertEqual(macho_minimum("cmd LC_VERSION_MIN_MACOSX\n version 11.2.1\n sdk 14.0\n"), (11, 2, 1))

    def test_unknown_or_wrong_platform_fails_closed(self):
        for output in ("", "sdk 27.0", modern("17.0", platform="IOS"), modern("invalid")):
            with self.subTest(output=output), self.assertRaises(ValueError):
                macho_minimum(output)

    def test_versions_compare_numerically(self):
        self.assertGreater(version("13.10"), version("13.9.1"))

    def make_bundle(self, root, minimum="13.0"):
        contents = root / "Fixture.app/Contents"
        (contents / "MacOS").mkdir(parents=True)
        (contents / "Frameworks").mkdir()
        with (contents / "Info.plist").open("wb") as stream:
            plistlib.dump({"CFBundleExecutable": "fixture", "LSMinimumSystemVersion": minimum, "CFBundleVersion": "9"}, stream)
        (contents / "MacOS/fixture").touch()
        (contents / "Frameworks/libmpv.dylib").touch()
        return contents.parent

    def test_maximum_library_requirement_and_preserved_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = self.make_bundle(Path(directory))
            self.assertEqual(update_minimum(bundle, lambda p: modern("27.0" if p.suffix == ".dylib" else "13.0")), ("27.0", 2))
            with (bundle / "Contents/Info.plist").open("rb") as stream:
                info = plistlib.load(stream)
            self.assertEqual(info["LSMinimumSystemVersion"], "27.0")
            self.assertEqual(info["CFBundleVersion"], "9")

    def test_configured_floor_never_lowered(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = self.make_bundle(Path(directory), "28.1.2")
            self.assertEqual(update_minimum(bundle, lambda _: modern("13.0")), ("28.1.2", 2))

    def test_failed_inspection_does_not_rewrite_plist(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = self.make_bundle(Path(directory))
            info = bundle / "Contents/Info.plist"
            before = info.read_bytes()
            with self.assertRaises(ValueError):
                update_minimum(bundle, lambda _: "unrecognized")
            self.assertEqual(info.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
