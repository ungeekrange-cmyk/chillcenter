#!/usr/bin/env bash
# Complete only the documented r7 partial install; never creates the account.
set -euo pipefail

readonly EXPECTED_SOURCE_COMMIT="126da0af167ee62588d92d90297f2dfc51962676"
readonly EXPECTED_BINARY_SHA="9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6"
readonly EXPECTED_HELPER_SHA="d508cfdf964c5d68497f426413226cbca4d70abdca807dbb16cc058bad0521df"
readonly SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

usage() {
  cat >&2 <<'EOF'
Usage: finish-pi-r7-account.sh --archive /absolute/chillcenter-pi64-r7.tar.gz \
  --sha256 <archive-sha256> --integration-gate /absolute/G-INTEGRATION-*.md \
  --ssh-control /absolute/private-control-socket --go

This is a one-time completion for the documented r7 partial state only. It
refuses absent or mismatched account state and never runs useradd.
EOF
}

archive=""
expected_archive_sha=""
integration_gate=""
ssh_control=""
explicit_go=false
while (($#)); do
  case "$1" in
    --archive) archive=${2:-}; shift 2 ;;
    --sha256) expected_archive_sha=${2:-}; shift 2 ;;
    --integration-gate) integration_gate=${2:-}; shift 2 ;;
    --ssh-control) ssh_control=${2:-}; shift 2 ;;
    --go) explicit_go=true; shift ;;
    --help|-h) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done

[[ "$explicit_go" == true && -f "$archive" && -f "$integration_gate" && -n "$ssh_control" ]] || { usage; exit 2; }
[[ "$archive" = /* && "$integration_gate" = /* && "$ssh_control" = /* ]] || { echo "All paths must be absolute." >&2; exit 2; }
[[ "$expected_archive_sha" =~ ^[[:xdigit:]]{64}$ ]] || { echo "Invalid archive SHA-256." >&2; exit 2; }
: "${CHILLCENTER_TARGET_HOST:?set the explicit SSH target user@host}" "${CHILLCENTER_NAS_SOURCE:?set the explicit CIFS NAS source}"
[[ "$CHILLCENTER_TARGET_HOST" =~ ^[A-Za-z0-9_][A-Za-z0-9._-]*@[A-Za-z0-9][A-Za-z0-9.-]*$ ]] || { echo "Invalid CHILLCENTER_TARGET_HOST." >&2; exit 2; }
[[ "$CHILLCENTER_NAS_SOURCE" =~ ^//[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$ ]] || { echo "Invalid CHILLCENTER_NAS_SOURCE." >&2; exit 2; }
readonly TARGET_HOST="$CHILLCENTER_TARGET_HOST"
readonly NAS_SOURCE="$CHILLCENTER_NAS_SOURCE"
actual_archive_sha=$(shasum -a 256 "$archive" | awk '{print $1}')
test "$actual_archive_sha" = "$expected_archive_sha"
test "$(shasum -a 256 "$SCRIPT_DIR/helpers/validate-ssh-control.py" | awk '{print $1}')" = "$EXPECTED_HELPER_SHA"
grep -Eq 'G-INTEGRATION-[A-Za-z0-9_.-]+' "$integration_gate"
grep -Eq '(^|[^A-Za-z])APPROVED([^A-Za-z]|$)' "$integration_gate"
grep -Fq "$EXPECTED_SOURCE_COMMIT" "$integration_gate"
grep -Fqi "$expected_archive_sha" "$integration_gate"
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
remote_stage="/tmp/chillcenter-first-install-$actual_archive_sha"

ssh "${ssh_options[@]}" "$TARGET_HOST" "bash -s -- '$remote_stage' '$actual_archive_sha' '$NAS_SOURCE'" <<'REMOTE_FINISH'
set -euo pipefail
stage=$1
expected_archive_sha=$2
nas_source=$3
test "$(sha256sum "$stage/archive.tar.gz" | awk '{print $1}')" = "$expected_archive_sha"
root="$stage"/unpacked/chillcenter-server-0.1.0-aarch64-unknown-linux-gnu
test -d "$root"
test -f "$root/usr/lib/chillcenter/chill-server"
test -f "$root/usr/lib/systemd/system/chillcenter.service"
test -f "$root/etc/chillcenter/server.json.example"
python3 - "$stage/archive.tar.gz" "$root" <<'PY'
import hashlib
import sys
import tarfile

archive_path, staged_root = sys.argv[1:]
package_root = "chillcenter-server-0.1.0-aarch64-unknown-linux-gnu"
used_files = {
    f"{package_root}/usr/lib/chillcenter/chill-server": f"{staged_root}/usr/lib/chillcenter/chill-server",
    f"{package_root}/usr/lib/systemd/system/chillcenter.service": f"{staged_root}/usr/lib/systemd/system/chillcenter.service",
    f"{package_root}/etc/chillcenter/server.json.example": f"{staged_root}/etc/chillcenter/server.json.example",
    f"{package_root}/usr/share/doc/chillcenter/LINUX-PACKAGING.md": f"{staged_root}/usr/share/doc/chillcenter/LINUX-PACKAGING.md",
    f"{package_root}/usr/share/doc/chillcenter/build-manifest.json": f"{staged_root}/usr/share/doc/chillcenter/build-manifest.json",
}

with tarfile.open(archive_path, "r:gz") as archive:
    archive_members = archive.getmembers()
    for member_name, staged_path in used_files.items():
        matching_members = [member for member in archive_members if member.name == member_name]
        if len(matching_members) != 1 or not matching_members[0].isreg():
            raise SystemExit(f"invalid immutable archive member: {member_name}")
        archive_file = archive.extractfile(matching_members[0])
        if archive_file is None:
            raise SystemExit(f"unreadable immutable archive member: {member_name}")
        archive_digest = hashlib.sha256(archive_file.read()).digest()
        with open(staged_path, "rb") as staged_file:
            staged_digest = hashlib.sha256(staged_file.read()).digest()
        if archive_digest != staged_digest:
            raise SystemExit(f"staged file differs from immutable archive: {member_name}")
PY
test "$(sha256sum "$root/usr/lib/chillcenter/chill-server" | awk '{print $1}')" = '9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6'
grep -Fq '"source_commit": "126da0af167ee62588d92d90297f2dfc51962676"' "$root/usr/share/doc/chillcenter/build-manifest.json"
grep -Fq '"binary_sha256": "9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6"' "$root/usr/share/doc/chillcenter/build-manifest.json"

sudo -n bash -s -- "$root" "$nas_source" <<'SUDO_FINISH'
set -euo pipefail
root=$1
nas_source=$2
validate_nas_mount() {
  findmnt -rn -T /srv/volumes/video -o TARGET,SOURCE,FSTYPE,OPTIONS |
    awk -v target='/srv/volumes/video' -v source="$nas_source" '
      $1 == target && $2 == source && $3 == "cifs" {
        matching_records++
        count = split($4, options, ",")
        for (option_index = 1; option_index <= count; option_index++) {
          if (options[option_index] == "ro") found_ro = 1
          if (options[option_index] == "rw") found_rw = 1
        }
      }
      END { exit matching_records == 1 && found_ro && !found_rw ? 0 : 1 }
    '
}

test "$(getent passwd chillcenter)" = 'chillcenter:x:997:983::/home/chillcenter:/usr/sbin/nologin'
test "$(id -u chillcenter)" = 997
test "$(id -g chillcenter)" = 983
test "$(id -G chillcenter | tr ' ' '\n' | sort | paste -sd,)" = '983,984'
test "$(getent group marvin-admin | awk -F: '{print $3}')" = 984
for path in /usr/lib/chillcenter /usr/lib/chillcenter/chill-server /usr/lib/systemd/system/chillcenter.service /etc/systemd/system/chillcenter.service /etc/systemd/system/chillcenter.service.d /etc/chillcenter /etc/chillcenter/server.json /var/lib/chillcenter; do
  test ! -e "$path" && test ! -L "$path"
done
! systemctl list-unit-files --no-legend chillcenter.service 2>/dev/null | grep -q '^chillcenter\.service'
! ss -ltn | awk 'NR>1 {print $4}' | grep -Eq '(^|:)8443$'
validate_nas_mount
cd /
sudo -n -u chillcenter find /srv/volumes/video -mindepth 1 -maxdepth 1 -print -quit | grep -q .

install -d -o root -g root -m 0755 /usr/lib/chillcenter /usr/lib/systemd/system /usr/share/doc/chillcenter
install -d -o root -g chillcenter -m 0750 /etc/chillcenter
install -d -o chillcenter -g chillcenter -m 0700 /var/lib/chillcenter
install -o root -g root -m 0755 "$root/usr/lib/chillcenter/chill-server" /usr/lib/chillcenter/chill-server
install -o root -g root -m 0644 "$root/usr/lib/systemd/system/chillcenter.service" /usr/lib/systemd/system/chillcenter.service
install -o root -g root -m 0644 "$root/usr/share/doc/chillcenter/LINUX-PACKAGING.md" /usr/share/doc/chillcenter/LINUX-PACKAGING.md
install -o root -g root -m 0644 "$root/usr/share/doc/chillcenter/build-manifest.json" /usr/share/doc/chillcenter/build-manifest.json
install -o root -g chillcenter -m 0640 "$root/etc/chillcenter/server.json.example" /etc/chillcenter/server.json
cmp "$root/usr/lib/systemd/system/chillcenter.service" /usr/lib/systemd/system/chillcenter.service
cmp "$root/etc/chillcenter/server.json.example" /etc/chillcenter/server.json
systemd-analyze verify /usr/lib/systemd/system/chillcenter.service
systemctl daemon-reload
systemctl enable --now chillcenter.service
systemctl is-active --quiet chillcenter.service
test "$(stat -c '%U:%G %a' /var/lib/chillcenter)" = 'chillcenter:chillcenter 700'
test "$(stat -c '%U:%G %a' /etc/chillcenter/server.json)" = 'root:chillcenter 640'
echo 'R7 account completion: PASS; independent verifier must perform runtime checks.'
SUDO_FINISH
REMOTE_FINISH

echo "DEPLOYED-R7-COMPLETION: archive_sha256=$actual_archive_sha target=$TARGET_HOST"
