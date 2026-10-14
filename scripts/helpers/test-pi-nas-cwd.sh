#!/usr/bin/env bash
# BUG-0042: sudo must not preserve an inaccessible caller cwd for NAS probing.
set -euo pipefail

script_dir="$(cd "$(dirname "$0")/.." && pwd)"
installer="$script_dir/install-pi-first-install.sh"
completion="$script_dir/finish-pi-r7-account.sh"

bash -n "$installer" "$completion"
rg -Uq 'cd /\nsudo -n -u chillcenter find "\$nas_root"' "$installer"
rg -Uq 'cd /\nsudo -n -u chillcenter find /srv/volumes/video' "$completion"
echo 'BUG-0042 NAS service-user cwd regressions: PASS'
