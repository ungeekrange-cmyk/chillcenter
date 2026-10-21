#!/usr/bin/env python3
"""Copy and relocate libmpv's complete Homebrew Mach-O dependency closure."""

from __future__ import annotations

import argparse
import ctypes
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


SYSTEM_PREFIXES = ("/System/Library/", "/usr/lib/")
FORBIDDEN_PREFIXES = ("/opt/homebrew/", "/usr/local/", "/opt/local/")
LICENSE_NAMES = re.compile(r"^(license|copying|copyright|notice)(\.|$)", re.IGNORECASE)


def run(*args: str) -> str:
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT)


def call_quiet(*args: str) -> None:
    result = subprocess.run(args, text=True, capture_output=True)
    if result.returncode != 0:
        raise subprocess.CalledProcessError(
            result.returncode, args, output=result.stdout, stderr=result.stderr
        )


def dependencies(path: Path) -> list[str]:
    lines = run("/usr/bin/otool", "-L", str(path)).splitlines()[1:]
    return [line.strip().split(" (", 1)[0] for line in lines if line.strip()]


def rpaths(path: Path) -> list[str]:
    lines = run("/usr/bin/otool", "-l", str(path)).splitlines()
    result: list[str] = []
    for index, line in enumerate(lines):
        if line.strip() == "cmd LC_RPATH":
            for candidate in lines[index + 1 : index + 5]:
                match = re.match(r"\s*path (.+) \(offset \d+\)", candidate)
                if match:
                    result.append(match.group(1))
                    break
    return result


def is_system(path: str) -> bool:
    return path.startswith(SYSTEM_PREFIXES)


def formula_identity(source: Path) -> tuple[str, str, Path]:
    resolved = source.resolve()
    parts = resolved.parts
    try:
        index = parts.index("Cellar")
        name, version = parts[index + 1], parts[index + 2]
    except (ValueError, IndexError) as error:
        raise RuntimeError(f"dependency is not from a versioned Homebrew Cellar: {resolved}") from error
    root = Path(*parts[: index + 3])
    return name, version, root


def safe_output_name(source: Path, root: Path, assigned: dict[Path, str]) -> str:
    if source == root:
        return "libmpv.dylib"
    candidate = source.name
    if candidate == "libmpv.dylib":
        candidate = "libmpv-transitive.dylib"
    for existing, name in assigned.items():
        if name == candidate and existing != source:
            raise RuntimeError(f"two different libraries share bundle name {candidate}")
    return candidate


def resolve_media_dependency(load_path: str, media_root: Path | None = None, source: Path | None = None) -> Path:
    if load_path.startswith(("@rpath/", "@loader_path/")) and media_root is not None and source is not None:
        # Relocatable bottles (e.g. Brotli) use peer dylib names. Resolve only a
        # flat sibling in the same pinned formula, never system/installed rpaths.
        name = load_path.split("/", 1)[1]
        if name != Path(name).name or name in {".", ".."}:
            raise RuntimeError("Non-peer relative bottle dependency refused")
        result = (source.parent / name).resolve(strict=True)
        if media_root.resolve() not in result.parents or not result.is_file():
            raise RuntimeError("Relative media dependency escaped isolated root")
        return result
    for prefix, relative in (("/opt/homebrew/", ""), ("@@HOMEBREW_PREFIX@@/", ""),
                             ("@@HOMEBREW_CELLAR@@/", "Cellar/")):
        if load_path.startswith(prefix):
            suffix = load_path[len(prefix):]
            if ".." in Path(suffix).parts:
                raise RuntimeError("Media dependency traversal refused")
            if media_root is None:
                if prefix != "/opt/homebrew/":
                    raise RuntimeError("Unrelocated bottle needs an isolated media root")
                return Path(load_path).resolve(strict=True)
            root = media_root.resolve(strict=True)
            result = (root / relative / suffix).resolve(strict=True)
            if root not in result.parents or not result.is_file():
                raise RuntimeError("Media dependency escaped isolated root")
            return result
    raise RuntimeError(f"unsupported external dependency: {load_path}")


def copy_closure(libmpv: Path, frameworks: Path, media_root: Path | None = None) -> tuple[dict[Path, str], dict[str, dict[str, object]]]:
    root = libmpv.resolve(strict=True)
    assigned: dict[Path, str] = {root: "libmpv.dylib"}
    original_dependencies: dict[Path, list[str]] = {}
    queue = [root]
    while queue:
        source = queue.pop(0)
        deps = dependencies(source)
        original_dependencies[source] = deps
        for load_path in deps:
            if load_path == f"@rpath/{source.name}":
                continue  # The bottle's own LC_ID_DYLIB, rewritten below.
            if is_system(load_path):
                continue
            dependency = resolve_media_dependency(load_path, media_root, source)
            if dependency not in assigned:
                assigned[dependency] = safe_output_name(dependency, root, assigned)
                queue.append(dependency)

    frameworks.mkdir(mode=0o755, parents=True, exist_ok=False)
    formulas: dict[str, dict[str, object]] = {}
    for source, output_name in sorted(assigned.items(), key=lambda item: item[1]):
        architecture = run("/usr/bin/lipo", "-archs", str(source)).strip().split()
        if architecture != ["arm64"]:
            raise RuntimeError(f"library is not arm64-only: {source}: {' '.join(architecture)}")
        destination = frameworks / output_name
        shutil.copy2(source, destination, follow_symlinks=True)
        destination.chmod(0o755)
        formula, version, cellar = formula_identity(source)
        record = formulas.setdefault(formula, {"version": version, "cellar": cellar, "dylibs": []})
        if record["version"] != version:
            raise RuntimeError(f"multiple installed versions used for {formula}")
        record["dylibs"].append(output_name)  # type: ignore[union-attr]

    for source, output_name in assigned.items():
        destination = frameworks / output_name
        call_quiet("/usr/bin/install_name_tool", "-id", f"@rpath/{output_name}", str(destination))
        for old in original_dependencies[source]:
            if not is_system(old):
                dependency = resolve_media_dependency(old, media_root, source)
                if dependency == source:
                    continue  # LC_ID_DYLIB was already replaced by -id.
                call_quiet(
                    "/usr/bin/install_name_tool", "-change", old, f"@loader_path/{assigned[dependency]}", str(destination)
                )
        for old_rpath in rpaths(destination):
            if old_rpath.startswith((*FORBIDDEN_PREFIXES, "@@HOMEBREW_PREFIX@@/", "@@HOMEBREW_CELLAR@@/")):
                call_quiet("/usr/bin/install_name_tool", "-delete_rpath", old_rpath, str(destination))

    verify_relocated(frameworks)
    return assigned, formulas


def verify_relocated(frameworks: Path) -> None:
    names = {path.name for path in frameworks.glob("*.dylib")}
    if "libmpv.dylib" not in names:
        raise RuntimeError("relocated libmpv.dylib is missing")
    for library in sorted(frameworks.glob("*.dylib")):
        if run("/usr/bin/lipo", "-archs", str(library)).strip() != "arm64":
            raise RuntimeError(f"bundled library is not arm64-only: {library.name}")
        for load_path in dependencies(library):
            if load_path.startswith(FORBIDDEN_PREFIXES):
                raise RuntimeError(f"external package-manager link remains: {library.name}: {load_path}")
            if load_path.startswith("@loader_path/") and Path(load_path).name not in names:
                raise RuntimeError(f"bundled dependency is missing: {library.name}: {load_path}")
            if not (
                is_system(load_path)
                or load_path.startswith("@loader_path/")
                or load_path == f"@rpath/{library.name}"
            ):
                raise RuntimeError(f"unexpected bundled load path: {library.name}: {load_path}")
        for value in rpaths(library):
            if value.startswith(FORBIDDEN_PREFIXES):
                raise RuntimeError(f"external package-manager rpath remains: {library.name}: {value}")


def brew_metadata(brew: Path, formulas: list[str]) -> dict[str, dict[str, object]]:
    environment = os.environ.copy()
    environment["HOMEBREW_NO_AUTO_UPDATE"] = "1"
    data = subprocess.check_output(
        [str(brew), "info", "--json=v2", *formulas], text=True, env=environment, stderr=subprocess.STDOUT
    )
    parsed = json.loads(data)
    return {item["name"]: item for item in parsed["formulae"]}


def copy_licenses(licenses: Path, formulas: dict[str, dict[str, object]], metadata: dict[str, dict[str, object]]) -> None:
    licenses.mkdir(mode=0o755, parents=True, exist_ok=False)
    notices = [
        "# Bundled native components",
        "",
        "This inventory describes the exact Homebrew Cellar files copied into this development bundle.",
        "Homebrew is a build/source provenance input only; the packaged Mach-O files do not link to Homebrew paths.",
        "License identifiers are Homebrew formula metadata, not a new legal interpretation by Chill Center.",
        "Upstream source URLs identify the corresponding released source; no notarization or paid signing is claimed.",
        "",
    ]
    for name in sorted(formulas):
        record = formulas[name]
        item = metadata.get(name, {})
        stable = item.get("urls", {}).get("stable", {}) if isinstance(item.get("urls"), dict) else {}
        source_url = stable.get("url", "UNAVAILABLE") if isinstance(stable, dict) else "UNAVAILABLE"
        license_id = item.get("license") or "UNSPECIFIED_BY_FORMULA"
        homepage = item.get("homepage") or "UNAVAILABLE"
        notices.extend(
            [
                f"## {name} {record['version']}",
                "",
                f"- Formula license metadata: `{license_id}`",
                f"- Homepage: {homepage}",
                f"- Released source: {source_url}",
                f"- Bundled libraries: {', '.join(sorted(record['dylibs']))}",
                "",
            ]
        )
        cellar = Path(record["cellar"])
        destination = licenses / name
        destination.mkdir(mode=0o755)
        copied = 0
        for current, directories, files in os.walk(cellar):
            relative_depth = len(Path(current).relative_to(cellar).parts)
            if relative_depth >= 3:
                directories[:] = []
                continue
            directories[:] = [directory for directory in directories if directory not in {"bin", "include", "lib"}]
            for filename in sorted(files):
                if LICENSE_NAMES.match(filename) or filename == "sbom.spdx.json":
                    source = Path(current) / filename
                    relative = source.relative_to(cellar)
                    flattened = "__".join(relative.parts)
                    shutil.copy2(source, destination / flattened)
                    copied += 1
        formula_files = sorted((cellar / ".brew").glob("*.rb"))
        for formula_file in formula_files:
            shutil.copy2(formula_file, destination / f"homebrew-formula__{formula_file.name}")
        if copied == 0:
            (destination / "NO-LICENSE-FILE-IN-BOTTLE.txt").write_text(
                "No matching license/copying/copyright/notice file was installed in this Homebrew Cellar.\n"
                "Consult the formula metadata and upstream source URL in THIRD-PARTY-NOTICES.md.\n",
                encoding="utf-8",
            )
    (licenses / "THIRD-PARTY-NOTICES.md").write_text("\n".join(notices), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--libmpv", type=Path)
    parser.add_argument("--frameworks", type=Path, required=True)
    parser.add_argument("--licenses", type=Path)
    parser.add_argument("--brew", type=Path)
    parser.add_argument("--media-root", type=Path)
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--load-test", action="store_true")
    arguments = parser.parse_args()
    if arguments.verify_only:
        verify_relocated(arguments.frameworks)
        if arguments.load_test:
            library = ctypes.CDLL(
                str(arguments.frameworks / "libmpv.dylib"), mode=os.RTLD_NOW
            )
            library.mpv_client_api_version.restype = ctypes.c_ulong
            version = int(library.mpv_client_api_version())
            if version >> 16 < 2:
                raise RuntimeError(f"bundled libmpv API is too old: {version >> 16}.{version & 0xFFFF}")
            print(f"Loaded bundled libmpv client API {version >> 16}.{version & 0xFFFF}.")
        print(f"Verified {len(list(arguments.frameworks.glob('*.dylib')))} relocated arm64 dylibs.")
        return 0
    if arguments.load_test:
        parser.error("--load-test requires --verify-only")
    if arguments.libmpv is None or arguments.licenses is None or (arguments.brew is None and arguments.media_root is None):
        parser.error("--libmpv, --licenses and either --brew or --media-root are required")
    _, formulas = copy_closure(arguments.libmpv, arguments.frameworks, arguments.media_root)
    if arguments.media_root:
        rows = json.loads((arguments.media_root / "media-lock.json").read_text(encoding="utf-8"))["packages"]
        metadata = {item["name"]: item for item in rows}
        if any(name not in metadata or record["version"] != metadata[name]["version"] for name, record in formulas.items()):
            raise RuntimeError("Media formula version differs from isolated provenance")
        shutil.copy2(arguments.media_root / "media-lock.json", arguments.licenses.parent / "media-inputs.lock.json")
    else:
        metadata = brew_metadata(arguments.brew, sorted(formulas))
    copy_licenses(arguments.licenses, formulas, metadata)
    print(f"Bundled {sum(len(record['dylibs']) for record in formulas.values())} arm64 dylibs from {len(formulas)} formulas.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"macOS bundle helper: {error}", file=sys.stderr)
        raise SystemExit(1)
