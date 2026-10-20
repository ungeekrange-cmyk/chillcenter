#!/usr/bin/env python3
"""Focused local fixtures for the legacy Pi v0.7 operator.

The production script deliberately fixes PATH.  Each fixture copies it and changes
only that one line so mocked system commands are isolated in a temporary directory;
the shipped operator has no test-mode switch or environment bypass.
"""
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import unittest
import zipfile


ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "scripts/pi-v07-legacy-server-web-operator.sh"
OLD_BUILD = "67773f7877d3"
NEW_BUILD = "a1c12ba4a582"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class OperatorRecoveryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="pi-v07-operator-test."))
        self.bin = self.tmp / "bin"; self.bin.mkdir()
        self.rollback = self.tmp / "rollback"; self.rollback.mkdir(mode=0o700)
        self.web = self.tmp / "current-web"; self.web.mkdir()
        self.binary = self.tmp / "chill-server"
        self.cert = self.tmp / "cert.pem"; self.cert.write_text("fixture")
        self._write_web(self.web, OLD_BUILD)
        self.binary.write_text("old-server")
        for name, body in {
            "systemctl": "#!/bin/sh\ncase \"$1\" in is-active) echo active;; esac\nexit 0\n",
            "curl": "#!/bin/sh\nprintf 200\n",
            "runuser": "#!/bin/sh\nexit 0\n", "ss": "#!/bin/sh\nexit 0\n",
            "timeout": "#!/bin/sh\nexec \"$@\"\n", "setsid": "#!/bin/sh\nexec \"$@\"\n",
            "mv": "#!/bin/sh\n[ \"$1\" = -T ] && shift\nexec /bin/mv \"$@\"\n",
        }.items():
            p = self.bin / name; p.write_text(body); p.chmod(0o755)
        self.script = self.tmp / "operator.sh"
        text = SOURCE.read_text().replace(
            "export PATH=/usr/sbin:/usr/bin:/sbin:/bin",
            'export PATH="$CC_FIXTURE_BIN:/usr/sbin:/usr/bin:/sbin:/bin"', 1)
        text = text.replace('[[ "$(id -u)" == 0 && "$CC_BASE_URL" =~ ^https://[^/]+$ ]]', '[[ "$CC_FIXTURE_ROOT" == yes && "$CC_BASE_URL" =~ ^https://[^/]+$ ]]', 1)
        self.script.write_text(text); self.script.chmod(0o755)

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def _write_web(self, root, build):
        root.mkdir(exist_ok=True)
        (root / "build.json").write_text(json.dumps({"build": build}))

    def _journal(self, old_bin=None, old_web=None):
        data = {
            "state": "swapping", "old_binary": str(old_bin or self.rollback / "old-server"),
            "new_binary": str(self.rollback / ".new-server"), "old_web": str(old_web or self.rollback / "old-web"),
            "new_web": str(self.rollback / ".new-web"), "old_hash": digest(self.rollback / "old-server") if (self.rollback / "old-server").exists() else digest(self.binary),
            "new_hash": hashlib.sha256(b"new-server").hexdigest(), "old_web_build": OLD_BUILD,
            "new_web_build": NEW_BUILD,
        }
        (self.rollback / "v07-transition.json").write_text(json.dumps(data))

    def _recover(self):
        env = os.environ | {"CC_FIXTURE_BIN": str(self.bin), "CC_FIXTURE_ROOT": "yes", "CC_SERVICE": "fixture", "CC_BINARY": str(self.binary),
                            "CC_WEB_DIR": str(self.web), "CC_ROLLBACK_DIR": str(self.rollback), "CC_CERT": str(self.cert),
                            "CC_BASE_URL": "https://fixture", "CC_EXPECTED_OLD_BINARY_SHA256": hashlib.sha256(b"old-server").hexdigest()}
        return subprocess.run(["bash", str(self.script), "--recover"], env=env, text=True, capture_output=True)

    def test_recovery_restores_pair_after_web_boundary(self):
        old_web = self.rollback / "old-web"; self.web.rename(old_web); self._write_web(self.web, NEW_BUILD)
        self._journal(old_web=old_web)
        result = self._recover()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(digest(self.binary), hashlib.sha256(b"old-server").hexdigest())
        self.assertEqual(json.loads((self.web / "build.json").read_text())["build"], OLD_BUILD)
        self.assertIn("recovery=PASS", result.stdout)

    def test_recovery_restores_pair_after_kill_between_replacements(self):
        old_web = self.rollback / "old-web"; self.web.rename(old_web); self._write_web(self.web, NEW_BUILD)
        old_bin = self.rollback / "old-server"; old_bin.write_text("old-server"); self.binary.write_text("new-server")
        self._journal(old_bin=old_bin, old_web=old_web)
        result = self._recover()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_text(), "old-server")
        self.assertEqual(json.loads((self.web / "build.json").read_text())["build"], OLD_BUILD)

    def test_recovery_after_kill_or_backup_timeout_before_any_rename(self):
        # This is the durable receipt written before the service stop/backup.
        self._journal()
        result = self._recover()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_text(), "old-server")
        self.assertTrue((self.rollback / "v07-transition.json").exists())

    def test_closed_modes_and_unsafe_zip_regressions(self):
        self.assertNotEqual(subprocess.run(["bash", str(SOURCE)], text=True, capture_output=True).returncode, 0)
        self.assertNotEqual(subprocess.run(["bash", str(SOURCE), "--bad"], text=True, capture_output=True).returncode, 0)
        source = SOURCE.read_text()
        check = re.search(r"safe_zip\(\)\{ python3 -I - \"\$1\" <<'PY'\n(.*?)\nPY", source, re.S).group(1)
        for member, mode in (("../escape", None), ("/absolute", None), ("linked", 0o120777), ("device", 0o20600)):
            with tempfile.NamedTemporaryFile(suffix=".zip") as f:
                with zipfile.ZipFile(f.name, "w") as z:
                    if mode:
                        entry = zipfile.ZipInfo(member); entry.external_attr = mode << 16; z.writestr(entry, "target")
                    else: z.writestr(member, "x")
                    z.writestr("safe-last", "x")  # regression: unsafe member is not last
                result = subprocess.run(["python3", "-I", "-", f.name], input=check, text=True, capture_output=True)
                self.assertNotEqual(result.returncode, 0, member)


if __name__ == "__main__":
    unittest.main(verbosity=2)
