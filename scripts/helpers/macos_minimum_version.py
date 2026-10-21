#!/usr/bin/env python3
"""Advertise the OS actually required by every packaged Mach-O, before signing."""

import argparse
from pathlib import Path
import plistlib
import re
import subprocess


def version(value):
    if not re.fullmatch(r"\d+\.\d+(?:\.\d+)?", value):
        raise ValueError(f"Invalid macOS version: {value!r}")
    parts = tuple(int(part) for part in value.split("."))
    return parts + (0,) * (3 - len(parts))


def macho_minimum(output):
    # vtool also prints SDK versions: those are not deployment requirements.
    if "LC_BUILD_VERSION" in output:
        if not re.search(r"^\s*platform MACOS\s*$", output, re.MULTILINE):
            raise ValueError("Expected a macOS Mach-O build version")
        match = re.search(r"^\s*minos (\S+)\s*$", output, re.MULTILINE)
    elif "LC_VERSION_MIN_MACOSX" in output:
        match = re.search(r"^\s*version (\S+)\s*$", output, re.MULTILINE)
    else:
        match = None
    if match is None:
        raise ValueError("Mach-O has no recognizable macOS deployment minimum")
    return version(match.group(1))


def update_minimum(bundle, inspect=None, maximum=None):
    info_path = bundle / "Contents/Info.plist"
    with info_path.open("rb") as stream:
        info = plistlib.load(stream)
    executable = info["CFBundleExecutable"]
    if Path(executable).name != executable or executable in {".", ".."}:
        raise ValueError("Invalid bundle executable name")
    libraries = sorted((bundle / "Contents/Frameworks").glob("*.dylib"))
    if not libraries:
        raise ValueError("Packaged media libraries are missing")
    files = [bundle / "Contents/MacOS" / executable, *libraries]
    required = version(info["LSMinimumSystemVersion"])
    for binary in files:
        output = inspect(binary) if inspect else subprocess.check_output(
            ["/usr/bin/vtool", "-show-build", str(binary)], text=True
        )
        required = max(required, macho_minimum(output))
    # Refuse incompatible inputs before changing plist/signing. A newer SDK is
    # fine; a newer deployment minimum is not a macOS 15-compatible build.
    if maximum is not None and required > version(maximum):
        raise ValueError(f"Bundle requires {required}, exceeding supported minimum {maximum}")
    # Only the new staging bundle is modified; this must run before codesign.
    value = f"{required[0]}.{required[1]}" + (f".{required[2]}" if required[2] else "")
    info["LSMinimumSystemVersion"] = value
    with info_path.open("wb") as stream:
        plistlib.dump(info, stream)
    return value, len(files)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--maximum", help="Fail if any deployment minimum exceeds this target")
    arguments = parser.parse_args()
    minimum, count = update_minimum(arguments.bundle, maximum=arguments.maximum)
    print(f"Bundle requires macOS {minimum} or newer ({count} Mach-O files inspected).")
