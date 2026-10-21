#!/usr/bin/env python3
"""Real Linux process/lock interruption tests; all services/data are local fixtures.

Run as root in a disposable local VM. Never connects to any remote host, reads
the household configuration, or operates a real systemd service.
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import unittest
import uuid

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("operator_fixture", HERE / "test-pi-v07-legacy-operator.py")
fixture_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture_module)


@unittest.skipUnless(sys.platform == "linux" and os.geteuid() == 0, "isolated Linux root fixture required")
class DurableJobTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixture_module.OperatorRecoveryTests()
        self.fixture.setUp()
        self.fixture.script.chmod(0o700)
        self.env, self.log = self.fixture._deployment_env()
        self.root = self.fixture.rollback
        self.launcher = self.fixture.tmp / "job.py"
        shutil.copyfile(HERE / "pi-operator-job.py", self.launcher)
        self.launcher.chmod(0o700)
        self.marker = self.fixture.tmp / "backup-entered"
        self.release = self.fixture.tmp / "release"
        self.finished = self.fixture.tmp / "backup-finished"
        self.env.update(CC_FIXTURE_ENTERED=str(self.marker), CC_FIXTURE_RELEASE=str(self.release),
                        CC_FIXTURE_FINISHED=str(self.finished))
        # Real setsid/timeout and real advisory locks; only backup data and
        # systemctl/TLS/payload commands remain isolated fixtures.
        (self.fixture.bin / "setsid").write_text("#!/bin/sh\nexec /usr/bin/setsid \"$@\"\n")
        (self.fixture.bin / "timeout").write_text("#!/bin/sh\nexec /usr/bin/timeout \"$@\"\n")
        (self.fixture.bin / "runuser").write_text(
            "#!/usr/bin/python3\nimport os,pathlib,time\n"
            "pathlib.Path(os.environ['CC_FIXTURE_ENTERED']).write_text(str(os.getpid()))\n"
            "deadline=time.monotonic()+20\n"
            "while not pathlib.Path(os.environ['CC_FIXTURE_RELEASE']).exists():\n"
            " if time.monotonic()>deadline: raise SystemExit(71)\n"
            " time.sleep(.02)\n"
            "pathlib.Path(os.environ['CC_FIXTURE_FINISHED']).write_text('done')\n"
            "print('Sauvegarde terminée : '+os.environ['CC_DATA_DIR']+'/backups/snapshot')\n")
        self.jobs = []

    def tearDown(self):
        # Release only owned barriers; never broad pgrep/kill or a data-lock unlink.
        self.release.touch()
        if hasattr(self, "mv_release"):
            self.mv_release.touch()
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if not self._busy():
                break
            time.sleep(.03)
        self.assertFalse(self._busy(), "fixture child still owns authority; preserve evidence")
        self.fixture.tearDown()

    def _busy(self):
        import fcntl
        lock = self.root / "operator.lock"
        if not lock.exists():
            return False
        with lock.open("r+") as stream:
            try:
                fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
                return False
            except BlockingIOError:
                return True

    def launch(self, mode="deploy", job_id=None, **extra):
        job_id = job_id or str(uuid.uuid4())
        command = [sys.executable, "-I", str(self.launcher), "start", "--root", str(self.root),
                   "--job-id", job_id, "--mode", mode, "--operator", str(self.fixture.script),
                   "--operator-sha256", extra.get("sha", fixture_module.digest(self.fixture.script))]
        result = subprocess.run(command, env=self.env, capture_output=True, text=True, timeout=5)
        if result.returncode == 0:
            self.jobs.append(job_id)
        return job_id, result

    def status(self, job_id):
        result = subprocess.run([sys.executable, "-I", str(self.launcher), "status", "--root",
                                 str(self.root), "--job-id", job_id], capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def wait_for(self, predicate, seconds=8):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = predicate()
            if value:
                return value
            time.sleep(.03)
        self.fail("owned fixture deadline expired")

    def start_backup(self):
        job_id, result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        try:
            self.wait_for(self.marker.exists)
        except AssertionError:
            transcript = self.root / "operator-jobs" / job_id / "transcript.log"
            self.fail(f"fixture backup did not start: {self.status(job_id)}\n{transcript.read_text()}")
        self.wait_for(lambda: self.status(job_id).get("operator_pid"))
        return job_id

    def terminal(self, job_id):
        self.wait_for(lambda: self.status(job_id)["state"] in {"SUCCEEDED", "FAILED"})
        self.wait_for(lambda: not self._busy())
        return self.status(job_id)

    def assert_recovery_busy(self):
        before = self.log.read_bytes()
        _, attempt = self.launch("recover")
        self.assertEqual(attempt.returncode, 42, attempt.stderr)
        direct = subprocess.run(["/bin/bash", str(self.fixture.script), "--recover"],
                                env=self.env, capture_output=True, text=True, timeout=5)
        self.assertEqual(direct.returncode, 42, direct.stderr)
        self.assertEqual(self.log.read_bytes(), before, "busy recovery touched the service")

    def test_detached_hangup_busy_checks_and_durable_success(self):
        job_id = self.start_backup()
        running = self.status(job_id)
        os.kill(running["worker"]["pid"], signal.SIGHUP)
        self.assert_recovery_busy()
        self.assertEqual(self.status(job_id)["state"], "RUNNING")
        self.release.touch()
        result = self.terminal(job_id)
        self.assertEqual((result["state"], result["exit_code"]), ("SUCCEEDED", 0))
        job = self.root / "operator-jobs" / job_id
        self.assertIn("deployment=PASS", (job / "transcript.log").read_text())
        self.assertEqual((job / "state.json").stat().st_mode & 0o777, 0o600)
        self.assertEqual(job.stat().st_mode & 0o777, 0o700)
        self.assertEqual(result["operator_sha256"], fixture_module.digest(self.fixture.script))
        _, replay = self.launch(job_id=job_id)
        self.assertNotEqual(replay.returncode, 0)
        self.assertEqual(self.fixture.binary.read_text(), "new-server")

    def test_killed_operator_cannot_recover_until_orphaned_backup_finishes(self):
        job_id = self.start_backup()
        os.kill(self.status(job_id)["operator_pid"], signal.SIGKILL)
        self.wait_for(lambda: self.status(job_id)["state"] == "FAILED")
        self.assertEqual(self.status(job_id)["exit_code"], -signal.SIGKILL)
        self.assert_recovery_busy()
        self.release.touch()
        self.wait_for(lambda: not self._busy())
        self.assertEqual(self.fixture.binary.read_text(), "old-server", "killed operator made a late swap")
        recovery, result = self.launch("recover")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.terminal(recovery)["state"], "SUCCEEDED")
        self.assertEqual(self.fixture.state.read_text().strip(), "active")
        self.assertFalse((self.root / "v07-transition.json").exists())

    def test_killed_supervisor_never_invents_terminal_receipt(self):
        job_id = self.start_backup()
        os.kill(self.status(job_id)["worker"]["pid"], signal.SIGKILL)
        self.wait_for(lambda: self.status(job_id)["state"] == "SUPERVISOR_LOST")
        self.assert_recovery_busy()
        self.release.touch()
        self.wait_for(lambda: not self._busy())
        result = self.status(job_id)
        self.assertEqual((result["state"], result["exit_code"]), ("SUPERVISOR_LOST", None))
        self.assertEqual(self.fixture.binary.read_text(), "new-server")
        self.assertFalse((self.root / "v07-transition.json").exists())

    def test_killed_backup_wrapper_cannot_restart_service_over_orphan(self):
        # Close only IO, not inherited lock descriptors. Otherwise command
        # substitution correctly waits for the orphan's still-open stdout,
        # which would never exercise the independent backup-lease guard.
        (self.fixture.bin / "runuser").write_text(
            "#!/usr/bin/python3\nimport os,pathlib,time\nos.close(1); os.close(2)\n"
            "pathlib.Path(os.environ['CC_FIXTURE_ENTERED']).write_text(str(os.getpid()))\n"
            "deadline=time.monotonic()+20\n"
            "while not pathlib.Path(os.environ['CC_FIXTURE_RELEASE']).exists():\n"
            " if time.monotonic()>deadline: raise SystemExit(71)\n"
            " time.sleep(.02)\n")
        job_id = self.start_backup()
        backup_pid = int(self.marker.read_text())
        fields = Path(f"/proc/{backup_pid}/status").read_text().splitlines()
        wrapper = int(next(line.split()[1] for line in fields if line.startswith("PPid:")))
        command = Path(f"/proc/{wrapper}/cmdline").read_bytes().split(b"\0")
        self.assertEqual(command[:2], [b"/bin/bash", b"-c"])
        self.assertIn(b"cleanup", command[2])
        os.kill(wrapper, signal.SIGKILL)  # exact owned wrapper, not the backup
        self.wait_for(lambda: self.status(job_id)["state"] == "FAILED")
        self.assertEqual(self.fixture.state.read_text().strip(), "stopped")
        self.assert_recovery_busy()
        self.assertNotIn("start fixture", self.log.read_text())
        self.release.touch()
        self.wait_for(lambda: not self._busy())
        recovery, result = self.launch("recover")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.terminal(recovery)["state"], "SUCCEEDED")

    def test_term_operator_waits_for_backup_before_automatic_recovery(self):
        job_id = self.start_backup()
        os.kill(self.status(job_id)["operator_pid"], signal.SIGTERM)
        self.assert_recovery_busy()
        self.release.touch()
        result = self.terminal(job_id)
        self.assertEqual((result["state"], result["exit_code"]), ("FAILED", 143))
        self.assertEqual(self.fixture.binary.read_text(), "old-server")
        self.assertEqual(self.fixture.state.read_text().strip(), "active")
        self.assertFalse((self.root / "v07-transition.json").exists())

    def test_kill_at_swap_boundary_then_recover_exact_pair(self):
        self.mv_release = self.fixture.tmp / "mv-release"
        marker = self.fixture.tmp / "mv-entered"
        self.env.update(CC_FIXTURE_MV_RELEASE=str(self.mv_release), CC_FIXTURE_MV_ENTERED=str(marker))
        (self.fixture.bin / "mv").write_text(
            "#!/bin/sh\n[ \"$1\" = -T ] && shift\n"
            "if [ \"$1\" = \"$CC_BINARY\" ]; then\n"
            " echo entered >\"$CC_FIXTURE_MV_ENTERED\"\n"
            " while [ ! -f \"$CC_FIXTURE_MV_RELEASE\" ]; do sleep .03; done\nfi\n"
            "exec /bin/mv \"$@\"\n")
        job_id = self.start_backup()
        self.release.touch()
        self.wait_for(marker.exists)
        os.kill(self.status(job_id)["operator_pid"], signal.SIGKILL)
        self.assert_recovery_busy()
        self.mv_release.touch()
        self.wait_for(lambda: not self._busy())
        self.assertEqual(self.status(job_id)["exit_code"], -signal.SIGKILL)
        recovery, result = self.launch("recover")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.terminal(recovery)["state"], "SUCCEEDED")
        self.assertEqual(self.fixture.binary.read_text(), "old-server")
        self.assertEqual(json.loads((self.fixture.web / "build.json").read_text())["build"], fixture_module.OLD_BUILD)

    def test_hash_symlink_and_private_permissions_fail_closed(self):
        _, wrong_hash = self.launch(sha="0" * 64)
        self.assertNotEqual(wrong_hash.returncode, 0)
        self.assertFalse(self.log.exists())
        (self.root / "operator.lock").symlink_to(self.fixture.binary)
        _, linked = self.launch()
        self.assertNotEqual(linked.returncode, 0)
        self.assertFalse(self.log.exists())
        self.assertEqual(self.fixture.binary.read_text(), "old-server")
        (self.root / "operator.lock").unlink()  # owned fixture link, not a data lock
        self.launcher.chmod(0o755)
        _, public_launcher = self.launch()
        self.assertNotEqual(public_launcher.returncode, 0)

    def test_real_runuser_retains_authority_after_parent_is_killed(self):
        # Exercise util-linux runuser with a different real uid. The child can
        # use its inherited lock but cannot open the private root directory.
        code = ("import os,fcntl,time; os.fstat(9); "
                "fcntl.flock(9,fcntl.LOCK_EX|fcntl.LOCK_NB); time.sleep(3); "
                "print('Sauvegarde terminée : '+os.environ['CC_DATA_DIR']+'/backups/snapshot')")
        import shlex
        (self.fixture.bin / "runuser").write_text(
            "#!/bin/sh\necho entered >\"$CC_FIXTURE_ENTERED\"\n"
            "exec /usr/sbin/runuser -u nobody -- /usr/bin/python3 -c " + shlex.quote(code) + "\n")
        job_id = self.start_backup()
        time.sleep(.15)  # allow the actual uid-switching exec, not a fake runuser
        os.kill(self.status(job_id)["operator_pid"], signal.SIGKILL)
        self.wait_for(lambda: self.status(job_id)["state"] == "FAILED")
        self.assert_recovery_busy()
        self.wait_for(lambda: not self._busy())
        recovery, result = self.launch("recover")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.terminal(recovery)["state"], "SUCCEEDED")

    def test_stale_boot_identity_and_unobserved_start_never_become_success(self):
        job_id = str(uuid.uuid4())
        job = self.root / "operator-jobs" / job_id
        job.mkdir(mode=0o700, parents=True)
        state = job / "state.json"
        state.write_text(json.dumps({"state": "RUNNING", "exit_code": 0,
                                     "worker": {"pid": os.getpid(), "boot": "old-boot", "start": "0"}}))
        state.chmod(0o600)
        result = self.status(job_id)
        self.assertEqual((result["state"], result["exit_code"]), ("SUPERVISOR_LOST", None))
        state.write_text(json.dumps({"state": "QUEUED", "exit_code": None}))
        self.assertEqual(self.status(job_id)["state"], "START_NOT_OBSERVED")

    def test_unmodified_shell_lock_guards_reject_links_and_public_files(self):
        # Unlike the payload fixture, this uses the real root/stat/path guards.
        self.fixture.script.write_bytes(fixture_module.SOURCE.read_bytes())
        lock = self.root / "operator.lock"
        for kind in ("symlink", "hardlink", "public"):
            if kind == "symlink":
                lock.symlink_to(self.fixture.binary)
            elif kind == "hardlink":
                os.link(self.fixture.binary, lock)
            else:
                lock.write_text("")
                lock.chmod(0o644)
            result = subprocess.run(["/bin/bash", str(self.fixture.script), "--recover"],
                                    env=self.env, capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("unsafe_operation_lock", result.stderr)
            self.assertFalse(self.log.exists())
            lock.unlink()  # only a newly created, never-held fixture lock


def ssh_fixture(action, root=None, job_id=None):
    """Explicit hook for a real SSH disconnect test, never an ordinary host."""
    import socket
    if os.geteuid() != 0 or socket.gethostname() != "colima-v07-security":
        raise RuntimeError("disposable_local_VM_required")
    if action == "start":
        case = DurableJobTests()
        case.setUp()
        job_id = case.start_backup()
        print(json.dumps({"root": str(case.root), "job_id": job_id}), flush=True)
        # The caller deliberately kills only its local SSH connection here.
        # No teardown: the separate root-private worker must survive reattach.
        time.sleep(60)
    else:
        root = Path(root)
        if root.parent.parent != Path("/tmp") or not root.parent.name.startswith("pi-v07-operator-test."):
            raise RuntimeError("owned_fixture_required")
        job = root / "operator-jobs" / str(uuid.UUID(job_id))
        request = json.loads((job / "request.json").read_text())
        launcher = root.parent / "job.py"
        def status():
            command = [sys.executable, "-I", str(launcher), "status", "--root", str(root), "--job-id", job_id]
            return json.loads(subprocess.check_output(command, text=True))
        before = status()
        assert before["state"] == "RUNNING" and before["operation_busy"]
        command = [sys.executable, "-I", str(launcher), "start", "--root", str(root),
                   "--job-id", str(uuid.uuid4()), "--mode", "recover", "--operator",
                   str(root.parent / "operator.sh"), "--operator-sha256", request["operator_sha256"]]
        busy = subprocess.run(command, env=request["environment"], capture_output=True, text=True)
        assert busy.returncode == 42
        release = Path(request["environment"]["CC_FIXTURE_RELEASE"])
        assert release == root.parent / "release"
        release.touch()
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            after = status()
            if after["state"] == "SUCCEEDED" and not after["operation_busy"]:
                break
            time.sleep(.05)
        assert after["state"] == "SUCCEEDED" and after["exit_code"] == 0
        assert "deployment=PASS" in (job / "transcript.log").read_text()
        print(json.dumps({"real_ssh_disconnect": "PASS", "recovery_while_busy": "REFUSED",
                          "terminal_exit_code": after["exit_code"], "job_id": job_id}))


if __name__ == "__main__":
    if sys.argv[1:2] == ["--ssh-start"]:
        ssh_fixture("start")
    elif sys.argv[1:2] == ["--ssh-finish"]:
        ssh_fixture("finish", *sys.argv[2:])
    else:
        unittest.main(verbosity=2)
