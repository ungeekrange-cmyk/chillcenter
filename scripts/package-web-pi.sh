#!/usr/bin/env bash
# Builds the web client and packs it as the archive consumed by scripts/pi-update-web-operator.sh:
# a tar containing exactly web/ (files + SHA256SUMS), nothing else. Runs locally; touches no remote host.
#   usage: scripts/package-web-pi.sh [output-dir]      (default: artifacts/web-pi, git-ignored)
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-artifacts/web-pi}"
rm -rf "$OUT"; mkdir -p "$OUT/stage"
node scripts/build-web.mjs "$OUT/stage/web"
( cd "$OUT/stage/web" && find . -type f ! -name SHA256SUMS -print | sed 's|^\./||' | LC_ALL=C sort | xargs shasum -a 256 > SHA256SUMS )
( cd "$OUT/stage/web" && shasum -a 256 -c --quiet SHA256SUMS )
COPYFILE_DISABLE=1 tar -C "$OUT/stage" -cf "$OUT/chillcenter-web.tar" web
tar -tf "$OUT/chillcenter-web.tar" | grep -v '^web/' && { echo "entrée hors web/" >&2; exit 1; } || true
ARCHIVE_SHA="$(shasum -a 256 "$OUT/chillcenter-web.tar" | cut -d' ' -f1)"
printf 'web_archive=%s\nweb_archive_sha256=%s\nweb_archive_bytes=%s\nfiles=%s\ngit_head=%s\n' \
  "$OUT/chillcenter-web.tar" "$ARCHIVE_SHA" "$(stat -f %z "$OUT/chillcenter-web.tar" 2>/dev/null || stat -c %s "$OUT/chillcenter-web.tar")" \
  "$(find "$OUT/stage/web" -type f | wc -l | tr -d ' ')" "$(git rev-parse HEAD)" | tee "$OUT/PACKAGE-INFO.txt"
