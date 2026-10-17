#!/usr/bin/env bash
# Builds the audio transcoder (AC-3 / E-AC-3 / DTS / TrueHD / MP3 -> AAC) to WebAssembly (ADR-0020). Runs INSIDE the pinned
# emscripten/emsdk container (see build-in-docker.sh); never needs the network.
#
#   FFMPEG_TARBALL   path to ffmpeg-9.0.2.tar.xz (mounted read-only)
#   FFMPEG_SHA256    expected SHA-256 of that tarball
#   OUT_DIR          where audio-transcoder.js / .wasm / BUILD-INFO.json are written
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
  --enable-avcodec --enable-avutil --enable-swresample --enable-decoder=ac3,eac3,dca,truehd,mlp,mp3 --enable-encoder=aac
  --extra-cflags="-O3 -fno-exceptions")
emconfigure ./configure "${CONFIGURE_LINE[@]}"
emmake make -j"$(nproc)" libavcodec/libavcodec.a libavutil/libavutil.a libswresample/libswresample.a

mkdir -p "$OUT_DIR"
EXPORTS='["_cc_a_open","_cc_a_alloc","_cc_a_free","_cc_a_send","_cc_a_flush","_cc_a_reset","_cc_a_next","_cc_a_packet","_cc_a_asc","_cc_a_asc_size","_cc_a_frame_size","_cc_a_initial_padding","_cc_a_src_rate","_cc_a_src_channels","_cc_a_errors","_cc_a_close"]'
emcc "$HERE/wrapper.c" -I. -Llibavcodec -lavcodec -Llibswresample -lswresample -Llibavutil -lavutil -O3 \
  -s WASM=1 -s MODULARIZE=1 -s EXPORT_NAME=createAudioTranscoder -s ENVIRONMENT=worker,web,node \
  -s ALLOW_MEMORY_GROWTH=1 -s INITIAL_MEMORY=16777216 -s MAXIMUM_MEMORY=134217728 -s NO_FILESYSTEM=1 \
  -s EXPORTED_FUNCTIONS="$EXPORTS" -s EXPORTED_RUNTIME_METHODS='["HEAPU8"]' -s STACK_SIZE=262144 \
  -o "$OUT_DIR/audio-transcoder.js"

# Provenance, to ship next to the binary (LGPL: exact source, exact configure line).
WASM_SHA="$(sha256sum "$OUT_DIR/audio-transcoder.wasm" | cut -d' ' -f1)"
JS_SHA="$(sha256sum "$OUT_DIR/audio-transcoder.js" | cut -d' ' -f1)"
cat > "$OUT_DIR/BUILD-INFO.json" <<EOF
{
  "ffmpeg": "$(basename "$SRC")",
  "ffmpeg_tarball_sha256": "$FFMPEG_SHA256",
  "emscripten": "$(emcc --version | head -1)",
  "license": "LGPL-2.1-or-later (FFmpeg libavcodec/libswresample/libavutil, built without --enable-gpl or --enable-nonfree)",
  "configure": "./configure ${CONFIGURE_LINE[*]}",
  "audio_transcoder_wasm_sha256": "$WASM_SHA",
  "audio_transcoder_js_sha256": "$JS_SHA",
  "wasm_bytes": $(stat -c %s "$OUT_DIR/audio-transcoder.wasm"),
  "js_bytes": $(stat -c %s "$OUT_DIR/audio-transcoder.js")
}
EOF
echo "built: $(stat -c %s "$OUT_DIR/audio-transcoder.wasm") bytes wasm"
