#!/usr/bin/env bash
# Review-only operator script (candidate V8 = V7 `b61b9fd6…` approved by review n°3, with the previously hardcoded old-binary hash
# made a mandatory parameter OLD_BINARY_SHA256; supersedes V6 `ba9777cd…` refused by independent review n°2 (N1 SIGPIPE) and V5
# `8d47e9df…` refused by review n°1; derived from the independently approved V4 `55c6be48…`).
# DO NOT EXECUTE before an independent reviewer has approved THESE EXACT BYTES (sha256 of this file)
# and the user has given explicit go for the Pi. Runs as root on the Pi, fed through stdin:
#   ssh <strict batch control connection> 'sudo -n env OLD_BINARY_SHA256=… NEW_BINARY_SHA256=… … /bin/bash -s --' < pi-update-web-operator.sh
# (sudoers must allow `env`: check beforehand with `sudo -n env true`; see docs/deployment/WEB-CLIENT-PI.md §5).
#
# Authority (nothing else): stop the service, private offline backup, replace the server binary,
# install /usr/lib/chillcenter/web (read-only static files), restart, verify over strict TLS.
# Not touched: configuration, systemd unit, NAS mount, TLS material, identity, profiles, databases.
# Accepted residual risk (review S6): the incoming archive in /var/tmp is read several times; its only possible
# writer is a root-equivalent operator, so no privilege boundary is crossed.
set -Eeuo pipefail
# A closed stdout/stderr (lost SSH session) must never kill the script, least of all its recovery (review 2, N1).
trap '' PIPE
# Name the failing command: the recovery output alone does not say why a deployment was rolled back (review 2, N5).
trap 'echo "ERR line $LINENO: $BASH_COMMAND" >&2' ERR
service='chillcenter'; binary='/usr/lib/chillcenter/chill-server'; web_dir='/usr/lib/chillcenter/web'
incoming_bin='/var/tmp/chill-server-web.new'; incoming_web='/var/tmp/chillcenter-web.tar'
rollback_dir='/root/chillcenter-rollback'
backup_root='/var/lib/chillcenter/data/backups'; config='/etc/chillcenter/server.json'; unit='/usr/lib/systemd/system/chillcenter.service'
max_web_bytes=8388608; min_free_kb=65536

parse_backup_path() {
  local output="$1" prefix='Sauvegarde terminée : ' path
  [[ "$output" == "$prefix"* ]] || return 1
  path="${output#"$prefix"}"
  [[ "$path" =~ ^/var/lib/chillcenter/data/backups/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]] || return 1
  printf '%s\n' "$path"
}
# Every archive member must be a plain file or directory strictly inside web/.
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
must_fail() { if "$@" >/dev/null 2>&1; then echo "SELF-TEST FAILED (accepted what must be refused): $*" >&2; exit 1; fi; }  # `! cmd` never trips errexit
if [[ "${1:-}" == '--self-test' ]]; then
  good='Sauvegarde terminée : /var/lib/chillcenter/data/backups/ba950d51-fc88-49ee-b801-4641fac531ff'
  [[ "$(parse_backup_path "$good")" == '/var/lib/chillcenter/data/backups/ba950d51-fc88-49ee-b801-4641fac531ff' ]]
  must_fail parse_backup_path 'Sauvegarde terminée : /tmp/not-allowed'
  must_fail parse_backup_path 'backup completed'
  validate_archive_listing $'web/\nweb/index.html\nweb/vendor/mpeg4/mpeg4-decoder.wasm' $'drwxr-xr-x a\n-rw-r--r-- b\n-rw-r--r-- c'
  must_fail validate_archive_listing $'web/index.html\n../etc/passwd' $'-rw-r--r-- a\n-rw-r--r-- b'
  must_fail validate_archive_listing $'/etc/passwd' $'-rw-r--r-- a'
  must_fail validate_archive_listing $'web/link' $'lrwxrwxrwx a'
  must_fail validate_archive_listing $'web/hard' $'hrw-r--r-- a'
  must_fail validate_archive_listing $'other/file' $'-rw-r--r-- a'
  must_fail validate_archive_listing $'web' $'-rw-r--r-- a'
  must_fail validate_archive_listing $'web/a\nb/c' $'-rw-r--r-- a'
  must_fail validate_archive_listing '' ''
  # A pipe whose reader quits early must not break the checks (review S1): capture, then match.
  page="$(printf '%s' "$(head -c 40000 /dev/zero | tr '\0' 'x')<title>Chill Center</title>")"; [[ "$page" == *'<title>Chill Center</title>'* ]]
  printf 'parser_self_test=PASS\n'; exit 0
fi

: "${OLD_BINARY_SHA256:?hash of the binary currently installed}" "${NEW_BINARY_SHA256:?}" "${WEB_ARCHIVE_SHA256:?}" "${CHILLCENTER_TLS_CERT_PATH:?set absolute public certificate path}" "${CHILLCENTER_BASE_URL:?e.g. https://192.168.1.42:8443}"
[[ "$OLD_BINARY_SHA256" =~ ^[0-9a-f]{64}$ && "$NEW_BINARY_SHA256" =~ ^[0-9a-f]{64}$ && "$WEB_ARCHIVE_SHA256" =~ ^[0-9a-f]{64}$ && "$NEW_BINARY_SHA256" != "$OLD_BINARY_SHA256" ]] || { echo 'invalid expected hashes' >&2; exit 2; }
[[ "$CHILLCENTER_BASE_URL" =~ ^https://([0-9]{1,3}\.){3}[0-9]{1,3}:[0-9]{1,5}$ ]] || { echo 'invalid base url' >&2; exit 2; }
new_expected="$NEW_BINARY_SHA256"
old_expected="$OLD_BINARY_SHA256"
# Rollback names derive from the validated old hash (hex only, so no path surprise).
rollback="$rollback_dir/chill-server-${old_expected:0:12}"; rollback_tmp_prefix="$rollback_dir/.chill-server-${old_expected:0:12}."
[[ "$(id -u)" == '0' ]] || { echo 'must run as root' >&2; exit 2; }
[[ "$CHILLCENTER_TLS_CERT_PATH" == /* && -f "$CHILLCENTER_TLS_CERT_PATH" && ! -L "$CHILLCENTER_TLS_CERT_PATH" ]] || { echo 'invalid public certificate path' >&2; exit 2; }
for path in /usr /usr/lib /usr/lib/chillcenter "$binary" "$web_dir" /etc /etc/chillcenter "$config" /root "$incoming_bin" "$incoming_web" /var/tmp; do [[ ! -L "$path" ]] || { echo "symlink refused: $path" >&2; exit 2; }; done
for tool in curl ss openssl runuser seq install mktemp sha256sum tar find du stat df awk; do command -v "$tool" >/dev/null || { echo "required tool missing: $tool" >&2; exit 2; }; done
[[ -f "$incoming_bin" && "$(sha256sum "$incoming_bin" | awk '{print $1}')" == "$new_expected" ]] || { echo 'incoming binary invalid' >&2; exit 2; }
[[ -f "$incoming_web" && "$(sha256sum "$incoming_web" | awk '{print $1}')" == "$WEB_ARCHIVE_SHA256" ]] || { echo 'incoming web archive invalid' >&2; exit 2; }
[[ "$(sha256sum "$binary" | awk '{print $1}')" == "$old_expected" ]] || { echo 'installed binary is not the expected one' >&2; exit 2; }
[[ "$(id -u chillcenter)" == '997' ]] || { echo 'unexpected chillcenter uid' >&2; exit 2; }
[[ ! -e "$web_dir" || ( -d "$web_dir" && ! -L "$web_dir" ) ]] || { echo 'unexpected web path' >&2; exit 2; }
validate_archive_listing "$(tar -tf "$incoming_web")" "$(tar -tvf "$incoming_web")" || { echo 'web archive content refused' >&2; exit 2; }
# Rollback location, checked before anything is stopped (review S4): the directory must be private, and an
# existing copy of the old binary is reused only if it is byte-identical.
[[ ! -L "$rollback_dir" && -d "$rollback_dir" && "$(stat -c '%a:%u:%g' "$rollback_dir")" == '700:0:0' ]] || { echo 'rollback directory missing or unsafe' >&2; exit 2; }
rollback_exists=0
if [[ -e "$rollback" || -L "$rollback" ]]; then
  [[ -f "$rollback" && ! -L "$rollback" && "$(sha256sum "$rollback" | awk '{print $1}')" == "$old_expected" ]] || { echo 'rollback file exists with unexpected content' >&2; exit 2; }
  rollback_exists=1
fi
free_kb() { df -Pk "$1" | awk 'NR==2{print $4}'; }
[[ "$(free_kb /usr/lib/chillcenter)" -ge "$min_free_kb" && "$(free_kb /var/lib/chillcenter/data)" -ge "$min_free_kb" && "$(free_kb "$rollback_dir")" -ge "$min_free_kb" ]] || { echo 'not enough free disk space' >&2; exit 2; }

# -q ignores ~/.curlrc and --noproxy ignores proxy variables of the root environment (review 2, N7).
curl_tls() { curl -q -sS --noproxy '*' --max-time 10 --cacert "$CHILLCENTER_TLS_CERT_PATH" "$@" </dev/null; }
# The url must point at a port the chill-server process is really listening on, not at another LAN host with the same certificate.
listening="$(ss -Hltnp </dev/null 2>/dev/null | grep -F chill-server || true)"
[[ "$listening" == *":${CHILLCENTER_BASE_URL##*:} "* ]] || { echo 'chill-server is not listening on the port of the given url' >&2; exit 2; }
# Strict-TLS probe of the CURRENT service before anything is stopped (review S3): wrong SAN, port or CA fails here, not after the swap.
[[ "$(curl_tls -o /dev/null -w '%{http_code}' "$CHILLCENTER_BASE_URL/api/2026-10-14/server")" == '200' ]] || { echo 'current service does not answer 200 over strict TLS at the given url' >&2; exit 2; }

config_before="$(sha256sum "$config" | awk '{print $1}')"; unit_before="$(sha256sum "$unit" | awk '{print $1}')"
spki_before="$(openssl x509 -in "$CHILLCENTER_TLS_CERT_PATH" -pubkey -noout | openssl pkey -pubin -outform DER | sha256sum | awk '{print $1}')"
completed=0; stop_attempted=0; root_stage=''; web_stage=''; rollback_tmp=''; restore_stage=''; web_swapped=0; web_moved=0; web_prev=''
cleanup_owned_file() { local candidate="$1" prefix="$2"; [[ -n "$candidate" && "$candidate" == "$prefix"* && -f "$candidate" && ! -L "$candidate" ]] && rm -f -- "$candidate" || true; }
recover_on_failure() {
  local status=$? current_hash restored=1
  # Recovery must run to the end whatever fails inside it (review S2): no errexit, no interruption by signals.
  set +e
  trap '' HUP INT TERM   # ignored: neither this handler nor the install/mv it spawns can be interrupted half way (review 2, N2)
  trap - EXIT ERR
  if (( completed == 0 && stop_attempted == 1 )); then
    echo "RECOVERY: restoring the previous state (exit status $status)" >&2
    if systemctl is-active --quiet "$service"; then systemctl stop "$service"; fi
    if systemctl is-active --quiet "$service"; then echo 'RECOVERY: the service could not be stopped; binary and web directory left untouched' >&2; else
      current_hash="$(sha256sum "$binary" | awk '{print $1}')"
      if [[ "$current_hash" == "$new_expected" ]]; then
        restored=0
        if [[ -f "$rollback" && ! -L "$rollback" && "$(sha256sum "$rollback" | awk '{print $1}')" == "$old_expected" ]]; then
          restore_stage="$(mktemp /usr/lib/chillcenter/.chill-server-rollback.XXXXXX)" \
            && install -o root -g root -m 0755 "$rollback" "$restore_stage" \
            && [[ "$(sha256sum "$restore_stage" | awk '{print $1}')" == "$old_expected" ]] \
            && mv -f "$restore_stage" "$binary" && restore_stage='' && restored=1
        fi
        (( restored == 1 )) || echo 'RECOVERY: the old binary could NOT be restored; the new binary and web directory are left in place' >&2
      fi
      # Put the web directory back exactly as found (absent, or the previous directory), only when the binary is consistent.
      if (( restored == 1 )); then
        if (( web_swapped == 1 )); then rm -rf --one-file-system -- "$web_dir"; fi
        if (( web_moved == 1 )) && [[ -n "$web_prev" && -d "$web_prev" && ! -L "$web_prev" ]]; then mv -T "$web_prev" "$web_dir"; fi
      fi
    fi
    # Start only after a stop was attempted; an early precondition failure is inert.
    if ! systemctl is-active --quiet "$service"; then systemctl start "$service"; fi
    if systemctl is-active --quiet "$service"; then
      echo 'RECOVERY: service is active' >&2
      sleep 2; if curl_tls -o /dev/null "$CHILLCENTER_BASE_URL/api/2026-10-14/server" 2>/dev/null; then echo 'RECOVERY: service answers over strict TLS' >&2; else echo 'RECOVERY: service active but NOT answering over strict TLS' >&2; fi
    else echo 'RECOVERY: SERVICE IS NOT ACTIVE' >&2; fi
  fi
  cleanup_owned_file "$root_stage" '/usr/lib/chillcenter/.chill-server-web.'
  cleanup_owned_file "$rollback_tmp" "$rollback_tmp_prefix"
  cleanup_owned_file "$restore_stage" '/usr/lib/chillcenter/.chill-server-rollback.'
  [[ -n "$web_stage" && "$web_stage" == /usr/lib/chillcenter/.web-new.* && -d "$web_stage" && ! -L "$web_stage" ]] && rm -rf --one-file-system -- "$web_stage"
  exit "$status"
}
trap recover_on_failure EXIT
trap 'exit 129' HUP; trap 'exit 130' INT; trap 'exit 143' TERM

# Stage everything privately and verify it before the service is touched.
root_stage="$(mktemp /usr/lib/chillcenter/.chill-server-web.XXXXXX)"
install -o root -g root -m 0755 "$incoming_bin" "$root_stage"
[[ -f "$root_stage" && ! -L "$root_stage" && "$(stat -c '%a:%u:%g' "$root_stage")" == '755:0:0' ]]
[[ "$(sha256sum "$root_stage" | awk '{print $1}')" == "$new_expected" ]]
web_stage="$(mktemp -d /usr/lib/chillcenter/.web-new.XXXXXX)"
tar -xf "$incoming_web" --no-same-owner --no-same-permissions -C "$web_stage"
[[ -d "$web_stage/web" && ! -L "$web_stage/web" ]]
[[ -z "$(find "$web_stage/web" ! -type f ! -type d -print -quit)" ]] || { echo 'special file in web archive' >&2; exit 2; }
[[ "$(du -sb "$web_stage/web" | cut -f1)" -le "$max_web_bytes" ]] || { echo 'web tree too large' >&2; exit 2; }
chown -R root:root "$web_stage/web"; find "$web_stage/web" -type d -exec chmod 0755 {} +; find "$web_stage/web" -type f -exec chmod 0644 {} +
[[ -f "$web_stage/web/SHA256SUMS" ]] || { echo 'SHA256SUMS missing' >&2; exit 2; }
( cd "$web_stage/web" && sha256sum -c --quiet --strict SHA256SUMS )
# SHA256SUMS must cover every file (it cannot list itself); only the top-level one is exempt (review S7).
listed="$(cd "$web_stage/web" && awk '{print $2}' SHA256SUMS | LC_ALL=C sort)"
present="$(cd "$web_stage/web" && find . -type f ! -path ./SHA256SUMS -printf '%P\n' | LC_ALL=C sort)"
[[ "$listed" == "$present" ]] || { echo 'SHA256SUMS does not cover the tree' >&2; exit 2; }
[[ -f "$web_stage/web/index.html" ]]

systemctl is-active --quiet "$service"
# Fail closed (review S8): if the socket listing itself fails, nothing is stopped.
sockets="$(ss -Htnp state established 2>/dev/null </dev/null)" || { echo 'ss failed; refusing stop' >&2; exit 2; }
connections="$(grep -F chill-server <<<"$sockets" || true)"
[[ -z "$connections" ]] || { echo 'active established client connection; refusing stop' >&2; exit 42; }
stop_attempted=1
systemctl stop "$service"
if systemctl is-active --quiet "$service"; then echo 'service still active after stop' >&2; exit 1; fi

backup_output="$(runuser -u chillcenter -- /bin/bash -c "cd /usr/lib/chillcenter && exec $binary --backup --config $config" </dev/null)"
backup_path="$(parse_backup_path "$backup_output")"
[[ -d "$backup_path" && ! -L "$backup_path" ]]
[[ "$(stat -c '%a:%u' "$backup_root")" == '700:997' && "$(stat -c '%a:%u' "$backup_path")" == '700:997' ]]
manifest="$backup_path/manifest.json"; [[ -f "$manifest" && ! -L "$manifest" ]]
manifest_mtime="$(stat -c '%Y' "$manifest")"; latest_mtime="$(find "$backup_path" -xdev -type f -printf '%T@\n' | LC_ALL=C sort -n | tail -n 1 | cut -d. -f1)"; [[ "$manifest_mtime" -ge "$latest_mtime" ]]

# Binary rollback copy (the directory already holds the R7 copy; the target file is created without overwriting).
if (( rollback_exists == 0 )); then
  [[ ! -e "$rollback" && ! -L "$rollback" ]]
  rollback_tmp="$(mktemp "${rollback_tmp_prefix}XXXXXX")"
  install -o root -g root -m 0755 "$binary" "$rollback_tmp"
  [[ -f "$rollback_tmp" && ! -L "$rollback_tmp" && "$(sha256sum "$rollback_tmp" | awk '{print $1}')" == "$old_expected" ]]
  ln "$rollback_tmp" "$rollback"
  rm -f -- "$rollback_tmp"; rollback_tmp=''
fi
[[ -f "$rollback" && ! -L "$rollback" && "$(sha256sum "$rollback" | awk '{print $1}')" == "$old_expected" ]]

# Install: previous web directory (if any) is kept in the rollback directory, then binary, then web.
if [[ -e "$web_dir" ]]; then
  web_prev="$rollback_dir/web-$(date -u +%Y%m%dT%H%M%SZ)"; [[ ! -e "$web_prev" ]]
  mv -T "$web_dir" "$web_prev"; web_moved=1   # flag right after the move (review S5)
fi
mv -f "$root_stage" "$binary"; root_stage=''
[[ "$(sha256sum "$binary" | awk '{print $1}')" == "$new_expected" ]]
mv -T "$web_stage/web" "$web_dir"; web_swapped=1; rmdir "$web_stage"; web_stage=''
systemctl start "$service"; systemctl is-active --quiet "$service"

# Verification over strict TLS (never -k); any failure triggers the automatic restoration above.
ready=0; for _ in $(seq 1 30); do if curl_tls -o /dev/null "$CHILLCENTER_BASE_URL/api/2026-10-14/server" 2>/dev/null; then ready=1; break; fi; sleep 1; done
[[ "$ready" == 1 ]] || { echo 'service not answering' >&2; exit 3; }
[[ "$(curl_tls -o /dev/null -w '%{http_code}' "$CHILLCENTER_BASE_URL/api/2026-10-14/server")" == '200' ]]                                  # native clients keep working
[[ "$(curl_tls -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: same-origin' "$CHILLCENTER_BASE_URL/api/2026-10-14/server")" == '200' ]]   # the web client
[[ "$(curl_tls -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: cross-site' "$CHILLCENTER_BASE_URL/api/2026-10-14/server")" == '403' ]]   # other sites stay refused
[[ "$(curl_tls -o /dev/null -w '%{http_code}' -H 'Origin: https://evil.example' "$CHILLCENTER_BASE_URL/api/2026-10-14/server")" == '403' ]]
page="$(curl_tls "$CHILLCENTER_BASE_URL/")"      # captured first: no pipe whose reader can quit early (review S1)
[[ "$page" == *'<title>Chill Center</title>'* ]]
[[ "$(curl_tls -o /dev/null -w '%{content_type}' "$CHILLCENTER_BASE_URL/vendor/mpeg4/mpeg4-decoder.wasm")" == 'application/wasm' ]]
[[ "$(curl_tls -o /dev/null -w '%{http_code}' "$CHILLCENTER_BASE_URL/.env")" == '404' ]]
[[ "$(curl_tls --path-as-is -o /dev/null -w '%{http_code}' "$CHILLCENTER_BASE_URL/../etc/passwd")" != '200' ]]

[[ "$(sha256sum "$config" | awk '{print $1}')" == "$config_before" ]]
[[ "$(sha256sum "$unit" | awk '{print $1}')" == "$unit_before" ]]
spki_after="$(openssl x509 -in "$CHILLCENTER_TLS_CERT_PATH" -pubkey -noout | openssl pkey -pubin -outform DER | sha256sum | awk '{print $1}')"; [[ "$spki_after" == "$spki_before" ]]
completed=1; trap - EXIT HUP INT TERM
printf 'deployed_utc=%s\nservice_pid=%s\ninstalled_hash=%s\nrollback_hash=%s\nweb_archive_hash=%s\nweb_previous=%s\nbackup_path=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(systemctl show "$service" -p MainPID --value)" "$new_expected" "$old_expected" "$WEB_ARCHIVE_SHA256" "${web_prev:-none}" "$backup_path"
