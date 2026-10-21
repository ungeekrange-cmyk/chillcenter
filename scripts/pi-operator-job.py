#!/usr/bin/env python3
"""Root-only durable launcher for the reviewed legacy server/web operator.

No SSH, downloader, installer or recovery policy lives here. The caller stages
reviewed bytes and explicit CC_* pins, chooses a UUID before connecting, then
uses start/status. A lost transport is never a successful/failed deployment.
"""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import uuid


class Refusal(RuntimeError):
    pass


def private_dir(path):
    """Reject aliases and writable ancestors; /tmp's root-owned sticky parent is OK."""
    if not path.is_absolute() or path.resolve() != path:
        raise Refusal("canonical_absolute_directory_required")
    for parent in (path, *path.parents):
        info = parent.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0:
            raise Refusal("root_owned_directory_required")
        if info.st_mode & 0o022 and not (info.st_uid == 0 and info.st_mode & stat.S_ISVTX):
            raise Refusal("writable_ancestor_refused")
    if stat.S_IMODE(path.stat().st_mode) != 0o700:
        raise Refusal("private_directory_required")


def open_file(path, flags=os.O_RDONLY, mode=0o600):
    fd = os.open(path, flags | os.O_NOFOLLOW | os.O_NONBLOCK, mode)
    info = os.fstat(fd)
    if (not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or
            info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != mode):
        os.close(fd)
        raise Refusal("unsafe_private_file")
    return fd


def read_file(path, limit=1024 * 1024, mode=0o600):
    with os.fdopen(open_file(path, mode=mode), "rb") as stream:
        data = stream.read(limit + 1)
    if len(data) > limit:
        raise Refusal("file_too_large")
    return data


def sync_dir(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def write_file(path, data):
    with os.fdopen(open_file(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL), "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def atomic_state(job, value):
    value = {**value, "recorded_at": datetime.now(timezone.utc).isoformat()}
    temp = job / (".state-" + uuid.uuid4().hex)
    write_file(temp, (json.dumps(value, sort_keys=True) + "\n").encode())
    os.replace(temp, job / "state.json")
    sync_dir(job)


def identity(pid):
    """Linux boot + start tick prevents PID reuse from reviving a stale receipt."""
    try:
        raw = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        if raw[0] == "Z":
            return None
        return {"pid": pid, "start": raw[19],
                "boot": Path("/proc/sys/kernel/random/boot_id").read_text().strip()}
    except (FileNotFoundError, ProcessLookupError):
        return None


def job_path(root, job_id):
    if str(uuid.UUID(job_id)) != job_id:
        raise Refusal("canonical_job_uuid_required")
    return root / "operator-jobs" / job_id


def lock_fd(root, create=False):
    return open_file(root / "operator.lock", os.O_RDWR | (os.O_CREAT if create else 0))


def busy(root):
    try:
        fd = lock_fd(root)
    except FileNotFoundError:
        return False
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return False
        except BlockingIOError:
            return True
    finally:
        os.close(fd)  # never LOCK_UN on an inherited open-file description


def start(args):
    root = args.root
    private_dir(root)
    if os.environ.get("CC_ROLLBACK_DIR") != str(root):
        raise Refusal("rollback_root_mismatch")
    operator = args.operator
    private_dir(operator.parent)
    code = read_file(operator, mode=0o700)
    if hashlib.sha256(code).hexdigest() != args.operator_sha256:
        raise Refusal("reviewed_operator_hash_mismatch")
    # Detached execution must not reopen a mutable user-owned launcher.
    launcher = Path(__file__).absolute()
    private_dir(launcher.parent)
    launcher_bytes = read_file(launcher, mode=0o700)
    fd = lock_fd(root, create=True)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise Refusal("operation_busy_poll_existing_job") from None
        jobs = root / "operator-jobs"
        jobs.mkdir(mode=0o700, exist_ok=True)
        private_dir(jobs)
        job = job_path(root, args.job_id)
        job.mkdir(mode=0o700)  # no retry can reuse or overwrite an existing UUID
        write_file(job / "operator.sh", code)
        os.chmod(job / "operator.sh", 0o700)
        write_file(job / "launcher.py", launcher_bytes)
        os.chmod(job / "launcher.py", 0o700)
        request = {"mode": args.mode, "operator_sha256": args.operator_sha256,
                   "environment": {k: v for k, v in os.environ.items() if k.startswith("CC_")}}
        write_file(job / "request.json", json.dumps(request).encode())
        atomic_state(job, {"state": "QUEUED", "exit_code": None})
        sync_dir(job)
        sync_dir(jobs)
        # FD 9 reserves authority before acknowledging start; the supervisor,
        # bash and backup descendants all inherit the SAME kernel lock.
        if fd != 9:
            os.dup2(fd, 9, inheritable=True)
        log = open_file(job / "transcript.log", os.O_WRONLY | os.O_CREAT | os.O_EXCL)
        try:
            subprocess.Popen([sys.executable, "-I", str(job / "launcher.py"),
                              "_run", str(root), args.job_id],
                             stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                             start_new_session=True, pass_fds=(9,),
                             env={"PATH": "/usr/sbin:/usr/bin:/sbin:/bin", "LANG": "C.UTF-8"})
        except OSError:
            atomic_state(job, {"state": "LAUNCH_FAILED", "exit_code": None})
            raise
        finally:
            os.close(log)
            if fd != 9:
                os.close(9)
        return {"job_id": args.job_id, "state": "QUEUED"}
    finally:
        os.close(fd)


def run_job(root, job_id):
    private_dir(root)
    job = job_path(root, job_id)
    private_dir(job)
    signal.signal(signal.SIGHUP, signal.SIG_IGN)
    request = json.loads(read_file(job / "request.json", 65536))
    if request["mode"] not in {"preflight", "deploy", "recover"}:
        raise Refusal("invalid_job_mode")
    if request["environment"].get("CC_ROLLBACK_DIR") != str(root):
        raise Refusal("rollback_root_mismatch")
    code = read_file(job / "operator.sh", mode=0o700)
    if hashlib.sha256(code).hexdigest() != request["operator_sha256"]:
        raise Refusal("operator_snapshot_changed")
    expected = (root / "operator.lock").stat()
    actual = os.fstat(9)
    if (expected.st_dev, expected.st_ino) != (actual.st_dev, actual.st_ino):
        raise Refusal("inherited_lock_mismatch")
    fcntl.flock(9, fcntl.LOCK_EX | fcntl.LOCK_NB)
    state = {"state": "RUNNING", "exit_code": None, "worker": identity(os.getpid()),
             "mode": request["mode"], "operator_sha256": request["operator_sha256"]}
    atomic_state(job, state)
    # No BASH_ENV, LD_PRELOAD, PYTHONPATH, shell expansion or inherited SSH IO.
    env = {"PATH": "/usr/sbin:/usr/bin:/sbin:/bin", "LANG": "C.UTF-8"}
    env.update({k: v for k, v in request["environment"].items() if k.startswith("CC_")})
    child = subprocess.Popen(["/bin/bash", str(job / "operator.sh"), "--" + request["mode"]],
                             stdin=subprocess.DEVNULL, pass_fds=(9,), env=env)
    state["operator_pid"] = child.pid
    atomic_state(job, state)
    result = child.wait()
    # Flush the durable transcript before publishing the terminal receipt.
    os.fsync(1)
    state.update(state="SUCCEEDED" if result == 0 else "FAILED", exit_code=result)
    atomic_state(job, state)


def status(root, job_id):
    private_dir(root)
    job = job_path(root, job_id)
    private_dir(job)
    state = json.loads(read_file(job / "state.json", 65536))
    occupied = busy(root)
    if state["state"] == "RUNNING" and identity(state["worker"]["pid"]) != state["worker"]:
        state.update(state="SUPERVISOR_LOST", exit_code=None)
    elif state["state"] == "QUEUED" and not occupied:
        state.update(state="START_NOT_OBSERVED", exit_code=None)
    # Not a receipt of success and never an automatic recovery recommendation.
    # A surviving backup may hold the operation lock after its supervisor dies.
    return {"job_id": job_id, **state, "operation_busy": occupied}


def main():
    if sys.platform != "linux" or os.geteuid() != 0:
        raise Refusal("linux_root_required")
    os.umask(0o077)
    if len(sys.argv) == 4 and sys.argv[1] == "_run":
        run_job(Path(sys.argv[2]), sys.argv[3])
        return
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("start", "status"):
        sub = commands.add_parser(command)
        sub.add_argument("--root", type=Path, required=True)
        sub.add_argument("--job-id", required=True)
        if command == "start":
            sub.add_argument("--mode", choices=("preflight", "deploy", "recover"), required=True)
            sub.add_argument("--operator", type=Path, required=True)
            sub.add_argument("--operator-sha256", required=True)
    args = parser.parse_args()
    print(json.dumps(start(args) if args.command == "start" else status(args.root, args.job_id)))


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, ValueError, KeyError) as error:
        # Do not echo paths, environment, input JSON, pins or raw exception text.
        print(str(error) if isinstance(error, Refusal) else "job_io_or_receipt_refused", file=sys.stderr)
        sys.exit(42 if isinstance(error, Refusal) and str(error).startswith("operation_busy") else 2)
