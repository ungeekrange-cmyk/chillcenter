#!/usr/bin/env bash
# Regenerates the small audio fixtures of web/test/fixtures (ADR-0020). Needs ffmpeg (test tooling only, never a runtime dependency).
# One distinct frequency sweep (chirp) per channel: unlike pure tones, a sweep has a single alignment peak, so a timing error or a
# mix-down mistake shows in the numbers.
set -euo pipefail
cd "$(dirname "$0")/../web/test/fixtures"
chirp() { echo "aevalsrc=exprs='0.35*sin(2*PI*($1*t+$2*t*t))':d=2:s=$3"; }   # f0 Hz, sweep Hz/s, rate
FILTER="$(chirp 150 150 48000)[a];$(chirp 210 180 48000)[b];$(chirp 270 210 48000)[c];$(chirp 50 20 48000)[d];$(chirp 330 240 48000)[e];$(chirp 390 270 48000)[f];[a][b][c][d][e][f]amerge=inputs=6"
ffmpeg -v error -y -filter_complex "$FILTER" -c:a ac3  -b:a 256k -ar 48000 -f matroska ac3-51.mkv
ffmpeg -v error -y -filter_complex "$FILTER" -c:a eac3 -b:a 256k -ar 48000 -f matroska eac3-51.mkv
STEREO48="$(chirp 200 220 48000)[l];$(chirp 260 260 48000)[r];[l][r]amerge=inputs=2"
STEREO44="$(chirp 200 220 44100)[l];$(chirp 260 260 44100)[r];[l][r]amerge=inputs=2"
# The experimental native DTS encoder only handles stereo; 5.1 decoding is covered by the AC-3/E-AC-3 mix-down tests.
ffmpeg -v error -y -filter_complex "$STEREO48" -c:a dca -strict -2 -b:a 384k -f matroska dts-stereo.mkv
ffmpeg -v error -y -filter_complex "$STEREO44" -c:a ac3 -b:a 128k -f matroska ac3-stereo-441.mkv
ls -l *.mkv
