#!/usr/bin/env bash
# Review-only operator script (ADR-0019/0020): replace ONLY the static web client (/usr/lib/chillcenter/web) with a new archive.
# The service is neither stopped nor restarted (it reads the files per request), the binary, the unit, the configuration, the
# certificate and the data are not touched. The previous directory is kept in /root/chillcenter-rollback and put back automatically if
# any verification fails. DO NOT EXECUTE before an independent reviewer has approved THESE EXACT BYTES (sha256 of this file) and the
# user has given explicit go for the Pi. Runs as root on the Pi, fed through stdin:
#   ssh <master> 'sudo -n env INSTALLED_BINARY_SHA256=… WEB_ARCHIVE_SHA256=… EXPECTED_BUILD_ID=… CHILLCENTER_TLS_CERT_PATH=… CHILLCENTER_BASE_URL=https://<ip>:8443 [CHILLCENTER_HTTP_URL=http://<ip>:8080] /bin/bash -s --' < pi-update-web-assets-operator.sh
#   self-test: ssh <master> 'sudo -n /bin/bash -s -- --self-test' < pi-update-web-assets-operator.sh
set -Eeuo pipefail
trap '' PIPE
trap 'echo "ERR line $LINENO: $BASH_COMMAND" >&2' ERR
service='chillcenter'; binary='/usr/lib/chillcenter/chill-server'; web_dir='/usr/lib/chillcenter/web'
incoming_web='/var/tmp/chillcenter-web.tar'; rollback_dir='/root/chillcenter-rollback'
config='/etc/chillcenter/server.json'; unit='/usr/lib/systemd/system/chillcenter.service'
max_web_bytes=16777216; min_free_kb=65536

# Every archive member must be a plain file or directory strictly inside web/ (same rule as the full deployment script).
validate_archive_listing() {
  local names="$1" detail="$2" name
  [[ -n "$names" ]] || return 1
  while IFS= read -r name; do
    [[ "$name" == web/* || "$name" == web/ ]] || return 1
    [[ "$name" != *'..'* && "$name" != /* ]] || return 1
  done <<<"$names"
  while IFS= read -r name; do
    [[ "${name:0:1}" == '-' || "${name:0:1}" == 'd' ]] || return 1
  done <<<"$detail"
}
build_id_of() { grep -o '"build": *"[0-9a-f]\{12\}"' | head -n 1 | grep -o '[0-9a-f]\{12\}'; }
must_fail() { if "$@" >/dev/null 2>&1; then echo "SELF-TEST FAILED (accepted what must be refused): $*" >&2; exit 1; fi; }
if [[ "${1:-}" == '--self-test' ]]; then
  validate_archive_listing $'web/\nweb/index.html\nweb/vendor/audio/audio-transcoder.wasm' $'drwxr-xr-x a\n-rw-r--r-- b\n-rw-r--r-- c'
  must_fail validate_archive_listing $'web/index.html\n../etc/passwd' $'-rw-r--r-- a\n-rw-r--r-- b'
  must_fail validate_archive_listing $'/etc/passwd' $'-rw-r--r-- a'
  must_fail validate_archive_listing $'web/link' $'lrwxrwxrwx a'
  must_fail validate_archive_listing $'web/hard' $'hrw-r--r-- a'
  must_fail validate_archive_listing $'other/file' $'-rw-r--r-- a'
  must_fail validate_archive_listing '' ''
  [[ "$(printf '{\n  "build": "0123456789ab",\n  "files": 3\n}\n' | build_id_of)" == '0123456789ab' ]]
  [[ -z "$(printf '{ "build": "xyz" }' | build_id_of || true)" ]]
  printf 'web_assets_self_test=PASS\n'; exit 0
fi

: "${INSTALLED_BINARY_SHA256:?}" "${WEB_ARCHIVE_SHA256:?}" "${EXPECTED_BUILD_ID:?}" "${CHILLCENTER_TLS_CERT_PATH:?absolute public certificate path}" "${CHILLCENTER_BASE_URL:?e.g. https://192.168.1.42:8443}"
[[ "$INSTALLED_BINARY_SHA256" =~ ^[0-9a-f]{64}$ && "$WEB_ARCHIVE_SHA256" =~ ^[0-9a-f]{64}$ && "$EXPECTED_BUILD_ID" =~ ^[0-9a-f]{12}$ ]] || { echo 'invalid expected values' >&2; exit 2; }
[[ "$CHILLCENTER_BASE_URL" =~ ^https://([0-9]{1,3}\.){3}[0-9]{1,3}:[0-9]{1,5}$ ]] || { echo 'invalid base url' >&2; exit 2; }
http_url="${CHILLCENTER_HTTP_URL:-}"
[[ -z "$http_url" || "$http_url" =~ ^http://([0-9]{1,3}\.){3}[0-9]{1,3}(:[0-9]{1,5})?$ ]] || { echo 'invalid http url' >&2; exit 2; }
[[ "$(id -u)" == '0' ]] || { echo 'must run as root' >&2; exit 2; }
[[ "$CHILLCENTER_TLS_CERT_PATH" == /* && -f "$CHILLCENTER_TLS_CERT_PATH" && ! -L "$CHILLCENTER_TLS_CERT_PATH" ]] || { echo 'invalid public certificate path' >&2; exit 2; }
for path in /usr /usr/lib /usr/lib/chillcenter "$binary" "$web_dir" /etc /etc/chillcenter "$config" /root "$rollback_dir" "$incoming_web" /var/tmp; do [[ ! -L "$path" ]] || { echo "symlink refused: $path" >&2; exit 2; }; done
for tool in curl ss openssl seq install mktemp sha256sum tar find du stat df awk grep date systemctl; do command -v "$tool" >/dev/null || { echo "required tool missing: $tool" >&2; exit 2; }; done
[[ "$(sha256sum "$binary" | awk '{print $1}')" == "$INSTALLED_BINARY_SHA256" ]] || { echo 'installed binary is not the expected one' >&2; exit 2; }
[[ -f "$incoming_web" && ! -L "$incoming_web" && "$(sha256sum "$incoming_web" | awk '{print $1}')" == "$WEB_ARCHIVE_SHA256" ]] || { echo 'incoming web archive invalid' >&2; exit 2; }
[[ -d "$web_dir" && ! -L "$web_dir" ]] || { echo 'no web directory to replace (first install is done by the full deployment script)' >&2; exit 2; }
[[ "$(systemctl is-active "$service" || true)" == 'active' ]] || { echo 'service is not active' >&2; exit 2; }
validate_archive_listing "$(tar -tf "$incoming_web")" "$(tar -tvf "$incoming_web")" || { echo 'web archive content refused' >&2; exit 2; }
[[ -d "$rollback_dir" && "$(stat -c '%a:%u:%g' "$rollback_dir")" == '700:0:0' ]] || { echo 'rollback directory missing or unsafe' >&2; exit 2; }
free_kb() { df -Pk "$1" | awk 'NR==2{print $4}'; }
[[ "$(free_kb /usr/lib/chillcenter)" -ge "$min_free_kb" && "$(free_kb "$rollback_dir")" -ge "$min_free_kb" ]] || { echo 'not enough free disk space' >&2; exit 2; }

curl_tls() { curl -q -sS --noproxy '*' --max-time 10 --cacert "$CHILLCENTER_TLS_CERT_PATH" "$@" </dev/null; }
curl_plain() { curl -q -sS --noproxy '*' --max-time 10 "$@" </dev/null; }
listening="$(ss -Hltnp </dev/null 2>/dev/null | grep -F chill-server || true)"
[[ "$listening" == *":${CHILLCENTER_BASE_URL##*:} "* ]] || { echo 'chill-server is not listening on the port of the given url' >&2; exit 2; }
[[ "$(curl_tls -o /dev/null -w '%{http_code}' "$CHILLCENTER_BASE_URL/api/2026-10-14/server")" == '200' ]] || { echo 'current service does not answer 200 over strict TLS at the given url' >&2; exit 2; }
previous_build="$(curl_tls "$CHILLCENTER_BASE_URL/build.json" | build_id_of || true)"
[[ "$previous_build" != "$EXPECTED_BUILD_ID" ]] || { echo "build $EXPECTED_BUILD_ID is already installed" >&2; exit 2; }

config_before="$(sha256sum "$config" | awk '{print $1}')"; unit_before="$(sha256sum "$unit" | awk '{print $1}')"
spki_before="$(openssl x509 -in "$CHILLCENTER_TLS_CERT_PATH" -pubkey -noout | openssl pkey -pubin -outform DER | sha256sum | awk '{print $1}')"
service_pid="$(systemctl show "$service" -p MainPID --value)"
completed=0; web_stage=''; web_swapped=0; web_moved=0; web_prev=''
recover_on_failure() {
  local status=$?
  # Recovery first, messages after, and nothing in it can stop it: no errexit, no signals, no dependence on a live stderr (review 7).
  set +e
  trap '' HUP INT TERM
  trap - EXIT ERR
  if (( completed == 0 )); then
    if (( web_swapped == 1 )); then rm -rf --one-file-system -- "$web_dir"; fi
    if (( web_moved == 1 )) && [[ -n "$web_prev" && -d "$web_prev" && ! -L "$web_prev" ]]; then mv -T "$web_prev" "$web_dir"; fi
    [[ -n "$web_stage" && "$web_stage" == /usr/lib/chillcenter/.web-new.* && -d "$web_stage" && ! -L "$web_stage" ]] && rm -rf --one-file-system -- "$web_stage"
    echo "RECOVERY: previous web directory restored (exit status $status)" >&2
    [[ -d "$web_dir" && -f "$web_dir/index.html" ]] && echo 'RECOVERY: web directory present' >&2 || echo 'RECOVERY: WEB DIRECTORY MISSING' >&2
    curl_tls -o /dev/null "$CHILLCENTER_BASE_URL/api/2026-10-14/server" 2>/dev/null && echo 'RECOVERY: service answers over strict TLS' >&2
  fi
  exit "$status"
}
trap recover_on_failure EXIT
trap 'exit 129' HUP; trap 'exit 130' INT; trap 'exit 143' TERM

# Stage and verify privately before anything is replaced.
web_stage="$(mktemp -d /usr/lib/chillcenter/.web-new.XXXXXX)"
tar -xf "$incoming_web" --no-same-owner --no-same-permissions -C "$web_stage"
[[ -d "$web_stage/web" && ! -L "$web_stage/web" ]]
[[ -z "$(find "$web_stage/web" ! -type f ! -type d -print -quit)" ]] || { echo 'special file in web archive' >&2; exit 2; }
[[ "$(du -sb "$web_stage/web" | cut -f1)" -le "$max_web_bytes" ]] || { echo 'web tree too large' >&2; exit 2; }
chown -R root:root "$web_stage/web"; find "$web_stage/web" -type d -exec chmod 0755 {} +; find "$web_stage/web" -type f -exec chmod 0644 {} +
[[ -f "$web_stage/web/SHA256SUMS" ]] || { echo 'SHA256SUMS missing' >&2; exit 2; }
( cd "$web_stage/web" && sha256sum -c --quiet --strict SHA256SUMS )
listed="$(cd "$web_stage/web" && awk '{print $2}' SHA256SUMS | LC_ALL=C sort)"
present="$(cd "$web_stage/web" && find . -type f ! -path ./SHA256SUMS -printf '%P\n' | LC_ALL=C sort)"
[[ "$listed" == "$present" ]] || { echo 'SHA256SUMS does not cover the tree' >&2; exit 2; }
[[ -f "$web_stage/web/index.html" && -f "$web_stage/web/build.json" ]]
[[ "$(build_id_of < "$web_stage/web/build.json")" == "$EXPECTED_BUILD_ID" ]] || { echo 'archive build id is not the expected one' >&2; exit 2; }

# Swap: the previous directory goes to the rollback directory first; the two renames are the only moment without a web directory.
web_prev="$rollback_dir/web-$(date -u +%Y%m%dT%H%M%SZ)"; [[ ! -e "$web_prev" ]]
mv -T "$web_dir" "$web_prev"; web_moved=1
mv -T "$web_stage/web" "$web_dir"; web_swapped=1; rmdir "$web_stage"; web_stage=''

# Verification over strict TLS (never -k) and, when given, over the plain-HTTP address; any failure restores the previous directory.
[[ "$(systemctl is-active "$service")" == 'active' && "$(systemctl show "$service" -p MainPID --value)" == "$service_pid" ]]   # the service was never touched
for base in "$CHILLCENTER_BASE_URL" ${http_url:+"$http_url"}; do
  if [[ "$base" == https://* ]]; then fetch=curl_tls; else fetch=curl_plain; fi
  [[ "$("$fetch" "$base/build.json" | build_id_of)" == "$EXPECTED_BUILD_ID" ]]
  page="$("$fetch" "$base/")"; [[ "$page" == *'<title>Chill Center</title>'* && "$page" == *"?v=$EXPECTED_BUILD_ID"* ]]
  [[ "$("$fetch" -o /dev/null -w '%{content_type}' "$base/vendor/mpeg4/mpeg4-decoder.wasm")" == 'application/wasm' ]]
  [[ "$("$fetch" -o /dev/null -w '%{http_code}' "$base/api/2026-10-14/server")" == '200' ]]
  [[ "$("$fetch" -o /dev/null -w '%{http_code}' "$base/.env")" == '404' ]]
done
[[ "$(curl_tls -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: cross-site' "$CHILLCENTER_BASE_URL/api/2026-10-14/server")" == '403' ]]
[[ "$(curl_tls --path-as-is -o /dev/null -w '%{http_code}' "$CHILLCENTER_BASE_URL/../etc/passwd")" != '200' ]]
[[ "$(sha256sum "$binary" | awk '{print $1}')" == "$INSTALLED_BINARY_SHA256" ]]
[[ "$(sha256sum "$config" | awk '{print $1}')" == "$config_before" ]]
[[ "$(sha256sum "$unit" | awk '{print $1}')" == "$unit_before" ]]
spki_after="$(openssl x509 -in "$CHILLCENTER_TLS_CERT_PATH" -pubkey -noout | openssl pkey -pubin -outform DER | sha256sum | awk '{print $1}')"; [[ "$spki_after" == "$spki_before" ]]
completed=1; trap - EXIT HUP INT TERM
printf 'updated_utc=%s\nservice_pid=%s\nprevious_build=%s\ninstalled_build=%s\nweb_archive_hash=%s\nweb_previous=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$service_pid" "$previous_build" "$EXPECTED_BUILD_ID" "$WEB_ARCHIVE_SHA256" "$web_prev"
