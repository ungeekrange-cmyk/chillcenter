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
[[ "$listen" =~ ^[0-9.]+:[0-9]{1,5}$ ]] || { echo 'Listen must be an explicit IPv4 address and decimal port.' >&2; exit 2; }
listen_ip=${listen%:*}; port=${listen##*:}; port_decimal=$((10#$port)); ((port_decimal >= 1 && port_decimal <= 65535)) || { echo 'Invalid listen port.' >&2; exit 2; }
port=$port_decimal
[[ "$tls_name" =~ ^[A-Za-z0-9._-]+$ ]] || { echo 'Invalid TLS name.' >&2; exit 2; }

for tool in bash cat chmod chown cmp curl getent grep id install ln minisign mktemp python3 rm sed seq sha256sum sleep systemctl systemd-analyze uname useradd; do
  command -v "$tool" >/dev/null || { echo "Required tool is missing: $tool" >&2; exit 1; }
done

python3 - "$release_pub" "$PINNED_RELEASE_KEY" "$listen_ip" "$port" "$tls_name" <<'PY'
import ipaddress, pathlib, socket, sys

key_path, expected_key, listen_text, port_text, tls_name = sys.argv[1:]
lines = pathlib.Path(key_path).read_text(encoding="utf-8").splitlines()
if len(lines) != 2 or not lines[0].startswith("untrusted comment: minisign public key") or lines[1] != expected_key:
    raise SystemExit("release key file is not the exact pinned full Minisign public file")
listen = ipaddress.ip_address(listen_text)
if not (listen.is_private or listen.is_loopback or listen.is_link_local) or listen.is_unspecified or listen.is_multicast:
    raise SystemExit("listen address must be private, loopback, or link-local")
resolved = {
    ipaddress.ip_address(item[4][0])
    for item in socket.getaddrinfo(tls_name, int(port_text), type=socket.SOCK_STREAM)
}
if listen not in resolved:
    raise SystemExit("TLS name does not resolve to the configured listen address")
print("LAN/TLS preflight: PASS")
PY

case "$(uname -m)" in
  aarch64|arm64) target=linux-aarch64; updater_target=updater-aarch64-unknown-linux-gnu ;;
  armv7l) target=linux-armv7; updater_target=updater-armv7-unknown-linux-gnueabihf ;;
  x86_64) target=linux-x86_64; updater_target=updater-x86_64-unknown-linux-gnu ;;
  *) echo 'Unsupported Linux architecture.' >&2; exit 1 ;;
esac

bootstrap_stage=$(mktemp -d /var/tmp/chillcenter-bootstrap.XXXXXX)
chmod 0700 "$bootstrap_stage"
cleanup_stage() { rm -rf -- "$bootstrap_stage"; }
trap cleanup_stage EXIT
python3 - "$bootstrap_stage" "$server" "$updater" "$release_pub" "$manifest" "$manifest_signature" \
  "$SCRIPT_DIR/../deploy/updates/chillcenter.service" \
  "$SCRIPT_DIR/../deploy/updates/chillcenter-update.service" \
  "$SCRIPT_DIR/../deploy/updates/chillcenter-update.timer" <<'PY'
import os, pathlib, stat, sys

stage = pathlib.Path(sys.argv[1])
sources = sys.argv[2:]
names = ["server", "updater", "release.pub", "manifest.json", "manifest.minisig",
         "chillcenter.service", "chillcenter-update.service", "chillcenter-update.timer"]
limits = [512*1024*1024, 128*1024*1024, 16*1024, 1024*1024, 16*1024,
          128*1024, 128*1024, 128*1024]
for source, name, limit in zip(sources, names, limits, strict=True):
    source_fd = os.open(source, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        info = os.fstat(source_fd)
        if not stat.S_ISREG(info.st_mode) or info.st_size < 1 or info.st_size > limit:
            raise SystemExit(f"unsafe bootstrap input: {name}")
        destination_fd = os.open(stage / name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
        try:
            remaining = info.st_size
            while remaining:
                chunk = os.read(source_fd, min(1024 * 1024, remaining))
                if not chunk:
                    raise SystemExit(f"short bootstrap input: {name}")
                view = memoryview(chunk)
                while view:
                    written = os.write(destination_fd, view)
                    if written < 1:
                        raise SystemExit(f"short bootstrap write: {name}")
                    view = view[written:]
                remaining -= len(chunk)
            os.fsync(destination_fd)
        finally:
            os.close(destination_fd)
    finally:
        os.close(source_fd)
stage_fd = os.open(stage, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
try:
    os.fsync(stage_fd)
finally:
    os.close(stage_fd)
PY
server="$bootstrap_stage/server"
updater="$bootstrap_stage/updater"
release_pub="$bootstrap_stage/release.pub"
manifest="$bootstrap_stage/manifest.json"
manifest_signature="$bootstrap_stage/manifest.minisig"
server_unit="$bootstrap_stage/chillcenter.service"
update_unit="$bootstrap_stage/chillcenter-update.service"
timer_unit="$bootstrap_stage/chillcenter-update.timer"
python3 - "$release_pub" "$PINNED_RELEASE_KEY" <<'PY'
import pathlib, sys
lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 2 or not lines[0].startswith("untrusted comment: minisign public key") or lines[1] != sys.argv[2]:
    raise SystemExit("staged release key is not the exact pinned Minisign public file")
PY

# Bootstrap verification cannot trust the downloaded updater binary to verify
# itself. Use the independently installed Minisign CLI, then bind both local
# binaries to the signed manifest before either binary is executed.
command -v minisign >/dev/null || { echo 'System minisign is required for bootstrap verification.' >&2; exit 1; }
command -v python3 >/dev/null || { echo 'Python 3 is required for strict bootstrap manifest parsing.' >&2; exit 1; }
minisign -Vm "$manifest" -x "$manifest_signature" -p "$release_pub" >/dev/null
python3 - "$manifest" "$server" "$updater" "$server_unit" "$update_unit" "$timer_unit" "$version" "$target" "$updater_target" "$KEY_ID" <<'PY'
import datetime, hashlib, json, pathlib, re, sys, urllib.parse

class Strict(dict):
    pass

def pairs(values):
    result = Strict()
    for key, value in values:
        if key in result:
            raise SystemExit(f"duplicate manifest key: {key}")
        result[key] = value
    return result

manifest_path, server_path, updater_path, server_unit, update_unit, timer_unit, version, server_target, updater_target, key_id = sys.argv[1:]
raw = pathlib.Path(manifest_path).read_bytes()
if len(raw) > 1024 * 1024:
    raise SystemExit("manifest exceeds bootstrap bound")
document = json.loads(raw, object_pairs_hook=pairs)
expected_root = {"schema_version", "component", "version", "release_sequence", "release_tag", "source_commit", "issued_at", "expires_at", "key_id", "compatibility", "assets"}
if set(document) != expected_root:
    raise SystemExit("manifest root fields differ from schema 1")
if document.get("schema_version") != 1 or document.get("component") != "server":
    raise SystemExit("not a schema-1 server manifest")
if document.get("version") != version or document.get("release_tag") != f"server-v{version}":
    raise SystemExit("manifest version/tag mismatch")
if document.get("key_id") != key_id:
    raise SystemExit("manifest key id mismatch")
if isinstance(document.get("release_sequence"), bool) or not isinstance(document.get("release_sequence"), int) or document["release_sequence"] < 1:
    raise SystemExit("invalid release sequence")
if not isinstance(document.get("source_commit"), str) or not re.fullmatch(r"[0-9a-f]{40}", document["source_commit"]):
    raise SystemExit("source commit must be a lowercase full SHA-1")
try:
    timestamp_pattern = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$")
    if not timestamp_pattern.fullmatch(document["issued_at"]) or not timestamp_pattern.fullmatch(document["expires_at"]):
        raise ValueError("timestamp syntax")
    issued = datetime.datetime.fromisoformat(document["issued_at"].replace("Z", "+00:00"))
    expires = datetime.datetime.fromisoformat(document["expires_at"].replace("Z", "+00:00"))
except (KeyError, TypeError, ValueError) as error:
    raise SystemExit("invalid RFC3339 manifest time") from error
now = datetime.datetime.now(datetime.timezone.utc)
if issued.tzinfo is None or expires.tzinfo is None or issued > now + datetime.timedelta(minutes=10) or expires <= now or expires <= issued:
    raise SystemExit("manifest is not currently valid")
compatibility = document.get("compatibility")
compatibility_fields = {"os_arch", "api", "required_server", "database", "renderer"}
if not isinstance(compatibility, dict) or "os_arch" not in compatibility or not set(compatibility) <= compatibility_fields:
    raise SystemExit("compatibility fields differ from schema 1")
targets = compatibility.get("os_arch", [])
if not isinstance(targets, list) or len(targets) > 16 or len(set(targets)) != len(targets) or server_target not in targets:
    raise SystemExit("manifest does not support this runtime target")
if any(not isinstance(value, str) or not re.fullmatch(r"[a-z0-9][a-z0-9._-]{0,127}", value) for value in targets):
    raise SystemExit("invalid runtime target")
semver = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
for field in ("api", "required_server"):
    bounds = compatibility.get(field)
    if bounds is not None and (not isinstance(bounds, dict) or set(bounds) != {"min", "max"}
                               or not semver.fullmatch(bounds.get("min", ""))
                               or not semver.fullmatch(bounds.get("max", ""))):
        raise SystemExit(f"invalid {field} compatibility")
def semver_tuple(value):
    return tuple(int(part) for part in value.split("."))
api_bounds = compatibility.get("api")
if api_bounds is None or not (semver_tuple(api_bounds["min"]) <= (2026, 10, 14) <= semver_tuple(api_bounds["max"])):
    raise SystemExit("manifest API range excludes bootstrap runtime")
if compatibility.get("required_server") is not None:
    raise SystemExit("server bootstrap cannot require a pre-existing server")
database = compatibility.get("database")
db_fields = {"current_schema", "target_schema", "read_min", "read_max", "write_min", "write_max", "rollback_min", "rollback_max"}
if database is not None:
    if not isinstance(database, dict) or set(database) != db_fields or any(isinstance(value, bool) or not isinstance(value, int) or value < 0 for value in database.values()):
        raise SystemExit("invalid database compatibility")
    if database["current_schema"] != 3 or database["target_schema"] != 3 or not all(database[low] <= 3 <= database[high] for low, high in (("read_min","read_max"),("write_min","write_max"),("rollback_min","rollback_max"))):
        raise SystemExit("bootstrap refuses database migration or incompatible schema")
else:
    raise SystemExit("server bootstrap requires explicit database compatibility")
renderer = compatibility.get("renderer")
if renderer is not None and (not isinstance(renderer, dict) or set(renderer) != {"theme_format", "renderer_min", "renderer_max"}
                             or any(isinstance(value, bool) or not isinstance(value, int) or value < 0 for value in renderer.values())):
    raise SystemExit("invalid renderer compatibility")
assets = document.get("assets")
if not isinstance(assets, list) or not assets or len(assets) > 32:
    raise SystemExit("invalid manifest assets")
seen_targets = set()
for asset in assets:
    if not isinstance(asset, dict) or set(asset) not in ({"target", "url", "byte_length", "sha256", "format"}, {"target", "url", "byte_length", "sha256", "format", "tauri_signature"}):
        raise SystemExit("asset fields differ from schema 1")
    asset_target = asset.get("target")
    if not isinstance(asset_target, str) or not re.fullmatch(r"[a-z0-9][a-z0-9._-]{0,127}", asset_target) or asset_target in seen_targets:
        raise SystemExit("invalid or duplicate asset target")
    seen_targets.add(asset_target)
    length = asset.get("byte_length")
    if isinstance(length, bool) or not isinstance(length, int) or length < 1 or length > 2 * 1024 * 1024 * 1024:
        raise SystemExit("invalid asset length")
    if not isinstance(asset.get("sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", asset["sha256"]):
        raise SystemExit("invalid asset hash")

def verify(target, local_path):
    matches = [asset for asset in assets if isinstance(asset, dict) and asset.get("target") == target]
    if len(matches) != 1:
        raise SystemExit(f"manifest must contain exactly one {target} asset")
    asset = matches[0]
    if set(asset) not in ({"target", "url", "byte_length", "sha256", "format"}, {"target", "url", "byte_length", "sha256", "format", "tauri_signature"}):
        raise SystemExit(f"unknown asset field for {target}")
    if asset.get("format") != "raw" or asset.get("tauri_signature") is not None:
        raise SystemExit(f"bootstrap asset {target} must be raw and non-Tauri")
    parsed = urllib.parse.urlparse(asset.get("url", ""))
    required = f"/ungeekrange-cmyk/chillcenter/releases/download/server-v{version}/"
    if parsed.scheme != "https" or parsed.netloc != "github.com" or not parsed.path.startswith(required) or parsed.query or parsed.fragment:
        raise SystemExit(f"asset URL is outside the fixed release for {target}")
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
verify("unit-chillcenter-service", server_unit)
verify("unit-chillcenter-update-service", update_unit)
verify("unit-chillcenter-update-timer", timer_unit)
print("External Minisign bootstrap verification: PASS")
PY
chmod 0700 "$server" "$updater"

for path in /usr/lib/chillcenter/chill-update /usr/lib/chillcenter/server /usr/lib/chillcenter/web /usr/lib/chillcenter/themes /etc/chillcenter/server.json /etc/chillcenter/updates.json /var/lib/chillcenter-update /var/lib/chillcenter /usr/lib/systemd/system/chillcenter.service /usr/lib/systemd/system/chillcenter-update.service /usr/lib/systemd/system/chillcenter-update.timer; do
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
  "health_url": "https://$tls_name:$port/api/2026-10-14/server",
  "status_url": "https://$tls_name:$port/api/2026-10-14/admin/updates",
  "health_resolve": "$listen_ip:$port",
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

install -o root -g root -m 0644 "$server_unit" /usr/lib/systemd/system/chillcenter.service
install -o root -g root -m 0644 "$update_unit" /usr/lib/systemd/system/chillcenter-update.service
install -o root -g root -m 0644 "$timer_unit" /usr/lib/systemd/system/chillcenter-update.timer
cmp -s "$server" /usr/lib/chillcenter/server/releases/"$version"-1/payload
cmp -s "$updater" /usr/lib/chillcenter/chill-update
cmp -s "$release_pub" /etc/chillcenter/release.pub
cmp -s "$server_unit" /usr/lib/systemd/system/chillcenter.service
cmp -s "$update_unit" /usr/lib/systemd/system/chillcenter-update.service
cmp -s "$timer_unit" /usr/lib/systemd/system/chillcenter-update.timer
systemd-analyze verify /usr/lib/systemd/system/chillcenter.service /usr/lib/systemd/system/chillcenter-update.service /usr/lib/systemd/system/chillcenter-update.timer
systemctl daemon-reload
systemctl enable --now chillcenter.service
systemctl is-active --quiet chillcenter.service

for _ in $(seq 1 30); do [[ -f /var/lib/chillcenter/data/tls/certificate.pem ]] && break; sleep 1; done
[[ -f /var/lib/chillcenter/data/tls/certificate.pem ]] || { echo 'Server did not produce its TLS certificate.' >&2; exit 1; }
install -o root -g root -m 0644 /var/lib/chillcenter/data/tls/certificate.pem /etc/chillcenter/health-ca.pem

curl --fail --silent --show-error --cacert /etc/chillcenter/health-ca.pem --resolve "$tls_name:$port:$listen_ip" "https://$tls_name:$port/api/2026-10-14/server" >/dev/null
/usr/lib/chillcenter/chill-update --config /etc/chillcenter/updates.json inspect >/dev/null
systemctl enable --now chillcenter-update.timer
echo 'Chill Center installed as a non-root service. Library and mount configuration were not modified.'
