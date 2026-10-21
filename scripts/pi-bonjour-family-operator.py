#!/usr/bin/env python3
"""Root-only, reversible AF_NETLINK systemd drop-in operator.

This operator deliberately does not install an application binary.  It stages
the single ADR-0032 exception before the reviewed binary operator takes its
unit baseline, and can later remove only the file it created.  It never edits
the vendor unit or a pre-existing drop-in.
"""
import argparse
import hashlib
import json
import os
import pathlib
import stat
import subprocess
import sys
import time
import uuid

SERVICE = "chillcenter"
DROPIN_NAME = "20-discovery-netlink.conf"
DROPIN_BYTES = b"[Service]\nRestrictAddressFamilies=AF_NETLINK\n"
BASE_FAMILIES = frozenset(("AF_UNIX", "AF_INET", "AF_INET6"))
JOURNAL = "bonjour-family-transition.json"


class Refusal(RuntimeError):
    pass


def fail(reason):
    raise Refusal(reason)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha_file(path):
    with open(path, "rb") as handle:
        return digest(handle.read())


def private_dir(path):
    info = os.lstat(path)
    if (not stat.S_ISDIR(info.st_mode) or stat.S_ISLNK(info.st_mode)
            or info.st_uid != 0 or stat.S_IMODE(info.st_mode) != 0o700):
        fail("private_root_0700_required")


def regular_root_0644(path):
    info = os.lstat(path)
    if (not stat.S_ISREG(info.st_mode) or stat.S_ISLNK(info.st_mode)
            or info.st_uid != 0 or stat.S_IMODE(info.st_mode) != 0o644):
        fail("owned_dropin_metadata_invalid")


def command(*argv, check=True):
    result = subprocess.run(argv, text=True, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=60)
    if check and result.returncode:
        fail("systemctl_failed:" + argv[1])
    return result


def effective_families(service):
    value = command("systemctl", "show", "--property=RestrictAddressFamilies",
                    "--value", service).stdout.strip()
    if not value or "=" in value:
        fail("restrict_address_families_unreadable")
    tokens = frozenset(value.split())
    if any(not token.startswith("AF_") for token in tokens):
        fail("restrict_address_families_unreadable")
    return tokens


def systemctl_cat(service):
    result = command("systemctl", "cat", service)
    if not result.stdout:
        fail("unit_bytes_unreadable")
    return result.stdout.encode()


def fsync_dir(path):
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def exclusive_file(path, data, mode):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        os.fchmod(descriptor, mode)
        os.write(descriptor, data)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    fsync_dir(path.parent)


def write_journal(path, value):
    temporary = path.with_name(path.name + ".new")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        os.fchmod(descriptor, 0o600)
        payload = json.dumps(value, sort_keys=True).encode()
        os.write(descriptor, payload)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    os.replace(temporary, path)
    fsync_dir(path.parent)


def load_journal(path, rollback_dir, dropin):
    info = os.lstat(path)
    if (not stat.S_ISREG(info.st_mode) or stat.S_ISLNK(info.st_mode)
            or info.st_uid != 0 or stat.S_IMODE(info.st_mode) != 0o600):
        fail("journal_not_root_0600")
    try:
        value = json.loads(path.read_bytes())
    except (OSError, ValueError):
        fail("invalid_journal")
    required = {"state", "service", "dropin", "dropin_sha256", "unit_sha256",
                "families_before", "operation_id"}
    if set(value) != required or value["state"] not in {"prepared", "applied", "rollback_pending", "rollback_complete"}:
        fail("invalid_journal")
    if value["service"] != SERVICE or value["dropin"] != str(dropin):
        fail("journal_binding_refused")
    if value["dropin_sha256"] != digest(DROPIN_BYTES):
        fail("journal_dropin_hash_refused")
    if value["families_before"] != sorted(BASE_FAMILIES):
        fail("journal_baseline_refused")
    if not isinstance(value["operation_id"], str) or len(value["operation_id"]) != 32:
        fail("journal_operation_refused")
    for name in ("unit_sha256", "dropin_sha256"):
        if not isinstance(value[name], str) or len(value[name]) != 64:
            fail("journal_hash_refused")
    for name in ("original-unit.txt", "original-families.txt", "installed-dropin.conf"):
        expected = rollback_dir / (value["operation_id"] + "-" + name)
        if not expected.exists():
            fail("private_snapshot_missing")
    return value


def dropin_path():
    return pathlib.Path("/etc/systemd/system") / (SERVICE + ".service.d") / DROPIN_NAME


def apply(rollback_dir, dropin):
    private_dir(rollback_dir)
    journal = rollback_dir / JOURNAL
    if journal.exists():
        fail("unfinished_transition_requires_rollback")
    if dropin.exists() or dropin.is_symlink():
        fail("owned_dropin_path_already_exists")
    before = effective_families(SERVICE)
    if before != BASE_FAMILIES:
        fail("unexpected_preexisting_family_set")
    unit = systemctl_cat(SERVICE)
    operation_id = uuid.uuid4().hex
    # Preserve the exact provider unit/effective set and the exact bytes we own,
    # all in the already-validated private rollback directory.
    exclusive_file(rollback_dir / (operation_id + "-original-unit.txt"), unit, 0o600)
    exclusive_file(rollback_dir / (operation_id + "-original-families.txt"),
                   (" ".join(sorted(before)) + "\n").encode(), 0o600)
    exclusive_file(rollback_dir / (operation_id + "-installed-dropin.conf"), DROPIN_BYTES, 0o600)
    record = {"state": "prepared", "service": SERVICE,
              "dropin": str(dropin), "dropin_sha256": digest(DROPIN_BYTES),
              "unit_sha256": digest(unit), "families_before": sorted(before),
              "operation_id": operation_id}
    # Publish recoverable intent before the first remote unit write.  A crash or
    # failed daemon-reload can therefore be resumed by rollback rather than
    # leaving an untracked relaxation behind.
    write_journal(journal, record)
    try:
        dropin.parent.mkdir(mode=0o755, parents=True)
    except FileExistsError:
        pass
    parent = os.lstat(dropin.parent)
    if not stat.S_ISDIR(parent.st_mode) or stat.S_ISLNK(parent.st_mode) or parent.st_uid != 0:
        fail("dropin_directory_invalid")
    exclusive_file(dropin, DROPIN_BYTES, 0o644)
    regular_root_0644(dropin)
    command("systemctl", "daemon-reload")
    after = effective_families(SERVICE)
    if after != BASE_FAMILIES | {"AF_NETLINK"}:
        fail("effective_family_set_not_exact")
    write_journal(journal, {**record, "state": "applied"})
    print("bonjour_family=APPLIED")


def rollback(rollback_dir, dropin):
    private_dir(rollback_dir)
    journal_path = rollback_dir / JOURNAL
    journal = load_journal(journal_path, rollback_dir, dropin)
    if journal["state"] == "rollback_complete":
        if effective_families(SERVICE) != BASE_FAMILIES:
            fail("rollback_family_set_unproven")
        print("bonjour_family_rollback=PASS")
        return
    exists = dropin.exists() or dropin.is_symlink()
    if exists:
        regular_root_0644(dropin)
        if sha_file(dropin) != journal["dropin_sha256"]:
            fail("owned_dropin_hash_changed")
        write_journal(journal_path, {**journal, "state": "rollback_pending"})
        # This is the only remote configuration path this operator removes.
        dropin.unlink()
        fsync_dir(dropin.parent)
    elif journal["state"] not in {"prepared", "rollback_pending"}:
        fail("owned_dropin_missing")
    if journal["state"] == "prepared" and not exists:
        if effective_families(SERVICE) != BASE_FAMILIES:
            fail("rollback_family_set_unproven")
        write_journal(journal_path, {**journal, "state": "rollback_complete"})
        print("bonjour_family_rollback=PASS")
        return
    command("systemctl", "daemon-reload")
    # The saved provider unit and every pre-existing drop-in must be byte-for-
    # byte the pre-change view before a restart can claim sandbox restoration.
    if digest(systemctl_cat(SERVICE)) != journal["unit_sha256"]:
        fail("original_unit_hash_unproven")
    command("systemctl", "restart", SERVICE)
    if command("systemctl", "is-active", SERVICE, check=False).stdout.strip() != "active":
        fail("service_not_active_after_sandbox_rollback")
    if effective_families(SERVICE) != BASE_FAMILIES:
        fail("rollback_family_set_unproven")
    write_journal(journal_path, {**journal, "state": "rollback_complete"})
    print("bonjour_family_rollback=PASS")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("apply", "rollback"))
    parser.add_argument("--rollback-dir", type=pathlib.Path, required=True)
    args = parser.parse_args()
    if os.geteuid() != 0:
        fail("root_required")
    if not args.rollback_dir.is_absolute():
        fail("absolute_paths_required")
    target = dropin_path()
    if args.mode == "apply":
        apply(args.rollback_dir, target)
    else:
        rollback(args.rollback_dir, target)


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, subprocess.SubprocessError) as error:
        print("REFUSED:" + str(error), file=sys.stderr)
        sys.exit(2)
