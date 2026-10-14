#!/usr/bin/env bash
# Local BUG-0043 contract checks; no SSH or Pi mutation.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
unit="$repo_root/deploy/linux/chillcenter.service"
dropin="$repo_root/deploy/linux/chillcenter.service.d/10-working-directory.conf"
script="$repo_root/scripts/apply-working-directory.sh"

bash -n "$script"
test "$(cat "$dropin")" = $'[Service]\nWorkingDirectory=/usr/lib/chillcenter'
rg -q '^WorkingDirectory=/usr/lib/chillcenter$' "$unit"
rg -Fq 'WorkingDirectory=/usr/lib/chillcenter' "$script"
rg -Fq 'test ! -e /etc/systemd/system/chillcenter.service.d' "$script"
rg -Fq 'mktemp -d /tmp/chillcenter-working-directory.XXXXXXXX' "$script"
rg -Fq 'test ! -L "$candidate"' "$script"
rg -Fq 'test "$(stat -c '\''%U:%G %a'\'' /etc/chillcenter/server.json)" = '\''root:chillcenter 640'\''' "$script"
rg -Fq 'id -G chillcenter' "$script"
rg -Fq 'sudo -n bash -s -- "$candidate" "$expected_dropin_sha" "$expected_vendor_unit_sha"' "$script"
rg -Fq 'systemctl enable --now chillcenter.service' "$script"

run_nested_argument_seam() {
  bash -s -- "$@" <<'INNER'
set -euo pipefail
candidate=$1
expected_dropin_sha=$2
expected_vendor_unit_sha=$3
test "$candidate" = candidate
test "$expected_dropin_sha" = dropin
test "$expected_vendor_unit_sha" = vendor
INNER
}

if run_nested_argument_seam candidate; then
  echo 'Nested sudo seam accepted missing SHA arguments.' >&2
  exit 1
fi
if run_nested_argument_seam candidate dropin; then
  echo 'Nested sudo seam accepted missing vendor SHA.' >&2
  exit 1
fi
run_nested_argument_seam candidate dropin vendor

python3 - <<'PY'
from pathlib import Path

data_root = Path("/var/lib/chillcenter")
def rejected(workspace: Path) -> bool:
    return data_root == workspace or workspace.is_relative_to(data_root) or data_root.is_relative_to(workspace)

assert rejected(Path("/"))  # RED: historical implicit systemd cwd
assert not rejected(Path("/usr/lib/chillcenter"))  # GREEN: service WorkingDirectory
PY

echo 'BUG-0043 WorkingDirectory contract regressions: PASS'
