#!/usr/bin/env python3
"""Disposable-Colima root/systemd boundary test entrypoint for updater fixes.

Never run on a Pi, NAS, developer workstation, or an already-installed host.
It makes only the documented Chill Center install roots writable and refuses
anything other than the dedicated v07 Colima VM.
"""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time

ROOTS = [pathlib.Path(p) for p in (
    "/usr/lib/chillcenter", "/etc/chillcenter", "/var/lib/chillcenter-data",
)]
SYSTEMD = pathlib.Path("/usr/lib/systemd/system")

def die(message):
    raise SystemExit("FAIL: " + message)

def run(*args, check=True, **kwargs):
    print("+", " ".join(map(str, args)))
    return subprocess.run(args, check=check, text=True, **kwargs)

def require_guard():
    if os.environ.get("CHILL_UPDATE_DISPOSABLE_VM") != "1":
        die("set CHILL_UPDATE_DISPOSABLE_VM=1 explicitly")
    if os.geteuid() != 0:
        die("must run as root in the disposable VM")
    hostname = run("/usr/bin/hostname", capture_output=True).stdout.strip()
    if hostname != "colima-v07-security":
        die("hostname guard refused %r" % hostname)
    if pathlib.Path("/run/systemd/system").exists() is False:
        die("PID 1 is not systemd")
    for path in ROOTS:
        if path.exists() or path.is_symlink():
            die("fixture host is not pristine: %s" % path)
    for unit in ("chillcenter.service", "chillcenter-update.service"):
        if (SYSTEMD / unit).exists() or (pathlib.Path("/etc/systemd/system") / unit).exists():
            die("fixture host has existing unit: " + unit)

def mode(path):
    return oct(path.stat().st_mode & 0o777)

def require(condition, message):
    if not condition:
        die(message)

def install_boundary_fixture(updater, server, repo):
    """Install real binaries and production units; no network and no private key."""
    user = "chillcenter"
    run("/usr/sbin/useradd", "--system", "--user-group", "--no-create-home", "--shell", "/usr/sbin/nologin", user)
    uid = int(run("/usr/bin/id", "-u", user, capture_output=True).stdout)
    gid = int(run("/usr/bin/id", "-g", user, capture_output=True).stdout)
    for root in ROOTS:
        root.mkdir(parents=True, mode=0o755)
    for p in ("server/releases/0.7.0-1", "web/releases", "themes/releases"):
        (ROOTS[0] / p).mkdir(parents=True, mode=0o755)
    for p in ("data", "data/updates", "data/backups"):
        q = ROOTS[2] / p; q.mkdir(parents=True, mode=0o700); os.chown(q, uid, gid)
    for p in ("trust", "cache"):
        (ROOTS[2] / p).mkdir(mode=0o700)
    shutil.copy2(updater, ROOTS[0] / "chill-update")
    shutil.copy2(server, ROOTS[0] / "server/releases/0.7.0-1/payload")
    os.chmod(ROOTS[0] / "chill-update", 0o755); os.chmod(ROOTS[0] / "server/releases/0.7.0-1/payload", 0o755)
    (ROOTS[0] / "server/current").symlink_to("releases/0.7.0-1")
    # An intentionally synthetic public key is generated only in this VM. It
    # is never a production signing key and no manifest is fetched from a network.
    run("/usr/bin/minisign", "-G", "-s", "/etc/chillcenter/release.key", "-p", "/etc/chillcenter/release.pub", "-W")
    os.chmod("/etc/chillcenter/release.key", 0o600)
    config = {"listen":"127.0.0.1:18443", "data_dir":"/var/lib/chillcenter-data/data", "tls_names":["localhost","127.0.0.1"]}
    pathlib.Path("/etc/chillcenter/server.json").write_text(json.dumps(config))
    os.chown("/etc/chillcenter/server.json", 0, gid); os.chmod("/etc/chillcenter/server.json", 0o640)
    # Recovery commands only need trusted config paths; TLS is created by the real server.
    updates = {"schema_version":1,"key_id":"synthetic-v07","public_key":"/etc/chillcenter/release.pub","state_root":"/var/lib/chillcenter-data/data/updates","trust_root":"/var/lib/chillcenter-data/trust","cache_root":"/var/lib/chillcenter-data/cache","backup_root":"/var/lib/chillcenter-data/data/backups","maintenance_flag":"/run/chillcenter-update/maintenance","quiescence_status":"/var/lib/chillcenter-data/data/updates/quiescence.json","server_config":"/etc/chillcenter/server.json","service_name":"chillcenter.service","service_uid":uid,"service_gid":gid,"health_url":"https://localhost:18443/api/2026-10-14/server","status_url":"https://localhost:18443/api/2026-10-14/admin/updates","health_resolve":"127.0.0.1:18443","health_ca_certificate":"/var/lib/chillcenter-data/data/tls/certificate.pem","component_roots":{"server":"/usr/lib/chillcenter/server","web":"/usr/lib/chillcenter/web","themes":"/usr/lib/chillcenter/themes"},"targets":{"server":"linux-aarch64","web":"web","themes":"universal"},"bootstrap_versions":{"server":"0.7.0","web":"0.0.0","themes":"0.0.0"},"api_version":"2026.10.14","database_schema":3,"renderer_version":1,"theme_format":1}
    pathlib.Path("/etc/chillcenter/updates.json").write_text(json.dumps(updates))
    os.chmod("/etc/chillcenter/updates.json", 0o640)
    for name in ("chillcenter.service", "chillcenter-update.service"):
        shutil.copy2(repo / "deploy/updates" / name, SYSTEMD / name)
    run("/usr/bin/systemctl", "daemon-reload")
    return uid

def service_boundary(uid):
    run("/usr/bin/systemctl", "start", "chillcenter.service")
    time.sleep(1)
    run("/usr/bin/systemctl", "is-active", "--quiet", "chillcenter.service")
    # Actual service UID can traverse/read slots, but cannot write their root-owned parents.
    read = run("/usr/sbin/runuser", "-u", "chillcenter", "--", "/usr/bin/test", "-r", "/usr/lib/chillcenter/server/current/payload", check=False)
    require(read.returncode == 0, "service UID cannot read server slot")
    write = run("/usr/sbin/runuser", "-u", "chillcenter", "--", "/usr/bin/touch", "/usr/lib/chillcenter/server/forbidden", check=False)
    require(write.returncode != 0, "service UID wrote root-owned slot")
    require(mode(pathlib.Path("/usr/lib/chillcenter/server/releases/0.7.0-1")) in ("0o750", "0o755"), "slot directory mode is not service-traversable")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--updater", required=True, type=pathlib.Path)
    parser.add_argument("--server", required=True, type=pathlib.Path)
    parser.add_argument("--repo", default=pathlib.Path(__file__).resolve().parents[1], type=pathlib.Path)
    args = parser.parse_args()
    require_guard()
    for binary in (args.updater, args.server):
        require(binary.is_file() and os.access(binary, os.X_OK), "missing executable: %s" % binary)
    uid = install_boundary_fixture(args.updater, args.server, args.repo)
    service_boundary(uid)
    # Exact live-root checks: Linux root-only ledger plus production recovery under
    # the real service sandbox.  These are intentionally fail-closed if the
    # cross-built binaries/Minisign/systemd image do not meet prerequisites.
    env = dict(os.environ, CHILL_UPDATE_DISPOSABLE_VM="1")
    run("/usr/bin/systemctl", "show", "chillcenter.service", "-p", "ProtectSystem", "-p", "ReadWritePaths", "-p", "ProtectProc", "-p", "ProcSubset")
    run("/usr/lib/chillcenter/chill-update", "--config", "/etc/chillcenter/updates.json", "recover", "server", env=env)
    print("PASS: real service UID root-slot boundary and root recovery preflight")
    print("NOT COVERED: signed cache/apply and TERM/KILL fault injection require a valid synthetic manifest/cache receipt generator matching the final helper CLI; do not claim CU-SEC-002/004/005/008 closed from this harness run.")

if __name__ == "__main__":
    main()
