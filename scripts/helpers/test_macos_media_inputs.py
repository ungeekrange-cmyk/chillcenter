import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from macos_bundle import resolve_media_dependency, is_system
from macos_media_inputs import read_lock, extract_selected, inventory, sha256, verify


class MediaInputTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.row = {"name": "mpv", "version": "0.41.0_11", "sha256": "a" * 64,
                    "url": "https://ghcr.io/v2/homebrew/core/mpv/blobs/sha256:" + "a" * 64,
                    "bottle_tag": "arm64_sequoia"}
        self.lock = {"schema_version": 1, "minimum_macos": "15.0", "architecture": "arm64", "packages": [self.row]}

    def write_lock(self, lock):
        path = self.root / "lock.json"
        path.write_text(json.dumps(lock))
        return path

    def archive(self, members):
        path = self.root / "bottle.tar.gz"
        with tarfile.open(path, "w:gz") as stream:
            for name, content, kind in members:
                member = tarfile.TarInfo(name)
                if kind == "file":
                    member.size = len(content)
                    stream.addfile(member, io.BytesIO(content))
                else:
                    member.type = {"link": tarfile.SYMTYPE, "hard": tarfile.LNKTYPE, "fifo": tarfile.FIFOTYPE}[kind]
                    member.linkname = content
                    stream.addfile(member)
        return path

    def test_lock_restricts_origin_platform_identity_and_digest(self):
        self.assertEqual(read_lock(self.write_lock(self.lock)), self.lock)
        for field, value in [("name", "../mpv"), ("version", "../bad"), ("sha256", "a" * 63),
                             ("url", "https://example.org/archive"), ("bottle_tag", "arm64_golden_gate")]:
            changed = copy.deepcopy(self.lock)
            changed["packages"][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                read_lock(self.write_lock(changed))
        duplicate = copy.deepcopy(self.lock)
        duplicate["packages"].append(self.row)
        with self.assertRaises(ValueError):
            read_lock(self.write_lock(duplicate))

    def test_only_libraries_and_licenses_are_extracted_without_executing_recipe(self):
        prefix = "mpv/0.41.0_11/"
        archive = self.archive([(prefix + "lib/libmpv.2.dylib", b"binary", "file"),
                                (prefix + "lib/libmpv.dylib", "libmpv.2.dylib", "link"),
                                (prefix + "LICENSE.GPL", b"license", "file"),
                                (prefix + ".brew/mpv.rb", b"raise 'never execute'", "file"),
                                (prefix + "bin/mpv", b"not extracted", "file")])
        extract_selected(archive, self.root, self.row)
        self.assertEqual((self.root / "Cellar" / prefix / "lib/libmpv.dylib").read_bytes(), b"binary")
        self.assertFalse((self.root / "Cellar" / prefix / "bin/mpv").exists())
        self.assertTrue((self.root / "Cellar" / prefix / "LICENSE.GPL").is_file())

    def test_escaping_paths_links_wrong_versions_and_special_files_fail_closed(self):
        for name, value, kind in [("../escape", b"x", "file"), ("/escape", b"x", "file"),
                                 ("mpv/0.40/lib/x.dylib", b"x", "file"),
                                 ("mpv/0.41.0_11/lib/x.dylib", "../../../../escape", "link"),
                                 ("mpv/0.41.0_11/lib/x.dylib", "/etc/passwd", "link"),
                                 ("mpv/0.41.0_11/lib/x.dylib", "", "fifo"),
                                 ("mpv/0.41.0_11/lib/x.dylib", "other", "hard")]:
            with self.subTest(name=name, kind=kind), self.assertRaises(ValueError):
                extract_selected(self.archive([(name, value, kind)]), self.root, self.row)

    def test_duplicate_dylibs_do_not_overwrite_extracted_bytes(self):
        name = "mpv/0.41.0_11/lib/x.dylib"
        with self.assertRaises(ValueError):
            extract_selected(self.archive([(name, b"first", "file"), (name, b"second", "file")]), self.root, self.row)
        self.assertEqual((self.root / "Cellar" / name).read_bytes(), b"first")

    def test_library_alias_cannot_resolve_to_a_notice_inside_the_formula(self):
        prefix = "mpv/0.41.0_11/"
        archive = self.archive([(prefix + "LICENSE.GPL", b"not a library", "file"),
                                (prefix + "lib/libmpv.dylib", "../LICENSE.GPL", "link")])
        with self.assertRaises(ValueError):
            extract_selected(archive, self.root, self.row)

    def test_dependency_resolution_never_falls_back_to_installed_homebrew(self):
        lib = self.root / "Cellar/mpv/0.41.0_11/lib/libmpv.dylib"
        lib.parent.mkdir(parents=True)
        lib.write_bytes(b"pinned")
        (self.root / "opt").mkdir()
        (self.root / "opt/mpv").symlink_to("../Cellar/mpv/0.41.0_11")
        for reference in ["/opt/homebrew/opt/mpv/lib/libmpv.dylib", "@@HOMEBREW_PREFIX@@/opt/mpv/lib/libmpv.dylib",
                          "@@HOMEBREW_CELLAR@@/mpv/0.41.0_11/lib/libmpv.dylib"]:
            self.assertEqual(resolve_media_dependency(reference, self.root), lib.resolve())
        with self.assertRaises(FileNotFoundError):
            resolve_media_dependency("/opt/homebrew/opt/ffmpeg/lib/libavcodec.dylib", self.root)
        with self.assertRaises(RuntimeError):
            resolve_media_dependency("/opt/homebrew/../../etc/passwd", self.root)
        self.assertFalse(is_system("/Applications/Xcode.app/lib/private.dylib"))

    def test_modified_cached_input_or_formula_alias_is_refused(self):
        lib = self.root / "Cellar/mpv/0.41.0_11/lib/libmpv.dylib"
        lib.parent.mkdir(parents=True)
        lib.write_bytes(b"pinned")
        (self.root / "opt").mkdir()
        alias = self.root / "opt/mpv"
        alias.symlink_to("../Cellar/mpv/0.41.0_11")
        lock_path = self.write_lock(self.lock)
        (self.root / "media-lock.json").write_text(json.dumps(self.lock))
        (self.root / "receipt.json").write_text(json.dumps({"lock_sha256": sha256(lock_path), "files": inventory(self.root)}))
        self.assertEqual(verify(self.root, lock_path), self.root / "opt/mpv/lib/libmpv.dylib")
        lib.write_bytes(b"changed")
        with self.assertRaises(ValueError):
            verify(self.root, lock_path)
        lib.write_bytes(b"pinned")
        alias.unlink()
        alias.symlink_to("/opt/homebrew/opt/mpv")
        with self.assertRaises(ValueError):
            verify(self.root, lock_path)


if __name__ == "__main__":
    unittest.main()
