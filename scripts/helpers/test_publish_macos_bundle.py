import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from publish_macos_bundle import publish


@unittest.skipUnless(sys.platform == "darwin", "macOS-only atomic publication")
class PublicationTests(unittest.TestCase):
    def test_bsd_mv_reproduces_the_original_directory_race(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);stage=root/".Chill.app.staging.fixture";target=root/"Chill.app"
            stage.mkdir();self.assertFalse(target.exists());target.mkdir()
            result=subprocess.run(["/bin/mv",str(stage),str(target)],capture_output=True)
            self.assertEqual(result.returncode,0)
            self.assertTrue((target/stage.name).is_dir(),"old mv nests the staging directory instead of refusing")

    def test_racing_destination_is_not_modified(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);stage=root/".Chill.app.staging.fixture";target=root/"Chill.app"
            stage.mkdir();self.assertFalse(target.exists());target.mkdir();marker=target/"user-owned.txt";marker.write_bytes(b"keep")
            with self.assertRaises(FileExistsError):publish(stage,target)
            self.assertEqual(marker.read_bytes(),b"keep");self.assertEqual(list(target.iterdir()),[marker]);self.assertTrue(stage.is_dir())

    def test_new_target_is_published_and_symlink_destination_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);stage=root/".Chill.app.staging.fixture";target=root/"Chill.app"
            stage.mkdir();(stage/"signed.txt").write_bytes(b"signed");publish(stage,target)
            self.assertFalse(stage.exists());self.assertEqual((target/"signed.txt").read_bytes(),b"signed")
            stage.mkdir();link=root/"Existing.app";link.symlink_to(target,target_is_directory=True)
            with self.assertRaises(FileExistsError):publish(stage,link)
            self.assertTrue(link.is_symlink());self.assertTrue(stage.is_dir());self.assertEqual((target/"signed.txt").read_bytes(),b"signed")


if __name__ == "__main__":unittest.main()
