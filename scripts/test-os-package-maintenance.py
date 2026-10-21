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
        self.scenario = self.tmp / "scenario"
        self.scenario.write_text("success")
        self.bin = self.tmp / "bin"; self.bin.mkdir()
        self._fake("apt-get", '''
            echo "apt-get $*" >> "$CC_LOG"
            s=$(cat "$CC_SCENARIO")
            case "$*" in *update*) [ "$s" = update_fail ] && exit 9;; *upgrade*)
              [ "$s" = upgrade_fail ] && exit 9; [ "$s" = term ] && sleep 10;; *check*) [ "$s" = check_fail ] && exit 9;; esac
            exit 0
        ''')
        self._fake("dpkg", '''
            echo "dpkg $*" >> "$CC_LOG"
            s=$(cat "$CC_SCENARIO"); n=$(grep -c '^dpkg ' "$CC_LOG")
            [ "$s" = dpkg_fail ] && exit 9
            [ "$s" = audit_before ] && [ "$n" -eq 1 ] && { echo broken; exit 0; }
            [ "$s" = audit_after ] && [ "$n" -eq 2 ] && { echo broken; exit 0; }
            exit 0
        ''')
        self._fake("reboot", 'echo reboot >> "$CC_LOG"')
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
        env = os.environ | {"CC_LOG": str(self.log), "CC_SCENARIO": str(self.scenario), "CC_TEST_ROOT": "0" if root else "1"}
        return subprocess.run([str(self.job)], env=env, text=True, capture_output=True)

    def calls(self): return self.log.read_text() if self.log.exists() else ""
    def test_success_runs_exact_safe_commands_then_reboots(self):
        r = self.execute(); self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("DEBIAN_FRONTEND=noninteractive", SOURCE.read_text())
        self.assertIn("LC_ALL=C", SOURCE.read_text())
        self.assertIn("umask 077", SOURCE.read_text())
        self.assertIn("APT::Update::Error-Mode=any update", self.calls())
        self.assertIn("DPkg::Lock::Timeout=300", self.calls())
        self.assertIn("--force-confold --with-new-pkgs --no-remove -y upgrade", self.calls())
        self.assertEqual(self.calls().splitlines()[-1], "reboot")
    def test_root_guard_stops_before_commands(self):
        self.assertNotEqual(self.execute(root=False).returncode, 0); self.assertEqual(self.calls(), "")
    def test_failures_never_reboot(self):
        for scenario in ("dpkg_fail", "audit_before", "update_fail", "upgrade_fail", "check_fail", "audit_after"):
            with self.subTest(scenario=scenario):
                self.log.unlink(missing_ok=True); r = self.execute(scenario)
                self.assertNotEqual(r.returncode, 0); self.assertNotIn("reboot", self.calls())
    def test_term_suppresses_reboot(self):
        self.scenario.write_text("term")
        env = os.environ | {"CC_LOG": str(self.log), "CC_SCENARIO": str(self.scenario), "CC_TEST_ROOT": "0"}
        p = subprocess.Popen([str(self.job)], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        for _ in range(50):
            if "upgrade" in self.calls(): break
            time.sleep(.02)
        os.killpg(p.pid, signal.SIGTERM); p.communicate(timeout=3)
        self.assertNotEqual(p.returncode, 0)
        self.assertNotIn("reboot", self.calls())
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
