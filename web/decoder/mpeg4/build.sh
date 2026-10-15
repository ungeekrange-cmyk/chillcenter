#!/usr/bin/env bash
# Builds the minimal MPEG-4 Part 2 decoder to WebAssembly (ADR-0014). Runs INSIDE the pinned
# emscripten/emsdk container (see build-in-docker.sh); never needs the network.
#
#   FFMPEG_TARBALL   path to ffmpeg-9.0.2.tar.xz (mounted read-only)
#   FFMPEG_SHA256    expected SHA-256 of that tarball
#   OUT_DIR          where mpeg4-decoder.js / .wasm / BUILD-INFO.json are written
set -euo pipefail
: "${FFMPEG_TARBALL:?}" "${FFMPEG_SHA256:?}" "${OUT_DIR:?}"
HERE="$(cd "$(dirname "$0")" && pwd)"

echo "$FFMPEG_SHA256  $FFMPEG_TARBALL" | sha256sum -c -
WORK="$(mktemp -d)"
tar -C "$WORK" -xf "$FFMPEG_TARBALL"
SRC="$(echo "$WORK"/ffmpeg-*)"
cd "$SRC"

# LGPL only: no --enable-gpl, no --enable-nonfree, no external libraries, no asm, no threads.
CONFIGURE_LINE=(--prefix="$WORK/install" --target-os=none --arch=x86_32 --enable-cross-compile
  --cc=emcc --cxx=em++ --objcc=emcc --dep-cc=emcc --ar=emar --ranlib=emranlib --nm=llvm-nm
  --disable-all --disable-autodetect --disable-programs --disable-doc --disable-debug --disable-stripping
  --disable-asm --disable-x86asm --disable-inline-asm --disable-runtime-cpudetect --disable-network
  --disable-pthreads --disable-w32threads --disable-os2threads
  --enable-avcodec --enable-avutil --enable-decoder=mpeg4
  --extra-cflags="-O3 -fno-exceptions")
emconfigure ./configure "${CONFIGURE_LINE[@]}"
emmake make -j"$(nproc)" libavcodec/libavcodec.a libavutil/libavutil.a

mkdir -p "$OUT_DIR"
EXPORTS='["_cc_open","_cc_alloc","_cc_free","_cc_send","_cc_flush_send","_cc_receive","_cc_reset","_cc_close","_cc_width","_cc_height","_cc_format","_cc_pts","_cc_plane","_cc_stride"]'
emcc "$HERE/wrapper.c" -I. -Llibavcodec -lavcodec -Llibavutil -lavutil -O3 \
  -s WASM=1 -s MODULARIZE=1 -s EXPORT_NAME=createMpeg4Decoder -s ENVIRONMENT=worker,web,node \
  -s ALLOW_MEMORY_GROWTH=1 -s INITIAL_MEMORY=33554432 -s MAXIMUM_MEMORY=268435456 -s NO_FILESYSTEM=1 \
  -s EXPORTED_FUNCTIONS="$EXPORTS" -s EXPORTED_RUNTIME_METHODS='["HEAPU8"]' -s STACK_SIZE=262144 \
  -o "$OUT_DIR/mpeg4-decoder.js"

# Provenance, to ship next to the binary (LGPL: exact source, exact configure line).
WASM_SHA="$(sha256sum "$OUT_DIR/mpeg4-decoder.wasm" | cut -d' ' -f1)"
JS_SHA="$(sha256sum "$OUT_DIR/mpeg4-decoder.js" | cut -d' ' -f1)"
cat > "$OUT_DIR/BUILD-INFO.json" <<EOF
{
  "ffmpeg": "$(basename "$SRC")",
  "ffmpeg_tarball_sha256": "$FFMPEG_SHA256",
  "emscripten": "$(emcc --version | head -1)",
  "license": "LGPL-2.1-or-later (FFmpeg libavcodec/libavutil, built without --enable-gpl or --enable-nonfree)",
  "configure": "./configure ${CONFIGURE_LINE[*]}",
  "mpeg4_decoder_wasm_sha256": "$WASM_SHA",
  "mpeg4_decoder_js_sha256": "$JS_SHA",
  "wasm_bytes": $(stat -c %s "$OUT_DIR/mpeg4-decoder.wasm"),
  "js_bytes": $(stat -c %s "$OUT_DIR/mpeg4-decoder.js")
}
EOF
echo "built: $(stat -c %s "$OUT_DIR/mpeg4-decoder.wasm") bytes wasm"
