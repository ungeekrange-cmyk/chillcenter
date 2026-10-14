#!/usr/bin/env bash
# First-install runbook for the user-authorized Pi target only.
# This script intentionally refuses to run until a reviewer-approved integration
# gate and an exact, externally supplied archive SHA-256 are present.
set -euo pipefail

readonly EXPECTED_SOURCE_COMMIT="126da0af167ee62588d92d90297f2dfc51962676"
readonly TARGET_HOST="admin@192.168.1.42"
readonly TARGET_ARCH="aarch64-unknown-linux-gnu"
readonly NAS_ROOT="/srv/volumes/video"
readonly LISTEN_ADDRESS="192.168.1.42:8443"
readonly SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

usage() {
  cat >&2 <<'EOF'
Usage: install-pi-first-install.sh --archive /absolute/chillcenter-pi64.tar.gz \
  --sha256 <archive-sha256> --integration-gate /absolute/G-INTEGRATION-*.md \
  (--go | --stage-only) [--ssh-control /absolute/private-control-socket]

Preconditions: the gate is an independent APPROVED integration gate for source
126da0af167ee62588d92d90297f2dfc51962676 and records this archive SHA-256.
The script is intentionally first-install-only: it stops if any Chill Center
account, unit, binary, configuration, or state already exists on the Pi.

--stage-only transfers the already verified archive into a private /tmp staging
directory and runs only `readelf`, `ldd`, and `--help` against it. It does not
create a user, write system paths, or start/enable a service.
--ssh-control consumes an already established private SSH ControlMaster socket;
it neither creates a master session nor changes known-hosts files.
EOF
}

archive=""
expected_archive_sha=""
integration_gate=""
explicit_go=false
stage_only=false
ssh_control=""
while (($#)); do
  case "$1" in
    --archive) archive=${2:-}; shift 2 ;;
    --sha256) expected_archive_sha=${2:-}; shift 2 ;;
    --integration-gate) integration_gate=${2:-}; shift 2 ;;
    --go) explicit_go=true; shift ;;
    --stage-only) stage_only=true; shift ;;
    --ssh-control) ssh_control=${2:-}; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done

[[ -f "$archive" && -f "$integration_gate" ]] || { usage; exit 2; }
[[ "$explicit_go" != "$stage_only" ]] || { echo "Choose exactly one of --go or --stage-only." >&2; exit 2; }
[[ "$archive" = /* && "$integration_gate" = /* ]] || { echo "Archive and gate must be absolute paths." >&2; exit 2; }
[[ "$expected_archive_sha" =~ ^[[:xdigit:]]{64}$ ]] || { echo "Invalid archive SHA-256." >&2; exit 2; }
if [[ -n "$ssh_control" ]]; then
  [[ "$ssh_control" = /* ]] || { echo "SSH control socket must be an absolute path." >&2; exit 2; }
  python3 "$SCRIPT_DIR/helpers/validate-ssh-control.py" "$ssh_control"
fi

ssh_options=(
  -o BatchMode=yes
  -o StrictHostKeyChecking=yes
  -o ConnectTimeout=8
  -o ServerAliveInterval=5
  -o ServerAliveCountMax=1
)
if [[ -n "$ssh_control" ]]; then
  ssh_options+=(-o ControlMaster=no -o "ControlPath=$ssh_control")
fi

actual_archive_sha=$(shasum -a 256 "$archive" | awk '{print $1}')
[[ "$actual_archive_sha" == "$expected_archive_sha" ]] || { echo "Archive SHA-256 mismatch." >&2; exit 1; }

# The approval must bind both source and supplied immutable archive; no generic
# APPROVED marker can unlock a deployment.
grep -Eq 'G-INTEGRATION-[A-Za-z0-9_.-]+' "$integration_gate" || { echo "Missing integration gate id." >&2; exit 1; }
grep -Eq '(^|[^A-Za-z])APPROVED([^A-Za-z]|$)' "$integration_gate" || { echo "Integration gate is not APPROVED." >&2; exit 1; }
grep -Fq "$EXPECTED_SOURCE_COMMIT" "$integration_gate" || { echo "Gate does not bind the approved source commit." >&2; exit 1; }
grep -Fqi "$expected_archive_sha" "$integration_gate" || { echo "Gate does not bind this archive SHA-256." >&2; exit 1; }

local_stage=$(mktemp -d /private/tmp/chillcenter-pi-install.XXXXXX)
cleanup_local() { rm -rf "$local_stage"; }
trap cleanup_local EXIT
python3 - "$archive" <<'PY'
import pathlib, tarfile, sys
with tarfile.open(pathlib.Path(sys.argv[1]), "r:gz") as archive:
    members = archive.getmembers()
    if not members:
        raise SystemExit("Archive is empty.")
    for member in members:
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts or member.issym() or member.islnk():
            raise SystemExit(f"Unsafe archive member: {member.name}")
PY
tar -xzf "$archive" -C "$local_stage" --no-same-owner --no-same-permissions
package_root=$(find "$local_stage" -mindepth 1 -maxdepth 1 -type d -print)
[[ $(printf '%s\n' "$package_root" | wc -l | tr -d ' ') -eq 1 ]] || { echo "Archive must contain exactly one package root." >&2; exit 1; }
manifest="$package_root/usr/share/doc/chillcenter/build-manifest.json"
binary="$package_root/usr/lib/chillcenter/chill-server"
unit="$package_root/usr/lib/systemd/system/chillcenter.service"
example_config="$package_root/etc/chillcenter/server.json.example"
[[ -f "$manifest" && -f "$binary" && -f "$unit" && -f "$example_config" ]] || { echo "Required package members missing." >&2; exit 1; }

python3 - "$manifest" "$binary" "$EXPECTED_SOURCE_COMMIT" "$TARGET_ARCH" <<'PY'
import hashlib, json, pathlib, sys
manifest_path = pathlib.Path(sys.argv[1])
binary_path = pathlib.Path(sys.argv[2])
expected_commit = sys.argv[3]
expected_target = sys.argv[4]
manifest = json.loads(manifest_path.read_text())
if manifest.get("source_commit") != expected_commit:
    raise SystemExit("Package manifest source commit mismatch.")
if manifest.get("target") != expected_target:
    raise SystemExit("Package manifest target mismatch.")
actual = hashlib.sha256(binary_path.read_bytes()).hexdigest()
if manifest.get("binary_sha256") != actual:
    raise SystemExit("Package manifest binary SHA-256 mismatch.")
header = binary_path.read_bytes()[:20]
if len(header) < 20 or header[:4] != b"\x7fELF" or header[4] != 2 or header[18:20] != b"\xb7\x00":
    raise SystemExit("Package binary is not ELF64/AArch64.")
print("Local artifact verification: PASS")
PY

# The production configuration is a verbatim copy of the reviewed package
# example, never generated by a remote shell.  Bind it before staging.
python3 - "$example_config" "$LISTEN_ADDRESS" "$NAS_ROOT" <<'PY'
import json, pathlib, sys
config = json.loads(pathlib.Path(sys.argv[1]).read_text())
expected = {
    "listen": sys.argv[2],
    "data_dir": "/var/lib/chillcenter/data",
    "tls_names": ["192.168.1.42"],
    "video_root": sys.argv[3],
}
if config != expected:
    raise SystemExit("Reviewed package configuration does not match the authorized Pi values.")
print("Local configuration contract: PASS")
PY

# No remote mutation occurs before this inventory. It reads only metadata and
# rejects an installation that appeared after the recorded baseline.
ssh "${ssh_options[@]}" "$TARGET_HOST" 'bash -s' <<'REMOTE_PREFLIGHT'
set -euo pipefail
validate_nas_mount() {
  expected_root=$1
  findmnt -rn -T "$expected_root" -o TARGET,SOURCE,FSTYPE,OPTIONS |
    awk -v target="$expected_root" -v source='//192.168.1.92/Video' '
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
sudo -n true
test "$(uname -m)" = aarch64
test "$(getent group marvin-admin | awk -F: '{print $3}')" = 984
validate_nas_mount /srv/volumes/video
for path in /usr/lib/chillcenter /usr/lib/chillcenter/chill-server /usr/lib/systemd/system/chillcenter.service /etc/systemd/system/chillcenter.service /etc/systemd/system/chillcenter.service.d /etc/chillcenter /etc/chillcenter/server.json /var/lib/chillcenter; do
  test ! -e "$path" && test ! -L "$path" || { echo "Refusing: pre-existing Chill Center path: $path" >&2; exit 1; }
done
! getent passwd chillcenter >/dev/null
! systemctl list-unit-files --no-legend chillcenter.service 2>/dev/null | grep -q '^chillcenter\.service'
! ss -ltn | awk 'NR>1 {print $4}' | grep -Eq '(^|:)8443$'
echo 'Remote first-install inventory: PASS'
REMOTE_PREFLIGHT

remote_stage="/tmp/chillcenter-first-install-${actual_archive_sha}"
ssh "${ssh_options[@]}" "$TARGET_HOST" "umask 077; mkdir '$remote_stage'"
scp "${ssh_options[@]}" -p "$archive" "$TARGET_HOST:$remote_stage/archive.tar.gz"

if [[ "$stage_only" == true ]]; then
  ssh "${ssh_options[@]}" "$TARGET_HOST" "bash -s -- '$remote_stage' '$actual_archive_sha'" <<'REMOTE_STAGE_ONLY'
set -euo pipefail
stage=$1
expected_archive_sha=$2
verify_help_contract() {
  binary=$1
  expected_help=$'Chill Center : Usage : chill-server --config /chemin/config.json\n       chill-server --backup --config /chemin/config.json\n       chill-server --restore /chemin/sauvegarde --config /chemin/config.json --confirm-replace\n       chill-server --recover --config /chemin/config.json --confirm-replace\n       chill-server --export-cinefolio --source-db /chemin/cinefolio.sqlite --image-cache /chemin/images --output /chemin/nouveau.zip'
  if help_output=$("$binary" --help 2>&1); then
    help_status=0
  else
    help_status=$?
  fi
  test "$help_status" -eq 1 || return 1
  test "$help_output" = "$expected_help" || return 1
  echo 'Runtime help contract: PASS'
}
test "$(sha256sum "$stage/archive.tar.gz" | awk '{print $1}')" = "$expected_archive_sha"
mkdir -m 700 "$stage/unpacked"
tar -xzf "$stage/archive.tar.gz" -C "$stage/unpacked" --no-same-owner --no-same-permissions
set -- "$stage"/unpacked/*
test $# -eq 1 && test -d "$1"
binary="$1/usr/lib/chillcenter/chill-server"
test -f "$binary"
readelf -h "$binary" | grep -Eq 'Class:.*ELF64'
readelf -h "$binary" | grep -Eq 'Machine:.*AArch64'
ldd "$binary"
verify_help_contract "$binary"
echo 'Stage-only binary checks: PASS; no privileged installation action occurred.'
REMOTE_STAGE_ONLY
  echo "STAGED_ONLY: source=$EXPECTED_SOURCE_COMMIT archive_sha256=$actual_archive_sha target=$TARGET_HOST"
  exit 0
fi

# The privileged block repeats first-install checks immediately before writes.
# It creates only the approved account/group membership, service files, local
# state and configuration; it neither remounts nor reads NAS credentials.
ssh "${ssh_options[@]}" "$TARGET_HOST" "bash -s -- '$remote_stage' '$actual_archive_sha'" <<'REMOTE_INSTALL'
set -euo pipefail
stage=$1
expected_archive_sha=$2
readonly NAS_ROOT=/srv/volumes/video
verify_help_contract() {
  binary=$1
  expected_help=$'Chill Center : Usage : chill-server --config /chemin/config.json\n       chill-server --backup --config /chemin/config.json\n       chill-server --restore /chemin/sauvegarde --config /chemin/config.json --confirm-replace\n       chill-server --recover --config /chemin/config.json --confirm-replace\n       chill-server --export-cinefolio --source-db /chemin/cinefolio.sqlite --image-cache /chemin/images --output /chemin/nouveau.zip'
  if help_output=$("$binary" --help 2>&1); then
    help_status=0
  else
    help_status=$?
  fi
  test "$help_status" -eq 1 || return 1
  test "$help_output" = "$expected_help" || return 1
  echo 'Runtime help contract: PASS'
}

test "$(sha256sum "$stage/archive.tar.gz" | awk '{print $1}')" = "$expected_archive_sha"
mkdir -m 700 "$stage/unpacked"
tar -xzf "$stage/archive.tar.gz" -C "$stage/unpacked" --no-same-owner --no-same-permissions
set -- "$stage"/unpacked/*
test $# -eq 1 && test -d "$1"
root=$1
test -f "$root/usr/lib/chillcenter/chill-server"
test -f "$root/usr/lib/systemd/system/chillcenter.service"
test -f "$root/etc/chillcenter/server.json.example"
binary="$root/usr/lib/chillcenter/chill-server"
# Non-privileged remote evidence for the exact transferred binary.  These
# checks occur before sudo, account creation, system paths, or systemctl.
readelf -h "$binary" | grep -Eq 'Class:.*ELF64'
readelf -h "$binary" | grep -Eq 'Machine:.*AArch64'
ldd "$binary"
verify_help_contract "$binary"

sudo -n bash -s -- "$root" "$NAS_ROOT" <<'SUDO_INSTALL'
set -euo pipefail
root=$1
nas_root=$2
validate_nas_mount() {
  expected_root=$1
  findmnt -rn -T "$expected_root" -o TARGET,SOURCE,FSTYPE,OPTIONS |
    awk -v target="$expected_root" -v source='//192.168.1.92/Video' '
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

test "$(uname -m)" = aarch64
test "$(getent group marvin-admin | awk -F: '{print $3}')" = 984
validate_nas_mount "$nas_root"
for path in /usr/lib/chillcenter /usr/lib/chillcenter/chill-server /usr/lib/systemd/system/chillcenter.service /etc/systemd/system/chillcenter.service /etc/systemd/system/chillcenter.service.d /etc/chillcenter /etc/chillcenter/server.json /var/lib/chillcenter; do
  test ! -e "$path" && test ! -L "$path" || { echo "Refusing: pre-existing Chill Center path: $path" >&2; exit 1; }
done
! getent passwd chillcenter >/dev/null
! systemctl list-unit-files --no-legend chillcenter.service 2>/dev/null | grep -q '^chillcenter\.service'
! ss -ltn | awk 'NR>1 {print $4}' | grep -Eq '(^|:)8443$'

useradd --system --user-group --no-create-home --shell /usr/sbin/nologin --groups marvin-admin chillcenter
id chillcenter | grep -Eq 'groups=.*marvin-admin'
# Proves directory enumeration through the approved supplementary group without
# reading a media file or logging any filename.
cd /
sudo -n -u chillcenter find "$nas_root" -mindepth 1 -maxdepth 1 -print -quit | grep -q .
install -d -o root -g root -m 0755 /usr/lib/chillcenter /usr/lib/systemd/system /usr/share/doc/chillcenter
install -d -o root -g chillcenter -m 0750 /etc/chillcenter
install -d -o chillcenter -g chillcenter -m 0700 /var/lib/chillcenter
install -o root -g root -m 0755 "$root/usr/lib/chillcenter/chill-server" /usr/lib/chillcenter/chill-server
install -o root -g root -m 0644 "$root/usr/lib/systemd/system/chillcenter.service" /usr/lib/systemd/system/chillcenter.service
install -o root -g root -m 0644 "$root/usr/share/doc/chillcenter/LINUX-PACKAGING.md" /usr/share/doc/chillcenter/LINUX-PACKAGING.md
install -o root -g root -m 0644 "$root/usr/share/doc/chillcenter/build-manifest.json" /usr/share/doc/chillcenter/build-manifest.json
install -o root -g chillcenter -m 0640 "$root/etc/chillcenter/server.json.example" /etc/chillcenter/server.json

systemd-analyze verify /usr/lib/systemd/system/chillcenter.service
systemctl daemon-reload
systemctl enable --now chillcenter.service
systemctl is-active --quiet chillcenter.service
test "$(stat -c '%U:%G %a' /var/lib/chillcenter)" = 'chillcenter:chillcenter 700'
test "$(stat -c '%U:%G %a' /etc/chillcenter/server.json)" = 'root:chillcenter 640'
test "$(id -u chillcenter)" -ne 0
echo 'Installation first-install: PASS; independent verifier must perform TLS/API/NAS smoke.'
SUDO_INSTALL
REMOTE_INSTALL

echo "DEPLOYED: source=$EXPECTED_SOURCE_COMMIT archive_sha256=$actual_archive_sha target=$TARGET_HOST"
echo 'Handoff required: distinct release verifier; do not claim TLS/API/NAS smoke from this installation script.'
