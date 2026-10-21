#!/usr/bin/env python3
"""Fixture-only tests for the optional OS package maintenance job.

The production script has fixed absolute command paths and a root guard.  Each
test patches those only in a temporary copy; it never runs apt or reboot.
"""
import os
import shutil
import signal
import subprocess
import tempfile
import textwrap
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "deploy/linux/maintenance/chillcenter-os-package-maintenance.sh"
SERVICE = ROOT / "deploy/linux/maintenance/chillcenter-os-package-maintenance.service"
CRON = ROOT / "deploy/linux/maintenance/chillcenter-os-package-maintenance.cron"


class MaintenanceTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="cc-os-maintenance-"))
        self.log = self.tmp / "calls"
        self.envlog = self.tmp / "environment"
        self.scenario = self.tmp / "scenario"
        self.scenario.write_text("success")
        self.bin = self.tmp / "bin"; self.bin.mkdir()
        self._fake("apt-get", '''
            printf '%s|%s|%s\n' "$DEBIAN_FRONTEND" "$LC_ALL" "$(umask)" >> "$CC_ENV_LOG"
            echo "apt-get $*" >> "$CC_LOG"
            s=$(cat "$CC_SCENARIO")
            case "$*" in *update*) [ "$s" = update_fail ] && exit 9;; *upgrade*)
              [ "$s" = upgrade_fail ] && exit 9; [ "$s" = term ] && sleep 10;; *check*) [ "$s" = check_fail ] && exit 9;; esac
            exit 0
        ''')
        self._fake("dpkg", '''
            printf '%s|%s|%s\n' "$DEBIAN_FRONTEND" "$LC_ALL" "$(umask)" >> "$CC_ENV_LOG"
            echo "dpkg $*" >> "$CC_LOG"
            s=$(cat "$CC_SCENARIO"); n=$(grep -c '^dpkg ' "$CC_LOG")
            [ "$s" = readiness_stall ] && sleep 10
            [ "$s" = dpkg_fail ] && exit 9
            [ "$s" = dpkg_after_fail ] && [ "$n" -eq 2 ] && exit 9
            [ "$s" = audit_before ] && [ "$n" -eq 1 ] && { echo broken; exit 0; }
            [ "$s" = audit_after ] && [ "$n" -eq 2 ] && { echo broken; exit 0; }
            exit 0
        ''')
        self._fake("reboot", 'printf \'%s|%s|%s\\n\' "$DEBIAN_FRONTEND" "$LC_ALL" "$(umask)" >> "$CC_ENV_LOG"\necho reboot >> "$CC_LOG"')
        target = self.tmp / "job"
        body = SOURCE.read_text()
        body = body.replace("[[ ${EUID} -eq 0 ]]", "[[ ${CC_TEST_ROOT:-1} -eq 0 ]]")
        body = body.replace("/usr/bin/apt-get", str(self.bin / "apt-get"))
        body = body.replace("/usr/bin/dpkg", str(self.bin / "dpkg"))
        body = body.replace("/usr/sbin/reboot", str(self.bin / "reboot"))
        target.write_text(body); target.chmod(0o700)
        self.job = target

    def tearDown(self): shutil.rmtree(self.tmp)

    def _fake(self, name, body):
        p = self.bin / name
        p.write_text("#!/bin/sh\nset -eu\n" + textwrap.dedent(body))
        p.chmod(0o700)

    def execute(self, scenario="success", root=True):
        self.scenario.write_text(scenario)
        env = os.environ | {"CC_LOG": str(self.log), "CC_ENV_LOG": str(self.envlog), "CC_SCENARIO": str(self.scenario), "CC_TEST_ROOT": "0" if root else "1"}
        return subprocess.run([str(self.job)], env=env, text=True, capture_output=True)

    def calls(self): return self.log.read_text() if self.log.exists() else ""
    def call_lines(self): return self.calls().splitlines()
    def expected_prefix(self, stage):
        calls = [
            "dpkg --audit",
            "apt-get -o APT::Update::Error-Mode=any update",
            "apt-get -o DPkg::Lock::Timeout=300 -o Dpkg::Options::=--force-confold --with-new-pkgs --no-remove -y upgrade",
            "apt-get check",
            "dpkg --audit",
        ]
        return calls[:stage]
    def assert_environment_boundary(self, count):
        self.assertEqual(self.envlog.read_text().splitlines(), ["noninteractive|C|0077"] * count)
    def test_success_runs_exact_safe_commands_then_reboots(self):
        r = self.execute(); self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.call_lines(), self.expected_prefix(5) + ["reboot"])
        self.assert_environment_boundary(6)
    def test_root_guard_stops_before_commands(self):
        self.assertNotEqual(self.execute(root=False).returncode, 0); self.assertEqual(self.calls(), "")
    def test_executable_guard_stops_before_commands(self):
        (self.bin / "apt-get").chmod(0o600)
        result = self.execute()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("required fixed-path command missing", result.stderr)
        self.assertEqual(self.calls(), "")
    def test_failures_stop_at_faulting_stage(self):
        expected = {"dpkg_fail": 1, "audit_before": 1, "update_fail": 2,
                    "upgrade_fail": 3, "check_fail": 4, "audit_after": 5,
                    "dpkg_after_fail": 5}
        diagnostics = {"dpkg_fail": "dpkg --audit failed (before)", "audit_before": "dpkg audit is non-empty (before)",
                       "update_fail": "apt-get update failed", "upgrade_fail": "apt-get upgrade failed",
                       "check_fail": "apt-get check failed", "audit_after": "dpkg audit is non-empty (after)",
                       "dpkg_after_fail": "dpkg --audit failed (after)"}
        for scenario, count in expected.items():
            with self.subTest(scenario=scenario):
                self.log.unlink(missing_ok=True); self.envlog.unlink(missing_ok=True); r = self.execute(scenario)
                self.assertEqual(r.returncode, 1)
                self.assertIn(diagnostics[scenario], r.stderr)
                self.assertEqual(self.call_lines(), self.expected_prefix(count))
                self.assert_environment_boundary(count)
    def _term_fixture(self, scenario="term", readiness_seconds=10):
        self.scenario.write_text(scenario)
        env = os.environ | {"CC_LOG": str(self.log), "CC_ENV_LOG": str(self.envlog), "CC_SCENARIO": str(self.scenario), "CC_TEST_ROOT": "0"}
        p = subprocess.Popen([str(self.job)], env=env, text=True, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, start_new_session=True)
        self.term_process = p
        stderr = ""
        try:
            deadline = time.monotonic() + readiness_seconds
            ready = False
            while time.monotonic() < deadline:
                if "upgrade" in self.calls():
                    ready = True
                    break
                if p.poll() is not None:
                    break
                time.sleep(.02)
            self.assertTrue(ready, "TERM fixture did not reach upgrade before readiness deadline")
            os.killpg(p.pid, signal.SIGTERM)
            _, stderr = p.communicate(timeout=3)
            return p.returncode, stderr
        finally:
            if p.poll() is None:
                try:
                    os.killpg(p.pid, signal.SIGTERM)
                    p.communicate(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(p.pid, signal.SIGKILL)
                    p.communicate(timeout=3)
            else:
                p.communicate()
            if p.stdout is not None:
                p.stdout.close()
            if p.stderr is not None:
                p.stderr.close()
    def test_term_suppresses_reboot(self):
        returncode, stderr = self._term_fixture()
        self.assertEqual(returncode, 143)
        self.assertIn("interrupted; reboot suppressed", stderr)
        self.assertEqual(self.call_lines(), self.expected_prefix(3))
        self.assert_environment_boundary(3)
    def test_stalled_term_readiness_is_reaped(self):
        with self.assertRaisesRegex(AssertionError, "readiness deadline"):
            self._term_fixture("readiness_stall", .1)
        self.assertIsNotNone(self.term_process.poll())
        self.assertTrue(self.term_process.stdout.closed)
        self.assertTrue(self.term_process.stderr.closed)
    def test_no_unsafe_apt_flags(self):
        source = SOURCE.read_text()
        for forbidden in ("autoremove", "dist-upgrade", "--allow-unauthenticated", "--allow-downgrades", "--allow-remove-essential", "--force-yes"):
            self.assertNotIn(forbidden, source)
    def test_service_and_cron_contract(self):
        self.assertIn("Type=oneshot", SERVICE.read_text())
        self.assertIn("KillMode=control-group", SERVICE.read_text())
        self.assertIn("TimeoutStartSec=infinity", SERVICE.read_text())
        self.assertIn("UMask=0077", SERVICE.read_text())
        self.assertIn("StandardInput=null", SERVICE.read_text())
        self.assertNotIn("Documentation=", SERVICE.read_text())
        self.assertEqual(CRON.read_text().splitlines()[-1], "0 3 * * 0 root /usr/bin/systemctl --no-block start chillcenter-os-package-maintenance.service")
    def test_documented_cron_destination_has_no_dot(self):
        doc = (ROOT / "docs/operations/OS-PACKAGE-MAINTENANCE.md").read_text()
        self.assertIn("`/etc/cron.d/chillcenter-os-package-maintenance`", doc)

if __name__ == "__main__": unittest.main(verbosity=2)
