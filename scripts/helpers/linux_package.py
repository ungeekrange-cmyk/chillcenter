#!/usr/bin/env python3
"""Small dependency-free checks used by the Linux packaging script."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import struct
import sys


MACHINES = {
    "aarch64-unknown-linux-gnu": (2, 183),
    "armv7-unknown-linux-gnueabihf": (1, 40),
    "x86_64-unknown-linux-gnu": (2, 62),
}


def validate_elf(path: pathlib.Path, target: str) -> None:
    expected = MACHINES.get(target)
    if expected is None:
        raise ValueError(f"unsupported resolved Linux target: {target}")
    data = path.read_bytes()[:20]
    if len(data) < 20 or data[:4] != b"\x7fELF" or data[5] not in (1, 2):
        raise ValueError("server output is not an ELF executable")
    elf_class = data[4]
    endian = "<" if data[5] == 1 else ">"
    machine = struct.unpack(f"{endian}H", data[18:20])[0]
    if (elf_class, machine) != expected:
        raise ValueError(
            f"ELF architecture mismatch: target {target}, "
            f"class={elf_class}, machine={machine}"
        )


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_manifest(args: argparse.Namespace) -> None:
    binary = pathlib.Path(args.binary)
    manifest = {
        "format": 1,
        "artifact": "chill-server",
        "target": args.target,
        "source_commit": args.source_commit,
        "binary_sha256": sha256(binary),
        "cargo": args.cargo_version,
        "rustc": args.rustc_version,
        "build": "cargo build --release --locked --offline -p chill-server --target <target>",
        "runtime": {
            "transcoding": False,
            "libmpv": False,
            "deployment_performed": False,
        },
    }
    pathlib.Path(args.output).write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    validate = commands.add_parser("validate-elf")
    validate.add_argument("--path", required=True)
    validate.add_argument("--target", required=True)
    checksum = commands.add_parser("sha256")
    checksum.add_argument("--path", required=True)
    manifest = commands.add_parser("manifest")
    manifest.add_argument("--binary", required=True)
    manifest.add_argument("--target", required=True)
    manifest.add_argument("--source-commit", required=True)
    manifest.add_argument("--cargo-version", required=True)
    manifest.add_argument("--rustc-version", required=True)
    manifest.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        if args.command == "validate-elf":
            validate_elf(pathlib.Path(args.path), args.target)
        elif args.command == "sha256":
            print(sha256(pathlib.Path(args.path)))
        else:
            write_manifest(args)
    except (OSError, ValueError) as error:
        print(f"Linux package validation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
