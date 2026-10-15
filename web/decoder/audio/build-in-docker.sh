#!/usr/bin/env bash
# Reproducible build of web/vendor/audio from pinned inputs (ADR-0020). No network inside the container.
#   usage: web/decoder/audio/build-in-docker.sh /path/to/ffmpeg-9.0.2.tar.xz
# The tarball is copied into .build-cache/ (git-ignored) because container VMs such as Colima only share
# the home directory; its SHA-256 is checked again inside the container by build.sh.
set -euo pipefail
TARBALL="${1:?chemin de ffmpeg-9.0.2.tar.xz}"
FFMPEG_SHA256="8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e"   # = sha256 pinned by Homebrew for 9.0.2
IMAGE="emscripten/emsdk@sha256:a7883da4a4995eda3b58a3740c5b88728caffd0452df10e7b748099d28661a98"  # emsdk 6.0.12, linux/arm64
REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
CACHE="$REPO/.build-cache"
mkdir -p "$REPO/web/vendor/audio" "$CACHE"
cp "$TARBALL" "$CACHE/ffmpeg.tar.xz"
docker run --rm --platform linux/arm64 --network none \
  -v "$CACHE:/in:ro" -v "$REPO/web/decoder/audio:/src:ro" -v "$REPO/web/vendor/audio:/out" \
  -e FFMPEG_TARBALL=/in/ffmpeg.tar.xz -e FFMPEG_SHA256="$FFMPEG_SHA256" -e OUT_DIR=/out \
  "$IMAGE" bash /src/build.sh
