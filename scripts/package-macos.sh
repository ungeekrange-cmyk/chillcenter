#!/bin/bash
# Standalone Apple Silicon development bundle. No notarization or paid signing.
# Runtime target: macOS 15.0+ (15/26/27), Apple Silicon. Published v0.7.2 still
# requires macOS 27.0 despite its Rust 13.0 floor; never forge Mach-O minima.
# Build inputs: Apple Silicon macOS, pinned Rust, Xcode command-line tools,
# Python 3 and pinned official Sequoia bottles. End users need none of those tools.
# See README.en.md#installation-prerequisites for Mac and Linux server hosts.
set -euo pipefail
project_dir="$(cd "$(dirname "$0")/.." && pwd)"
bundle_dir="${1:-$project_dir/artifacts/macos/Chill Center.app}"
case "$bundle_dir" in
  /*.app) ;;
  *) echo "Use an absolute .app destination." >&2; exit 1 ;;
esac
if [ -e "$bundle_dir" ] || [ -L "$bundle_dir" ]; then
  echo "Destination exists; choose a new path. No bundle was overwritten." >&2
  exit 1
fi
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
  echo "This development bundle targets Apple Silicon macOS." >&2; exit 1
fi
for tool in cargo /usr/bin/codesign /usr/bin/install_name_tool /usr/bin/lipo /usr/bin/otool /usr/bin/plutil /usr/bin/python3 /usr/bin/sips /usr/bin/vtool; do
  command -v "$tool" >/dev/null || { echo "Required tool missing: $tool" >&2; exit 1; }
done
media_root="${CHILLCENTER_MEDIA_ROOT:-$project_dir/target/macos-media-sequoia}"
if [ -e "$media_root" ] || [ -L "$media_root" ]; then
  /usr/bin/python3 "$project_dir/scripts/helpers/macos_media_inputs.py" "$media_root" --verify-only
else
  /usr/bin/python3 "$project_dir/scripts/helpers/macos_media_inputs.py" "$media_root"
fi
libmpv="$media_root/opt/mpv/lib/libmpv.dylib"
[ -f "$libmpv" ] || { echo "libmpv packaging input not found." >&2; exit 1; }

bundle_parent="$(dirname "$bundle_dir")"
mkdir -p "$bundle_parent"
stage="$bundle_parent/.$(basename "$bundle_dir").staging.$(/usr/bin/uuidgen)"
owned_target=""
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  target_dir="$CARGO_TARGET_DIR"
else
  target_dir="$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/chillcenter-macos-target.XXXXXX")"
  owned_target="$target_dir"
fi
cleanup() {
  [ ! -e "$stage" ] || /bin/rm -rf "$stage"
  [ -z "$owned_target" ] || /bin/rm -rf "$owned_target"
}
trap cleanup EXIT

MACOSX_DEPLOYMENT_TARGET=15.0 CARGO_TARGET_DIR="$target_dir" cargo build --manifest-path "$project_dir/Cargo.toml" --release --locked -p chillcenter-desktop
executable="$target_dir/release/chillcenter-desktop"
[ "$(/usr/bin/lipo -archs "$executable")" = "arm64" ] || { echo "Desktop executable is not arm64-only." >&2; exit 1; }

mkdir -p "$stage/Contents/MacOS" "$stage/Contents/Resources"
cp "$project_dir/apps/desktop/Info.plist" "$stage/Contents/Info.plist"
cp "$executable" "$stage/Contents/MacOS/chillcenter-desktop"
cp "$project_dir/icons/branding/vector/default.svg" "$stage/Contents/Resources/approved-default.svg"

icon_source="$project_dir/icons/branding/vector/default.png"
/usr/bin/python3 "$project_dir/scripts/helpers/make_icns.py" \
  "$icon_source" "$stage/Contents/Resources/AppIcon.icns"

/usr/bin/python3 "$project_dir/scripts/helpers/macos_bundle.py" \
  --libmpv "$libmpv" \
  --frameworks "$stage/Contents/Frameworks" \
  --licenses "$stage/Contents/Resources/licenses" \
  --media-root "$media_root"

# Homebrew's media libraries can require a newer OS than the Rust executable.
# Derive the distributable's real minimum instead of advertising a false floor.
/usr/bin/python3 "$project_dir/scripts/helpers/macos_minimum_version.py" "$stage" --maximum 15.0
/usr/bin/plutil -lint "$stage/Contents/Info.plist"
for dylib in "$stage"/Contents/Frameworks/*.dylib; do
  /usr/bin/codesign --force --sign - --timestamp=none "$dylib"
done
/usr/bin/codesign --force --sign - --timestamp=none "$stage/Contents/MacOS/chillcenter-desktop"
/usr/bin/codesign --force --sign - --timestamp=none "$stage"
/usr/bin/codesign --verify --deep --strict --verbose=2 "$stage"

# Verify the signed bytes, not merely the pre-sign relocation pass.
/usr/bin/python3 "$project_dir/scripts/helpers/macos_bundle.py" --verify-only --load-test \
  --frameworks "$stage/Contents/Frameworks"
if /usr/bin/otool -L "$stage/Contents/MacOS/chillcenter-desktop" "$stage"/Contents/Frameworks/*.dylib | /usr/bin/grep -E '/opt/homebrew/|/usr/local/|/opt/local/' >/dev/null; then
  echo "External package-manager dependency remains in the signed bundle." >&2
  exit 1
fi

/usr/bin/python3 "$project_dir/scripts/helpers/publish_macos_bundle.py" "$stage" "$bundle_dir"
echo "Standalone ad-hoc Apple Silicon bundle: $bundle_dir"
