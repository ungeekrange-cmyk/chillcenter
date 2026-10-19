#!/usr/bin/env bash
# Generic first-install-only Linux installer. It consumes already downloaded,
# inspectable local files; it never downloads or executes a remote script and
# never mounts, remounts, writes, chowns, or probes the configured library.
set -euo pipefail
readonly PINNED_RELEASE_KEY='RWT4AsJIC3n9GIPorIeouS7Ycc5ROkeCsMXypDtOGnikR2Jvm6cuxeQA'
readonly KEY_ID='18FD790B48C202F8'
readonly SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

usage() {
  cat >&2 <<'EOF'
Usage: install-server.sh --server /absolute/chill-server \
  --updater /absolute/chill-update --release-pub /absolute/release.pub \
  --manifest /absolute/server-manifest.json \
  --manifest-signature /absolute/server-manifest.json.minisig \
  --version X.Y.Z --library /absolute/read-only-library \
  --listen ADDRESS:PORT --tls-name DNS_OR_IP

First install only. The supplied release must be verified before this script is
executed. Existing accounts, binaries, configs, units or state are not replaced.
EOF
}

server= updater= release_pub= manifest= manifest_signature= version= library= listen= tls_name=
while (($#)); do
  case "$1" in
    --server) server=${2:-}; shift 2 ;;
    --updater) updater=${2:-}; shift 2 ;;
    --release-pub) release_pub=${2:-}; shift 2 ;;
    --manifest) manifest=${2:-}; shift 2 ;;
    --manifest-signature) manifest_signature=${2:-}; shift 2 ;;
    --version) version=${2:-}; shift 2 ;;
    --library) library=${2:-}; shift 2 ;;
    --listen) listen=${2:-}; shift 2 ;;
    --tls-name) tls_name=${2:-}; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done

[[ ${EUID:-$(id -u)} -eq 0 ]] || { echo 'Run as root.' >&2; exit 2; }
for value in "$server" "$updater" "$release_pub" "$manifest" "$manifest_signature" "$library"; do [[ "$value" = /* ]] || { usage; exit 2; }; done
[[ -f "$server" && ! -L "$server" && -x "$server" ]] || { echo 'Server must be a regular executable.' >&2; exit 2; }
[[ -f "$updater" && ! -L "$updater" && -x "$updater" ]] || { echo 'Updater must be a regular executable.' >&2; exit 2; }
[[ -f "$release_pub" && ! -L "$release_pub" ]] || { echo 'Release public key must be a regular file.' >&2; exit 2; }
[[ -f "$manifest" && ! -L "$manifest" && -f "$manifest_signature" && ! -L "$manifest_signature" ]] || { echo 'Signed server manifest files are required.' >&2; exit 2; }
library_pattern='^/[A-Za-z0-9._/ -]+$'
[[ -d "$library" && ! -L "$library" && "$library" =~ $library_pattern ]] || { echo 'Library must be an existing absolute directory without JSON metacharacters.' >&2; exit 2; }
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || { echo 'Version must be stable SemVer.' >&2; exit 2; }
[[ "$listen" =~ ^[A-Za-z0-9.:-]+:[0-9]{1,5}$ ]] || { echo 'Invalid listen address.' >&2; exit 2; }
port=${listen##*:}; ((port >= 1 && port <= 65535)) || { echo 'Invalid listen port.' >&2; exit 2; }
[[ "$tls_name" =~ ^[A-Za-z0-9._:-]+$ ]] || { echo 'Invalid TLS name.' >&2; exit 2; }
[[ "$(sed -n '2p' "$release_pub")" == "$PINNED_RELEASE_KEY" ]] || { echo 'Release key is not the pinned Chill Center trust root.' >&2; exit 1; }

case "$(uname -m)" in
  aarch64|arm64) target=linux-aarch64; updater_target=updater-aarch64-unknown-linux-gnu ;;
  armv7l) target=linux-armv7; updater_target=updater-armv7-unknown-linux-gnueabihf ;;
  x86_64) target=linux-x86_64; updater_target=updater-x86_64-unknown-linux-gnu ;;
  *) echo 'Unsupported Linux architecture.' >&2; exit 1 ;;
esac

# Bootstrap verification cannot trust the downloaded updater binary to verify
# itself. Use the independently installed Minisign CLI, then bind both local
# binaries to the signed manifest before either binary is executed.
command -v minisign >/dev/null || { echo 'System minisign is required for bootstrap verification.' >&2; exit 1; }
command -v python3 >/dev/null || { echo 'Python 3 is required for strict bootstrap manifest parsing.' >&2; exit 1; }
minisign -Vm "$manifest" -x "$manifest_signature" -p "$release_pub" >/dev/null
python3 - "$manifest" "$server" "$updater" "$version" "$target" "$updater_target" <<'PY'
import hashlib, json, pathlib, sys

class Strict(dict):
    pass

def pairs(values):
    result = Strict()
    for key, value in values:
        if key in result:
            raise SystemExit(f"duplicate manifest key: {key}")
        result[key] = value
    return result

manifest_path, server_path, updater_path, version, server_target, updater_target = sys.argv[1:]
raw = pathlib.Path(manifest_path).read_bytes()
if len(raw) > 1024 * 1024:
    raise SystemExit("manifest exceeds bootstrap bound")
document = json.loads(raw, object_pairs_hook=pairs)
if document.get("schema_version") != 1 or document.get("component") != "server":
    raise SystemExit("not a schema-1 server manifest")
if document.get("version") != version or document.get("release_tag") != f"server-v{version}":
    raise SystemExit("manifest version/tag mismatch")
assets = document.get("assets")
if not isinstance(assets, list) or len(assets) > 32:
    raise SystemExit("invalid manifest assets")

def verify(target, local_path):
    matches = [asset for asset in assets if isinstance(asset, dict) and asset.get("target") == target]
    if len(matches) != 1:
        raise SystemExit(f"manifest must contain exactly one {target} asset")
    asset = matches[0]
    path = pathlib.Path(local_path)
    size = path.stat().st_size
    digest_state = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest_state.update(chunk)
    digest = digest_state.hexdigest()
    if asset.get("byte_length") != size or asset.get("sha256") != digest:
        raise SystemExit(f"signed size/hash mismatch for {target}")

verify(server_target, server_path)
verify(updater_target, updater_path)
print("External Minisign bootstrap verification: PASS")
PY

for path in /usr/lib/chillcenter/chill-update /usr/lib/chillcenter/server /usr/lib/chillcenter/web /usr/lib/chillcenter/themes /etc/chillcenter/server.json /etc/chillcenter/updates.json /var/lib/chillcenter-update /var/lib/chillcenter; do
  [[ ! -e "$path" && ! -L "$path" ]] || { echo "Refusing existing path: $path" >&2; exit 1; }
done
! getent passwd chillcenter >/dev/null || { echo 'Refusing existing chillcenter account.' >&2; exit 1; }
! systemctl list-unit-files chillcenter.service --no-legend 2>/dev/null | grep -q '^chillcenter\.service' || { echo 'Refusing existing unit.' >&2; exit 1; }

useradd --system --user-group --no-create-home --shell /usr/sbin/nologin chillcenter
uid=$(id -u chillcenter); gid=$(id -g chillcenter)
install -d -o root -g root -m 0755 /usr/lib/chillcenter /usr/lib/chillcenter/server/releases/"$version"-1 /usr/lib/chillcenter/web/releases /usr/lib/chillcenter/themes/releases
install -d -o root -g chillcenter -m 0750 /etc/chillcenter
install -d -o chillcenter -g chillcenter -m 0700 /var/lib/chillcenter /var/lib/chillcenter/data /var/lib/chillcenter/data/updates /var/lib/chillcenter/data/backups
install -d -o root -g root -m 0700 /var/lib/chillcenter-update /var/lib/chillcenter-update/trust /var/lib/chillcenter-update/cache
install -d -o root -g root -m 0755 /run/chillcenter-update
install -o root -g root -m 0755 "$server" /usr/lib/chillcenter/server/releases/"$version"-1/payload
ln -s releases/"$version"-1 /usr/lib/chillcenter/server/current
install -o root -g root -m 0755 "$updater" /usr/lib/chillcenter/chill-update
install -o root -g root -m 0644 "$release_pub" /etc/chillcenter/release.pub

if [[ "$tls_name" == localhost ]]; then tls_names='["localhost"]'; else tls_names="[\"$tls_name\",\"localhost\"]"; fi
printf '{\n  "listen": "%s",\n  "data_dir": "/var/lib/chillcenter/data",\n  "tls_names": %s,\n  "video_root": "%s"\n}\n' "$listen" "$tls_names" "$library" > /etc/chillcenter/server.json
chown root:chillcenter /etc/chillcenter/server.json
chmod 0640 /etc/chillcenter/server.json

install -o root -g root -m 0644 "$SCRIPT_DIR/../deploy/updates/chillcenter.service" /usr/lib/systemd/system/chillcenter.service
install -o root -g root -m 0644 "$SCRIPT_DIR/../deploy/updates/chillcenter-update.service" /usr/lib/systemd/system/chillcenter-update.service
install -o root -g root -m 0644 "$SCRIPT_DIR/../deploy/updates/chillcenter-update.timer" /usr/lib/systemd/system/chillcenter-update.timer
systemd-analyze verify /usr/lib/systemd/system/chillcenter.service /usr/lib/systemd/system/chillcenter-update.service /usr/lib/systemd/system/chillcenter-update.timer
systemctl daemon-reload
systemctl enable --now chillcenter.service
systemctl is-active --quiet chillcenter.service

for _ in $(seq 1 30); do [[ -f /var/lib/chillcenter/data/tls/certificate.pem ]] && break; sleep 1; done
[[ -f /var/lib/chillcenter/data/tls/certificate.pem ]] || { echo 'Server did not produce its TLS certificate.' >&2; exit 1; }
install -o root -g root -m 0644 /var/lib/chillcenter/data/tls/certificate.pem /etc/chillcenter/health-ca.pem

cat > /etc/chillcenter/updates.json <<EOF
{
  "schema_version": 1,
  "key_id": "$KEY_ID",
  "public_key": "/etc/chillcenter/release.pub",
  "state_root": "/var/lib/chillcenter/data/updates",
  "trust_root": "/var/lib/chillcenter-update/trust",
  "cache_root": "/var/lib/chillcenter-update/cache",
  "backup_root": "/var/lib/chillcenter/data/backups",
  "maintenance_flag": "/run/chillcenter-update/maintenance",
  "quiescence_status": "/var/lib/chillcenter/data/updates/quiescence.json",
  "server_config": "/etc/chillcenter/server.json",
  "service_name": "chillcenter.service",
  "service_uid": $uid,
  "service_gid": $gid,
  "health_url": "https://localhost:$port/api/2026-10-14/server",
  "health_ca_certificate": "/etc/chillcenter/health-ca.pem",
  "component_roots": {"server":"/usr/lib/chillcenter/server","web":"/usr/lib/chillcenter/web","themes":"/usr/lib/chillcenter/themes"},
  "targets": {"server":"$target","web":"web","themes":"universal"},
  "bootstrap_versions": {"server":"$version","web":"0.0.0","themes":"0.0.0"},
  "api_version": "2026.10.14",
  "database_schema": 3,
  "renderer_version": 1,
  "theme_format": 1
}
EOF
chown root:root /etc/chillcenter/updates.json
chmod 0640 /etc/chillcenter/updates.json

curl --fail --silent --show-error --cacert /etc/chillcenter/health-ca.pem "https://localhost:$port/api/2026-10-14/server" >/dev/null
/usr/lib/chillcenter/chill-update --config /etc/chillcenter/updates.json inspect >/dev/null
systemctl enable --now chillcenter-update.timer
echo 'Chill Center installed as a non-root service. Library and mount configuration were not modified.'
