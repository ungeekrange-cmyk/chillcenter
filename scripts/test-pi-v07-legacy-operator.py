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
import sqlite3
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
        self.state = self.tmp / "state"; self.state.write_text("active")
        self.generation = self.tmp / "generation"; self.generation.write_text("0")
        self._write_web(self.web, OLD_BUILD)
        self.binary.write_text("old-server")
        for name, body in {
            "systemctl": "#!/bin/sh\nprintf '%s\\n' \"$*\" >>\"$CC_FIXTURE_LOG\"\ncase \"$1\" in is-active) [ \"$(cat \"$CC_FIXTURE_STATE\")\" = active ] && { echo active; exit 0; } || exit 3;; stop) echo stopped >\"$CC_FIXTURE_STATE\";; start) echo active >\"$CC_FIXTURE_STATE\"; n=$(cat \"$CC_FIXTURE_GENERATION\" 2>/dev/null || echo 0); echo $((n+1)) >\"$CC_FIXTURE_GENERATION\";; show) printf 'User=fixture\\nGroup=fixture\\nFragmentPath=/fixture.service\\n';; cat) printf '[Service]\\nExecStart=/fixture/chill-server\\nUser=fixture\\n';; esac\n",
            "curl": "#!/bin/sh\nurl=; for x in \"$@\"; do url=$x; done; case \"$url\" in */admin/scan) n=0; [ -z \"$CC_FIXTURE_SCAN_COUNT\" ] || { n=$(cat \"$CC_FIXTURE_SCAN_COUNT\" 2>/dev/null || echo 0); n=$((n+1)); echo $n >\"$CC_FIXTURE_SCAN_COUNT\"; }; [ \"$CC_FIXTURE_BUSY_SECOND\" = yes ] && [ \"$n\" -ge 2 ] && printf '{\\\"running\\\":true}\\n' || printf '{\\\"running\\\":false}\\n';; */build.json) cat \"$CC_FIXTURE_WEB/build.json\";; *) printf '200\\n';; esac\n",
            "runuser": "#!/bin/sh\nexit 0\n", "ss": "#!/bin/sh\nexit 0\n",
            "timeout": "#!/bin/sh\nwhile [ $# -gt 0 ]; do case \"$1\" in --*) shift;; *s) shift;; *) break;; esac; done; exec \"$@\"\n", "setsid": "#!/bin/sh\nexec \"$@\"\n",
            "mv": "#!/bin/sh\n[ \"$1\" = -T ] && shift\nexec /bin/mv \"$@\"\n",
            "runuser": "#!/bin/sh\n[ \"$CC_FIXTURE_BACKUP_FAIL\" = yes ] && exit 70\nprintf 'Sauvegarde terminée : %s/backups/snapshot\\n' \"$CC_DATA_DIR\"\n",
            "chown": "#!/bin/sh\nexit 0\n", "install": "#!/bin/sh\nprev=; for x; do src=$prev; dst=$x; prev=$x; done; cp \"$src\" \"$dst\"; chmod 755 \"$dst\"\n",
            "stat": "#!/bin/sh\ncase \"$2\" in '%a:%u:%g') echo 700:0:0;; '%d') echo 1;; *) echo fixture-stat;; esac\n",
        }.items():
            p = self.bin / name; p.write_text(body); p.chmod(0o755)
        self.script = self.tmp / "operator.sh"
        text = SOURCE.read_text().replace(
            "export PATH=/usr/sbin:/usr/bin:/sbin:/bin",
            'export PATH="$CC_FIXTURE_BIN:/usr/sbin:/usr/bin:/sbin:/bin"', 1)
        text = text.replace('[[ "$(id -u)" == 0 && "$CC_BASE_URL" =~ ^https://[^/]+$ ]]', '[[ "$CC_FIXTURE_ROOT" == yes && "$CC_BASE_URL" =~ ^https://[^/]+$ ]]', 1)
        text = text.replace("/usr/lib/chillcenter", str(self.tmp))
        self.script.write_text(text); self.script.chmod(0o755)

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def _write_web(self, root, build):
        root.mkdir(exist_ok=True)
        (root / "build.json").write_text(json.dumps({"build": build}))

    def _journal(self, old_bin=None, old_web=None, state="swapping"):
        data = {
            "state": state, "old_binary": str(old_bin or self.rollback / "old-server"),
            "new_binary": str(self.rollback / ".new-server"), "old_web": str(old_web or self.rollback / "old-web"),
            "new_web": str(self.rollback / ".new-web"), "old_hash": digest(self.rollback / "old-server") if (self.rollback / "old-server").exists() else digest(self.binary),
            "new_hash": hashlib.sha256(b"new-server").hexdigest(), "old_web_build": OLD_BUILD,
            "new_web_build": NEW_BUILD,
        }
        (self.rollback / "v07-transition.json").write_text(json.dumps(data))

    def _recover(self):
        self.log = self.tmp / "systemctl.log"
        env = os.environ | {"CC_FIXTURE_BIN": str(self.bin), "CC_FIXTURE_ROOT": "yes", "CC_SERVICE": "fixture", "CC_BINARY": str(self.binary),
                            "CC_WEB_DIR": str(self.web), "CC_ROLLBACK_DIR": str(self.rollback), "CC_CERT": str(self.cert),
                            "CC_BASE_URL": "https://fixture", "CC_FIXTURE_LOG": str(self.log), "CC_FIXTURE_STATE": str(self.state), "CC_FIXTURE_GENERATION":str(self.generation), "CC_EXPECTED_OLD_BINARY_SHA256": hashlib.sha256(b"old-server").hexdigest()}
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
        self.assertFalse((self.rollback / "v07-transition.json").exists())

    def test_recovery_after_copy_before_binary_rename_keeps_old_current(self):
        old_bin = self.rollback / "old-server"; old_bin.write_text("old-server")
        self._journal(old_bin=old_bin)
        result = self._recover()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_text(), "old-server")
        self.assertTrue(old_bin.exists())

    def test_prepared_receipt_recovery_does_not_stop_active_service(self):
        self._journal(state="prepared")
        result = self._recover()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("stop fixture", self.log.read_text())

    def test_full_deploy_preserves_persistent_state_while_replacing_payload_pair(self):
        """A normal deployment reaches PASS: payloads change, settings do not."""
        data = self.tmp / "data"; data.mkdir()
        config = self.tmp / "server-config.json"; config.write_text("config")
        library = data / "library.json"; library.write_text("library")
        token = data / "tmdb-token"; token.write_text("secret")
        db = data / "catalog.sqlite"; sqlite3.connect(db).execute("pragma user_version=3").connection.close()
        backup = data / "backups" / "snapshot"; (backup / "secrets" / "tls").mkdir(parents=True)
        backup.chmod(0o700)
        shutil.copy2(db, backup / "catalog.sqlite"); shutil.copy2(config, backup / "server-config.json")
        shutil.copy2(self.cert, backup / "secrets/tls/certificate.pem"); shutil.copy2(self.cert, backup / "secrets/tls/service-key.pem")
        entries = []
        for p in (backup / "catalog.sqlite", backup / "server-config.json", backup / "secrets/tls/certificate.pem", backup / "secrets/tls/service-key.pem"):
            entries.append({"path": str(p.relative_to(backup)), "size": p.stat().st_size, "sha256": digest(p)})
        (backup / "manifest.json").write_text(json.dumps({"format_version": 1, "schema_version": 3, "data_root": str(data), "entries": entries}))
        new_server = self.tmp / "new-server"; new_server.write_text("new-server")
        web_zip = self.tmp / "web.zip"
        with zipfile.ZipFile(web_zip, "w") as z: z.writestr("build.json", json.dumps({"build": NEW_BUILD}))
        server_manifest = self.tmp / "server.json"; web_manifest = self.tmp / "web.json"; sig1 = self.tmp / "server.sig"; sig2 = self.tmp / "web.sig"; pub = self.tmp / "pub"
        sig1.write_text("sig1"); sig2.write_text("sig2"); pub.write_text("pub")
        server_manifest.write_text(json.dumps({"schema_version":1,"component":"server","version":"0.7.0","key_id":"key","compatibility":{"api":{"min":"2026.10.14","max":"2026.10.14"},"database":{k:3 for k in ("current_schema","target_schema","read_min","read_max","write_min","write_max","rollback_min","rollback_max")}},"assets":[{"target":"linux-aarch64","sha256":digest(new_server)}]}))
        web_manifest.write_text(json.dumps({"schema_version":1,"component":"web","version":"0.7.0","key_id":"key","compatibility":{"api":{"min":"2026.10.14","max":"2026.10.14"},"required_server":{"min":"0.7.0"}},"assets":[{"target":"web","sha256":digest(web_zip)}]}))
        log = self.tmp / "deploy.log"
        env = os.environ | {"CC_FIXTURE_BIN":str(self.bin), "CC_FIXTURE_ROOT":"yes", "CC_FIXTURE_STATE":str(self.state), "CC_FIXTURE_GENERATION":str(self.generation), "CC_FIXTURE_LOG":str(log), "CC_FIXTURE_WEB":str(self.web), "CC_SERVICE":"fixture", "CC_SERVICE_USER":"fixture", "CC_BINARY":str(self.binary), "CC_WEB_DIR":str(self.web), "CC_CONFIG":str(config), "CC_DATA_DIR":str(data), "CC_ROLLBACK_DIR":str(self.rollback), "CC_CERT":str(self.cert), "CC_BASE_URL":"https://fixture", "CC_SERVER_FILE":str(new_server), "CC_WEB_ZIP":str(web_zip), "CC_SERVER_MANIFEST":str(server_manifest), "CC_SERVER_MANIFEST_SIG":str(sig1), "CC_WEB_MANIFEST":str(web_manifest), "CC_WEB_MANIFEST_SIG":str(sig2), "CC_RELEASE_PUB":str(pub), "CC_EXPECTED_RELEASE_PUB_SHA256":digest(pub), "CC_EXPECTED_SERVER_MANIFEST_SHA256":digest(server_manifest), "CC_EXPECTED_SERVER_MANIFEST_SIG_SHA256":digest(sig1), "CC_EXPECTED_WEB_MANIFEST_SHA256":digest(web_manifest), "CC_EXPECTED_WEB_MANIFEST_SIG_SHA256":digest(sig2), "CC_EXPECTED_KEY_ID":"key", "CC_EXPECTED_OLD_BINARY_SHA256":hashlib.sha256(b"old-server").hexdigest(), "CC_EXPECTED_NEW_BINARY_SHA256":digest(new_server), "CC_EXPECTED_WEB_SHA256":digest(web_zip), "CC_EXPECTED_VERSION":"0.7.0", "CC_EXPECTED_WEB_BUILD":NEW_BUILD}
        result = subprocess.run(["bash", str(self.script), "--deploy"], env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("deployment=PASS", result.stdout)
        self.assertEqual(self.generation.read_text().strip(), "1")  # restart changed runtime PID/start metadata
        self.assertEqual(self.binary.read_text(), "new-server")
        self.assertEqual(json.loads((self.web / "build.json").read_text())["build"], NEW_BUILD)
        self.assertEqual((config.read_text(), library.read_text(), token.read_text()), ("config", "library", "secret"))
        # A final busy result occurs after staging but before the stop intent; it
        # must leave the active service alone even though a prepared receipt exists.
        log.unlink(); scan_count = self.tmp / "scan-count"
        env |= {"CC_EXPECTED_OLD_BINARY_SHA256":digest(self.binary), "CC_FIXTURE_BUSY_SECOND":"yes", "CC_FIXTURE_SCAN_COUNT":str(scan_count)}
        blocked = subprocess.run(["bash", str(self.script), "--deploy"], env=env, text=True, capture_output=True)
        self.assertEqual(blocked.returncode, 42, blocked.stderr)
        self.assertNotIn("stop fixture", log.read_text())
        recovered = subprocess.run(["bash", str(self.script), "--recover"], env=env, text=True, capture_output=True)
        self.assertEqual(recovered.returncode, 0, recovered.stderr)
        log.unlink()
        env |= {"CC_FIXTURE_BUSY_SECOND":"", "CC_FIXTURE_SCAN_COUNT":str(self.tmp / "failure-scan-count"), "CC_FIXTURE_BACKUP_FAIL":"yes"}
        failed_backup = subprocess.run(["bash", str(self.script), "--deploy"], env=env, text=True, capture_output=True)
        self.assertNotEqual(failed_backup.returncode, 0)
        self.assertEqual(self.state.read_text().strip(), "active")
        self.assertFalse((self.rollback / "v07-transition.json").exists())
        self.assertIn("stop fixture", log.read_text())

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
