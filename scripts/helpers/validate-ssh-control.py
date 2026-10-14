#!/usr/bin/env python3
"""Validate a user-owned private SSH ControlMaster socket without opening it."""

import os
import stat
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("Usage: validate-ssh-control.py /absolute/private-control-socket", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    if not path.is_absolute():
        print("SSH control socket must be an absolute path.", file=sys.stderr)
        return 2
    if any(token in str(path) for token in ("%", "$")):
        print("SSH control socket path must not contain SSH expansion tokens.", file=sys.stderr)
        return 1
    parent = path.parent
    try:
        parent_metadata = parent.lstat()
    except OSError:
        print("SSH control socket parent is inaccessible.", file=sys.stderr)
        return 1
    if not stat.S_ISDIR(parent_metadata.st_mode) or stat.S_ISLNK(parent_metadata.st_mode):
        print("SSH control socket parent is not a real directory.", file=sys.stderr)
        return 1
    if parent_metadata.st_uid != os.getuid():
        print("SSH control socket parent is not owned by the current user.", file=sys.stderr)
        return 1
    if stat.S_IMODE(parent_metadata.st_mode) != 0o700:
        print("SSH control socket parent must have mode 0700.", file=sys.stderr)
        return 1
    try:
        metadata = path.lstat()
    except OSError:
        print("SSH control socket is inaccessible.", file=sys.stderr)
        return 1
    if not stat.S_ISSOCK(metadata.st_mode):
        print("SSH control path is not a socket.", file=sys.stderr)
        return 1
    if metadata.st_uid != os.getuid():
        print("SSH control socket is not owned by the current user.", file=sys.stderr)
        return 1
    if stat.S_IMODE(metadata.st_mode) & 0o077:
        print("SSH control socket permits group or other access.", file=sys.stderr)
        return 1
    print("SSH control socket validation: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
