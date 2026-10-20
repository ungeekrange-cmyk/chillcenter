#!/usr/bin/env python3
"""Create a read-only drag-to-Applications DMG from a verified Mac bundle.

This is manual-install media, not a replacement for Tauri's signed update TAR.
No app is launched, installed, resigned or overwritten by this packager.
"""

import argparse
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile


def validate_paths(bundle, destination):
    if not bundle.is_absolute() or not destination.is_absolute():
        raise ValueError("Use absolute app and DMG paths")
    if bundle.suffix != ".app" or not bundle.is_dir() or bundle.is_symlink():
        raise ValueError("Input must be a regular .app directory")
    if destination.suffix != ".dmg":
        raise ValueError("Output must have the .dmg extension")
    if destination.exists() or destination.is_symlink():
        raise FileExistsError("Destination exists; no disk image was overwritten")
    if bundle.resolve() in destination.resolve().parents:
        raise ValueError("DMG output must be outside the input application")
    with (bundle / "Contents/Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if info.get("CFBundleIdentifier") != "home.chillcenter.desktop":
        raise ValueError("Expected the normal Chill Center application, not a QA bundle")
    for field in ("CFBundleShortVersionString", "LSMinimumSystemVersion"):
        value = info.get(field)
        if not isinstance(value, str) or not value or any(c not in "0123456789." for c in value):
            raise ValueError(f"Missing or invalid {field}")
    return info


def publish_image(stage, destination):
    # A hard link is an atomic, exclusive file publication on the same volume.
    # Unlike rename/replace it cannot overwrite a destination that appeared
    # after preflight. TemporaryDirectory removes only our other link later.
    os.link(stage, destination)


def package(bundle, destination):
    if sys.platform != "darwin":
        raise RuntimeError("DMG packaging requires macOS system tools")
    info = validate_paths(bundle, destination)
    subprocess.run(["/usr/bin/codesign", "--verify", "--deep", "--strict", str(bundle)], check=True)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".chillcenter-dmg-", dir=destination.parent) as temporary:
        stage = Path(temporary)
        content = stage / "content"
        content.mkdir()
        app = content / "Chill Center.app"
        shutil.copytree(bundle, app, symlinks=True)
        subprocess.run(["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app)], check=True)
        (content / "Applications").symlink_to("/Applications", target_is_directory=True)
        (content / "Installation.txt").write_text(
            f"Chill Center {info['CFBundleShortVersionString']} — Apple Silicon\n"
            f"macOS {info['LSMinimumSystemVersion']}+\n\n"
            "FR — Quittez Chill Center, puis glissez l'application sur Applications.\n"
            "Éjectez ce disque et ouvrez Chill Center depuis Applications.\n\n"
            "EN — Quit Chill Center, then drag the app onto Applications.\n"
            "Eject this disk and open Chill Center from Applications.\n\n"
            "Signature ad hoc, sans notarisation Apple.\n"
            "Ad-hoc signed; not Apple-notarized.\n",
            encoding="utf-8",
        )
        image = stage / "Chill-Center.dmg"
        subprocess.run([
            "/usr/bin/hdiutil", "create", "-quiet", "-format", "UDZO", "-fs", "HFS+",
            "-volname", "Chill Center", "-srcfolder", str(content), "-o", str(image),
        ], check=True)
        subprocess.run(["/usr/bin/hdiutil", "verify", str(image)], check=True)
        publish_image(image, destination)
    print(f"Read-only manual installer: {destination}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    parser.add_argument("destination", type=Path)
    arguments = parser.parse_args()
    try:
        package(arguments.bundle, arguments.destination)
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"DMG not published: {error}\n")
