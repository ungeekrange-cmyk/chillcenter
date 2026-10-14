#!/usr/bin/env python3
"""Atomic macOS bundle publication. Never merge into or replace an existing app."""
import ctypes
import os
from pathlib import Path
import sys


def publish(stage: Path, target: Path) -> None:
    if sys.platform != "darwin":
        raise RuntimeError("macOS publication requires renamex_np")
    if not stage.is_absolute() or not target.is_absolute() or target.suffix != ".app":
        raise ValueError("absolute stage and .app target required")
    if stage.parent != target.parent or not stage.is_dir() or stage.is_symlink():
        raise ValueError("same-directory regular staging bundle required")
    # Apple SDK sys/stdio.h: RENAME_EXCL=0x4, available since macOS 10.12.
    rename = ctypes.CDLL(None, use_errno=True).renamex_np
    rename.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    if rename(os.fsencode(stage), os.fsencode(target), 0x4) != 0:
        errno = ctypes.get_errno()
        raise OSError(errno, os.strerror(errno), str(target))


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("usage: publish_macos_bundle.py STAGE NEW.app")
        publish(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, RuntimeError) as error:
        print(f"Bundle not published; existing destination unchanged: {error}", file=sys.stderr)
        raise SystemExit(1)
