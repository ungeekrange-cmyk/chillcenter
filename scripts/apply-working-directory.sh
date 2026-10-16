#!/usr/bin/env bash
# Apply only the BUG-0043 WorkingDirectory drop-in to the documented r7 install.
set -euo pipefail

readonly EXPECTED_BINARY_SHA="9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6"
readonly EXPECTED_HELPER_SHA="d508cfdf964c5d68497f426413226cbca4d70abdca807dbb16cc058bad0521df"
readonly EXPECTED_VENDOR_UNIT_SHA="76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2"
readonly SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

usage() {
  cat >&2 <<'EOF'
Usage: apply-working-directory.sh --dropin /absolute/10-working-directory.conf \
  --sha256 <dropin-sha256> --vendor-unit-sha256 <r7-vendor-unit-sha256> \
  --integration-gate /absolute/G-INTEGRATION-*.md \
  --ssh-control /absolute/private-control-socket --go
EOF
}

dropin=""
expected_dropin_sha=""
expected_vendor_unit_sha=""
integration_gate=""
ssh_control=""
explicit_go=false
while (($#)); do
  case "$1" in
    --dropin) dropin=${2:-}; shift 2 ;;
    --sha256) expected_dropin_sha=${2:-}; shift 2 ;;
    --vendor-unit-sha256) expected_vendor_unit_sha=${2:-}; shift 2 ;;
    --integration-gate) integration_gate=${2:-}; shift 2 ;;
    --ssh-control) ssh_control=${2:-}; shift 2 ;;
    --go) explicit_go=true; shift ;;
    --help|-h) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done

[[ "$explicit_go" == true && -f "$dropin" && -f "$integration_gate" && -n "$ssh_control" ]] || { usage; exit 2; }
[[ "$dropin" = /* && "$integration_gate" = /* && "$ssh_control" = /* ]] || { echo "All paths must be absolute." >&2; exit 2; }
[[ "$expected_dropin_sha" =~ ^[[:xdigit:]]{64}$ && "$expected_vendor_unit_sha" =~ ^[[:xdigit:]]{64}$ ]] || { echo "Invalid SHA-256." >&2; exit 2; }
: "${CHILLCENTER_TARGET_HOST:?set the explicit SSH target user@host}"
[[ "$CHILLCENTER_TARGET_HOST" =~ ^[A-Za-z0-9_][A-Za-z0-9._-]*@[A-Za-z0-9][A-Za-z0-9.-]*$ ]] || { echo "Invalid CHILLCENTER_TARGET_HOST." >&2; exit 2; }
readonly TARGET_HOST="$CHILLCENTER_TARGET_HOST"
actual_dropin_sha=$(shasum -a 256 "$dropin" | awk '{print $1}')
test "$actual_dropin_sha" = "$expected_dropin_sha"
test "$expected_vendor_unit_sha" = "$EXPECTED_VENDOR_UNIT_SHA"
test "$(shasum -a 256 "$SCRIPT_DIR/helpers/validate-ssh-control.py" | awk '{print $1}')" = "$EXPECTED_HELPER_SHA"
test "$(cat "$dropin")" = $'[Service]\nWorkingDirectory=/usr/lib/chillcenter'
grep -Eq 'G-INTEGRATION-[A-Za-z0-9_.-]+' "$integration_gate"
grep -Eq '(^|[^A-Za-z])APPROVED([^A-Za-z]|$)' "$integration_gate"
grep -Fqi "$expected_dropin_sha" "$integration_gate"
grep -Fqi "$expected_vendor_unit_sha" "$integration_gate"
grep -Fqi "$EXPECTED_BINARY_SHA" "$integration_gate"
grep -Fqi "$EXPECTED_HELPER_SHA" "$integration_gate"
python3 "$SCRIPT_DIR/helpers/validate-ssh-control.py" "$ssh_control"

ssh_options=(
  -o BatchMode=yes
  -o StrictHostKeyChecking=yes
  -o ConnectTimeout=8
  -o ServerAliveInterval=5
  -o ServerAliveCountMax=1
  -o ControlMaster=no
  -o "ControlPath=$ssh_control"
)
remote_dir=$(ssh "${ssh_options[@]}" "$TARGET_HOST" 'umask 077; mktemp -d /tmp/chillcenter-working-directory.XXXXXXXX')
[[ "$remote_dir" = /tmp/chillcenter-working-directory.* ]] || { echo "Unexpected remote staging path." >&2; exit 1; }
remote_candidate="$remote_dir/10-working-directory.conf"
scp "${ssh_options[@]}" "$dropin" "$TARGET_HOST:$remote_candidate"

ssh "${ssh_options[@]}" "$TARGET_HOST" "bash -s -- '$remote_candidate' '$actual_dropin_sha' '$expected_vendor_unit_sha'" <<'REMOTE_APPLY'
set -euo pipefail
candidate=$1
expected_dropin_sha=$2
expected_vendor_unit_sha=$3

sudo -n bash -s -- "$candidate" "$expected_dropin_sha" "$expected_vendor_unit_sha" <<'SUDO_APPLY'
set -euo pipefail
candidate=$1
expected_dropin_sha=$2
expected_vendor_unit_sha=$3
candidate_dir=$(dirname "$candidate")
admin_uid=$(id -u admin)
test "$(systemctl is-active chillcenter.service || true)" = inactive
test "$(systemctl is-enabled chillcenter.service || true)" = disabled
test "$(sha256sum /usr/lib/chillcenter/chill-server | awk '{print $1}')" = '9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6'
test "$(sha256sum /usr/lib/systemd/system/chillcenter.service | awk '{print $1}')" = "$expected_vendor_unit_sha"
test "$(getent passwd chillcenter)" = 'chillcenter:x:997:983::/home/chillcenter:/usr/sbin/nologin'
test "$(id -G chillcenter | tr ' ' '\n' | sort | paste -sd,)" = '983,984'
test "$(stat -c '%U:%G %a' /etc/chillcenter/server.json)" = 'root:chillcenter 640'
test "$(stat -c '%U:%G %a' /var/lib/chillcenter)" = 'chillcenter:chillcenter 700'
test ! -e /etc/systemd/system/chillcenter.service.d && test ! -L /etc/systemd/system/chillcenter.service.d
test -f "$candidate" && test ! -L "$candidate"
test "$(stat -c '%u' "$candidate")" = "$admin_uid"
test -d "$candidate_dir" && test ! -L "$candidate_dir"
test "$(stat -c '%u %a' "$candidate_dir")" = "$admin_uid 700"
test "$(sha256sum "$candidate" | awk '{print $1}')" = "$expected_dropin_sha"
install -d -o root -g root -m 0755 /etc/systemd/system/chillcenter.service.d
install -o root -g root -m 0644 "$candidate" /etc/systemd/system/chillcenter.service.d/10-working-directory.conf
cmp "$candidate" /etc/systemd/system/chillcenter.service.d/10-working-directory.conf
systemd-analyze verify chillcenter.service
systemctl daemon-reload
systemctl enable --now chillcenter.service
systemctl is-active --quiet chillcenter.service
SUDO_APPLY
REMOTE_APPLY

echo "WORKING-DIRECTORY-DROPIN-APPLIED: sha256=$actual_dropin_sha target=$TARGET_HOST"
