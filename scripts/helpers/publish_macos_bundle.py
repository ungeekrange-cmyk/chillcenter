#!/usr/bin/env python3
"""Atomic macOS bundle publication. Never merge into or replace an existing app."""
import ctypes
import os
from pathlib import Path
import sys


def publish_directory(stage: Path, target: Path) -> None:
    if sys.platform != "darwin":
        raise RuntimeError("macOS publication requires renamex_np")
    if not stage.is_absolute() or not target.is_absolute():
        raise ValueError("absolute directory paths required")
    if not stage.is_dir() or stage.is_symlink():
        raise ValueError("regular staging directory required")
    # Apple SDK sys/stdio.h: RENAME_EXCL=0x4, available since macOS 10.12.
    rename = ctypes.CDLL(None, use_errno=True).renamex_np
    rename.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    if rename(os.fsencode(stage), os.fsencode(target), 0x4) != 0:
        errno = ctypes.get_errno()
        raise OSError(errno, os.strerror(errno), str(target))


def publish(stage: Path, target: Path) -> None:
    if target.suffix != ".app" or stage.parent != target.parent:
        raise ValueError("same-directory stage and .app target required")
    publish_directory(stage, target)


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("usage: publish_macos_bundle.py STAGE NEW.app")
        publish(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, RuntimeError) as error:
        print(f"Bundle not published; existing destination unchanged: {error}", file=sys.stderr)
        raise SystemExit(1)
