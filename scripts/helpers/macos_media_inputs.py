#!/usr/bin/env python3
"""Prepare pinned Sequoia media inputs without installing or executing a bottle."""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tarfile
import tempfile

NAME = re.compile(r"[a-z0-9][a-z0-9@+._-]{0,79}")
VERSION = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,79}")
LICENSE = re.compile(r"^(license|copying|copyright|notice)(\.|$)", re.I)
MAX_ARCHIVE = 256 * 1024 * 1024
MAX_UNPACKED = 512 * 1024 * 1024


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_lock(path):
    lock = json.loads(path.read_text(encoding="utf-8"))
    if (lock.get("schema_version"), lock.get("minimum_macos"), lock.get("architecture")) != (1, "15.0", "arm64"):
        raise ValueError("Expected the reviewed ARM64 macOS 15 input lock")
    rows = lock.get("packages", [])
    if not 1 <= len(rows) <= 100:
        raise ValueError("Invalid media package count")
    names = set()
    for row in rows:
        name, version, digest = row["name"], row["version"], row["sha256"]
        if not NAME.fullmatch(name) or not VERSION.fullmatch(version) or name in names:
            raise ValueError("Invalid/duplicate package identity")
        # URLs are content-addressed, fixed to Homebrew's official registry.
        registry_name = name.replace("@", "/")
        expected = f"https://ghcr.io/v2/homebrew/core/{registry_name}/blobs/sha256:{digest}"
        if not re.fullmatch(r"[a-f0-9]{64}", digest) or row["url"] != expected or row["bottle_tag"] != "arm64_sequoia":
            raise ValueError("Invalid registry URL, checksum or platform")
        names.add(name)
    if "mpv" not in names:
        raise ValueError("Pinned libmpv is missing")
    return lock


def selected_member(member, row):
    path = PurePosixPath(member.name)
    if path.is_absolute() or ".." in path.parts or "\\" in member.name:
        raise ValueError("Unsafe bottle member path")
    prefix = (row["name"], row["version"])
    # Ignore only archive metadata outside the formula; never extract it.
    if path.parts[:2] != prefix:
        if path.parts and path.parts[0] == row["name"] and len(path.parts) > 1:
            raise ValueError("Bottle version differs from lock")
        return False
    relative = path.parts[2:]
    if not relative:
        return False
    library = len(relative) == 2 and relative[0] == "lib" and relative[1].endswith(".dylib")
    notice = len(relative) <= 3 and (LICENSE.match(path.name) or path.name == "sbom.spdx.json")
    recipe = len(relative) == 2 and relative[0] == ".brew" and relative[1].endswith(".rb")
    return bool(library or notice or recipe)


def extract_selected(archive, root, row):
    """Extract only regular dylibs/notices and bounded relative dylib symlinks."""
    base = root / "Cellar" / row["name"] / row["version"]
    seen, links, count, size = set(), [], 0, 0
    with tarfile.open(archive, "r:gz") as bottle:
        for member in bottle:
            count += 1
            if count > 100000:
                raise ValueError("Bottle member count exceeds limit")
            size += member.size
            if member.size<0 or size>MAX_UNPACKED or len(member.name)>4096:
                raise ValueError("Bottle unpacked size/name exceeds limit")
            if not selected_member(member, row):
                continue
            if member.name in seen or member.size < 0:
                raise ValueError("Duplicate/invalid bottle member")
            seen.add(member.name)
            target = root / "Cellar" / member.name
            if member.issym():
                relative = PurePosixPath(member.linkname)
                resolved = target.parent.joinpath(*relative.parts).resolve()
                if (not target.name.endswith(".dylib") or relative.is_absolute()
                        or "\\" in member.linkname or base.resolve() not in resolved.parents):
                    raise ValueError("Unsafe bottle symlink")
                links.append((target, member.linkname))
                continue
            if not member.isfile():
                raise ValueError("Special/hardlink bottle member refused")
            target.parent.mkdir(parents=True, exist_ok=True)
            source = bottle.extractfile(member)
            if source is None:
                raise ValueError("Bottle member has no content")
            with source, target.open("xb") as output:
                shutil.copyfileobj(source, output, 1024 * 1024)
            target.chmod(0o644)
        # All files were created before links: a tar entry cannot redirect writes.
        for target, link in links:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.symlink_to(link)
        for target, _ in links:
            resolved = target.resolve(strict=True)
            if (resolved.parent != (base / "lib").resolve()
                    or not resolved.name.endswith(".dylib") or not resolved.is_file()):
                raise ValueError("Dangling/escaping bottle library link")
    if not list((base / "lib").glob("*.dylib")):
        raise ValueError(f"No dynamic libraries extracted for {row['name']}")


def download(row, cache):
    target = cache / (row["sha256"] + ".tar.gz")
    if target.exists():
        if target.is_symlink() or not target.is_file() or sha256(target) != row["sha256"]:
            raise ValueError("Existing bottle cache is not the pinned regular file")
        return target
    # QQ== is the registry's anonymous public-read marker, not a user credential.
    part = cache / (row["sha256"] + ".partial")
    subprocess.run(["/usr/bin/curl", "--disable", "--fail", "--silent", "--show-error", "--location",
                    "--proto", "=https", "--proto-redir", "=https", "--max-time", "180",
                    "--retry", "3", "--retry-connrefused", "--retry-delay", "1",
                    "--max-filesize", str(MAX_ARCHIVE), "--header", "Authorization: Bearer QQ==",
                    "--output", str(part), row["url"]], check=True)
    if part.stat().st_size > MAX_ARCHIVE or sha256(part) != row["sha256"]:
        raise ValueError(f"Bottle checksum mismatch: {row['name']}")
    part.rename(target)
    return target


def inventory(root):
    result = {}
    for path in sorted((root / "Cellar").rglob("*")):
        name = path.relative_to(root).as_posix()
        if path.is_symlink():
            result[name] = {"link": os.readlink(path)}
        elif path.is_file():
            result[name] = {"sha256": sha256(path)}
        elif not path.is_dir():
            raise ValueError("Unexpected input file type")
    return result


def verify(root, lock_path):
    lock = read_lock(lock_path)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("Expected an isolated regular media input directory")
    receipt = json.loads((root / "receipt.json").read_text(encoding="utf-8"))
    if receipt.get("lock_sha256") != sha256(lock_path) or inventory(root) != receipt.get("files"):
        raise ValueError("Media inputs changed since verified extraction")
    if json.loads((root / "media-lock.json").read_text(encoding="utf-8")) != lock:
        raise ValueError("Isolated license/provenance metadata differs from lock")
    for row in lock["packages"]:
        link = root / "opt" / row["name"]
        if not link.is_symlink() or os.readlink(link) != f"../Cellar/{row['name']}/{row['version']}":
            raise ValueError("Isolated formula alias changed")
    return root / "opt/mpv/lib/libmpv.dylib"


def prepare(root, lock_path):
    lock = read_lock(lock_path)
    if not root.is_absolute() or root.exists() or root.is_symlink():
        raise ValueError("Use a new absolute media input destination")
    root.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".chillcenter-media-", dir=root.parent) as temporary:
        stage = Path(temporary)
        cache, content = stage / "cache", stage / "content"
        cache.mkdir()
        content.mkdir()
        with ThreadPoolExecutor(max_workers=4) as pool:
            archives = list(pool.map(lambda row: download(row, cache), lock["packages"]))
        for row, archive in zip(lock["packages"], archives):
            extract_selected(archive, content, row)
        (content / "opt").mkdir()
        for row in lock["packages"]:
            (content / "opt" / row["name"]).symlink_to(f"../Cellar/{row['name']}/{row['version']}")
        (content / "media-lock.json").write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")
        (content / "receipt.json").write_text(json.dumps({"lock_sha256": sha256(lock_path), "files": inventory(content)}, indent=2) + "\n", encoding="utf-8")
        verify(content, lock_path)
        # Exclusive directory publication: never merge into existing input trees.
        from publish_macos_bundle import publish_directory
        publish_directory(content, root)
    return verify(root, lock_path)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--lock", type=Path, default=Path(__file__).resolve().parents[2] / "deploy/macos/media-sequoia.lock.json")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    try:
        library = verify(args.destination, args.lock) if args.verify_only else prepare(args.destination, args.lock)
        print(f"Verified pinned Sequoia libmpv input: {library}")
    except (OSError, ValueError, RuntimeError, tarfile.TarError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Media inputs not published: {error}\n")
