#!/usr/bin/env python3
"""Host-side real SSH disconnect proof, restricted to the disposable loopback VM.

The guest script also refuses any hostname except colima-v07-security. No Pi
address, agent forwarding or household credentials are used by this harness.
"""
import argparse
import json
from pathlib import Path
import select
import shlex
import signal
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ssh-config", required=True)
    parser.add_argument("--guest-script", default=str(Path(__file__).resolve().with_name("test-pi-operator-job.py")))
    args = parser.parse_args()
    base = ["ssh", "-F", args.ssh_config, "-o", "ControlMaster=no", "-o", "ControlPath=none",
            "-o", "ForwardAgent=no", "-T", "lima-colima-v07-security"]
    effective = subprocess.check_output(base[:-1] + ["-G", base[-1]], text=True)
    options = dict(line.split(" ", 1) for line in effective.splitlines())
    assert options["hostname"] == "127.0.0.1", "disposable loopback VM only"
    assert options.get("proxycommand", "none") == options.get("proxyjump", "none") == "none"
    remote = ["sudo", "-n", "python3", "-B", "-u", args.guest_script]
    start = subprocess.Popen(base + [shlex.join(remote + ["--ssh-start"])],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        assert select.select([start.stdout], [], [], 15)[0], "fixture start deadline"
        receipt = json.loads(start.stdout.readline())
        start.kill()  # the actual local SSH transport, not the remote worker
        assert start.wait(timeout=5) == -signal.SIGKILL
    finally:
        if start.poll() is None:
            start.kill()
            start.wait(timeout=5)
        start.stdout.close()
        start.stderr.close()
    finish = subprocess.run(base + [shlex.join(remote + ["--ssh-finish", receipt["root"], receipt["job_id"]])],
                            capture_output=True, text=True, timeout=15)
    assert finish.returncode == 0, finish.stderr
    print(finish.stdout.strip())


if __name__ == "__main__":
    main()
