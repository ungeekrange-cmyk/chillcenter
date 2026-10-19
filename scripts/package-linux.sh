#!/bin/bash
# Build a local, non-installing Linux server archive. It never deploys or starts services.
set -euo pipefail

project_dir="$(cd "$(dirname "$0")/.." && pwd)"
helper="$project_dir/scripts/helpers/linux_package.py"

usage() {
  echo "Usage: scripts/package-linux.sh --target <aarch64-unknown-linux-gnu|armv7-unknown-linux-gnueabihf|host-linux> --output /absolute/package.tar.gz" >&2
}

target=""
output=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --target) [ "$#" -ge 2 ] || { usage; exit 2; }; target="$2"; shift 2 ;;
    --output) [ "$#" -ge 2 ] || { usage; exit 2; }; output="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done
[ -n "$target" ] && [ -n "$output" ] || { usage; exit 2; }
case "$output" in
  /*.tar.gz) ;;
  *) echo "Output must be a new absolute .tar.gz path." >&2; exit 2 ;;
esac
checksum="$output.sha256"
if [ -e "$output" ] || [ -L "$output" ] || [ -e "$checksum" ] || [ -L "$checksum" ]; then
  echo "Output or checksum already exists; nothing was overwritten." >&2
  exit 1
fi

for tool in cargo rustc rustup git python3 tar; do
  command -v "$tool" >/dev/null || { echo "Required build tool missing: $tool" >&2; exit 1; }
done
if [ -n "$(git -C "$project_dir" status --porcelain --untracked-files=normal)" ]; then
  echo "The source tree is not clean; commit the exact packaging source before building." >&2
  exit 1
fi

case "$target" in
  aarch64-unknown-linux-gnu|armv7-unknown-linux-gnueabihf)
    resolved_target="$target"
    ;;
  host-linux)
    if [ "$(uname -s)" != Linux ]; then
      echo "host-linux requires a Linux build host; this host is $(uname -s)." >&2
      exit 1
    fi
    resolved_target="$(rustc -vV | sed -n 's/^host: //p')"
    case "$resolved_target" in
      x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|armv7-unknown-linux-gnueabihf) ;;
      *) echo "Unsupported Linux host target: $resolved_target" >&2; exit 1 ;;
    esac
    ;;
  *) echo "Unsupported target: $target" >&2; usage; exit 2 ;;
esac

if ! rustup target list --installed | grep -Fqx "$resolved_target"; then
  echo "Rust target is not installed: $resolved_target. Install it explicitly; packaging did not modify the toolchain." >&2
  exit 1
fi

output_parent="$(dirname "$output")"
mkdir -p "$output_parent"
stage="$(mktemp -d "$output_parent/.chillcenter-linux-package.XXXXXX")"
owned_target=""
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  target_dir="$CARGO_TARGET_DIR"
else
  target_dir="$(mktemp -d "${TMPDIR:-/tmp}/chillcenter-linux-target.XXXXXX")"
  owned_target="$target_dir"
fi
cleanup() {
  [ ! -d "$stage" ] || rm -rf "$stage"
  [ -z "$owned_target" ] || [ ! -d "$owned_target" ] || rm -rf "$owned_target"
}
trap cleanup EXIT

CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$target_dir" \
  cargo build --manifest-path "$project_dir/Cargo.toml" --release --locked --offline \
  -p chill-server --target "$resolved_target"
binary="$target_dir/$resolved_target/release/chill-server"
[ -f "$binary" ] || { echo "Cargo completed without the expected server binary: $binary" >&2; exit 1; }
python3 "$helper" validate-elf --path "$binary" --target "$resolved_target"

version="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["server"])' "$project_dir/versions.json")"
[ -n "$version" ] || { echo "Workspace version not found." >&2; exit 1; }
source_commit="$(git -C "$project_dir" rev-parse HEAD)"
root="$stage/chillcenter-server-$version-$resolved_target"
install -d "$root/usr/lib/chillcenter" "$root/usr/lib/systemd/system" \
  "$root/etc/chillcenter" "$root/usr/share/doc/chillcenter"
install -m 0755 "$binary" "$root/usr/lib/chillcenter/chill-server"
install -m 0644 "$project_dir/deploy/linux/chillcenter.service" \
  "$root/usr/lib/systemd/system/chillcenter.service"
install -m 0644 "$project_dir/deploy/linux/server.json.example" \
  "$root/etc/chillcenter/server.json.example"
install -m 0644 "$project_dir/deploy/linux/LICENSE-MIT.txt" \
  "$root/usr/share/doc/chillcenter/LICENSE-MIT.txt"
install -m 0644 "$project_dir/docs/implementation/LINUX-PACKAGING.md" \
  "$root/usr/share/doc/chillcenter/LINUX-PACKAGING.md"
install -m 0644 "$project_dir/Cargo.lock" "$root/usr/share/doc/chillcenter/Cargo.lock"
python3 "$helper" manifest --binary "$binary" --target "$resolved_target" \
  --source-commit "$source_commit" --cargo-version "$(cargo --version)" \
  --rustc-version "$(rustc --version)" \
  --output "$root/usr/share/doc/chillcenter/build-manifest.json"

archive_stage="$stage/package.tar.gz"
COPYFILE_DISABLE=1 tar -C "$stage" -czf "$archive_stage" "$(basename "$root")"
archive_hash="$(python3 "$helper" sha256 --path "$archive_stage")"
printf '%s  %s\n' "$archive_hash" "$(basename "$output")" >"$stage/package.sha256"
chmod 0644 "$archive_stage" "$stage/package.sha256"
if ! ln "$stage/package.sha256" "$checksum"; then
  echo "Checksum destination appeared during packaging; nothing was replaced." >&2
  exit 1
fi
if ! ln "$archive_stage" "$output"; then
  # Do not unlink a public name that another process may have replaced meanwhile.
  # Leave the published checksum as evidence of the interrupted operation.
  echo "Archive destination appeared during packaging; nothing was replaced. Checksum publication remains at $checksum; inspect it before choosing a new output." >&2
  exit 1
fi
echo "Local Linux package: $output"
echo "SHA-256: $archive_hash"
echo "No installation, service activation, mount, or deployment was performed."
