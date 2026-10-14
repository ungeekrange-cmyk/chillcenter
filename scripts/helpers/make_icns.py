#!/usr/bin/env python3
"""Build a modern PNG-backed ICNS file from the approved square artwork."""

from __future__ import annotations

import argparse
from pathlib import Path
import struct
import subprocess
import tempfile


CHUNKS = (
    (16, b"icp4"),
    (32, b"icp5"),
    (64, b"icp6"),
    (128, b"ic07"),
    (256, b"ic08"),
    (512, b"ic09"),
    (1024, b"ic10"),
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    arguments = parser.parse_args()
    payload = bytearray()
    with tempfile.TemporaryDirectory(prefix="chillcenter-icns-") as temporary:
        temporary_path = Path(temporary)
        for size, kind in CHUNKS:
            image = temporary_path / f"{size}.png"
            subprocess.check_call(
                [
                    "/usr/bin/sips",
                    "-s",
                    "format",
                    "png",
                    "-z",
                    str(size),
                    str(size),
                    str(arguments.source),
                    "--out",
                    str(image),
                ],
                stdout=subprocess.DEVNULL,
            )
            data = image.read_bytes()
            payload.extend(kind)
            payload.extend(struct.pack(">I", len(data) + 8))
            payload.extend(data)
    output = b"icns" + struct.pack(">I", len(payload) + 8) + payload
    arguments.destination.write_bytes(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
