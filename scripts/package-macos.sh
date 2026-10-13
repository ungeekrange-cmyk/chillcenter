#!/bin/bash
# Development bundle only: no notarization, installer or final libmpv payload.
set -euo pipefail
project_dir="$(cd "$(dirname "$0")/.." && pwd)"
bundle_dir="${1:-$project_dir/artifacts/macos/Chill Center.app}"
case "$bundle_dir" in
  /*.app) ;;
  *) echo "Use an absolute .app destination." >&2; exit 1 ;;
esac
if [ -e "$bundle_dir" ]; then
  echo "Destination exists; choose a new path. No bundle was overwritten." >&2
  exit 1
fi
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
  echo "This development bundle targets Apple Silicon macOS." >&2; exit 1
fi
cargo build --manifest-path "$project_dir/Cargo.toml" --release --locked -p chillcenter-desktop
target_dir="${CARGO_TARGET_DIR:-$project_dir/target}"
mkdir -p "$bundle_dir/Contents/MacOS"
cp "$project_dir/apps/desktop/Info.plist" "$bundle_dir/Contents/Info.plist"
cp "$target_dir/release/chillcenter-desktop" "$bundle_dir/Contents/MacOS/chillcenter-desktop"
/usr/bin/plutil -lint "$bundle_dir/Contents/Info.plist"
/usr/bin/codesign --sign - "$bundle_dir"
/usr/bin/codesign --verify --strict --verbose=2 "$bundle_dir"
echo "Development bundle: $bundle_dir"
