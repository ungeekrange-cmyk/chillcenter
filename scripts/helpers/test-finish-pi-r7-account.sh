#!/usr/bin/env bash
# Static regression checks for the r7 partial-state completion runbook.
set -euo pipefail

script_dir="$(cd "$(dirname "$0")/.." && pwd)"
script="$script_dir/finish-pi-r7-account.sh"
bash -n "$script"
bash "$script" --help >/dev/null
! rg -q '^[[:space:]]*(sudo[[:space:]]+[^[:space:]]+[[:space:]]+)?useradd([[:space:]]|$)' "$script"
rg -q '^cd /$' "$script"
rg -q 'chillcenter:x:997:983::/home/chillcenter:/usr/sbin/nologin' "$script"
rg -Fq 'id -G chillcenter' "$script"
rg -q 'cmp "\$root/usr/lib/systemd/system/chillcenter.service"' "$script"
rg -Fq 'tarfile.open(archive_path, "r:gz")' "$script"
rg -Fq 'len(matching_members) != 1 or not matching_members[0].isreg()' "$script"
rg -Fq 'staged file differs from immutable archive' "$script"
for archive_member in \
  'usr/lib/chillcenter/chill-server' \
  'usr/lib/systemd/system/chillcenter.service' \
  'etc/chillcenter/server.json.example' \
  'usr/share/doc/chillcenter/LINUX-PACKAGING.md' \
  'usr/share/doc/chillcenter/build-manifest.json'; do
  rg -Fq "$archive_member" "$script"
done
echo 'R7 completion runbook static regressions: PASS'
