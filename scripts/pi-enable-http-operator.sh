#!/usr/bin/env bash
# Review-only operator script (ADR-0018): add `"http_listen": "<LAN_IP>:8080"` to /etc/chillcenter/server.json, restart the
# service, verify, and put the previous configuration back automatically if anything fails. DO NOT EXECUTE before an independent
# reviewer has approved THESE EXACT BYTES (sha256 of this file) and the user has given explicit go for the Pi. Runs as root on the
# Pi, fed through stdin:
#   enable : ssh <master> 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 NEW_BINARY_SHA256=… CONFIG_SHA256=… CHILLCENTER_TLS_CERT_PATH=… /bin/bash -s --' < pi-enable-http-operator.sh
#   revert : ssh <master> 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 CONFIG_SHA256=<hash of the file now> PREVIOUS_CONFIG_SHA256=<hash of the ORIGINAL file> /bin/bash -s -- --revert' < pi-enable-http-operator.sh
#   check  : ssh <master> 'sudo -n /bin/bash -s -- --self-test' < pi-enable-http-operator.sh
#
# Authority (nothing else): edit /etc/chillcenter/server.json (one added line), keep a private copy of the previous file in
# /root/chillcenter-rollback, restart the `chillcenter` service. Not touched: the binary, the unit, the certificate, the data,
# the firewall. The installed binary must already be the one that understands `http_listen` (an older binary refuses the file).
set -Eeuo pipefail
trap '' PIPE
trap 'echo "ERR line $LINENO: $BASH_COMMAND" >&2' ERR
service='chillcenter'; config='/etc/chillcenter/server.json'; binary='/usr/lib/chillcenter/chill-server'
rollback_dir='/root/chillcenter-rollback'; http_port=8080; https_port=8443; max_config_bytes=65536

valid_lan_ip() {
  local ip="$1" octet
  [[ "$ip" =~ ^(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})$ ]] || return 1
  for octet in "${BASH_REMATCH[@]:1}"; do (( octet <= 255 )) || return 1; done
  [[ "${BASH_REMATCH[1]}" != 0 && "${BASH_REMATCH[1]}" != 127 && "${BASH_REMATCH[1]}" -lt 224 ]] || return 1
}
# Insert one line right after the first line that is exactly `{`; refuse anything else (the file is pinned by hash anyway).
render_config() {  # $1 = file, $2 = ip ; prints the new content
  awk -v line="  \"http_listen\": \"$2:$http_port\"," 'BEGIN{done=0} { print } !done && $0 ~ /^\{[ \t]*$/ { print line; done=1 } END{ if(!done) exit 3 }' "$1"
}
must_fail() { if "$@" >/dev/null 2>&1; then echo "SELF-TEST FAILED (accepted what must be refused): $*" >&2; exit 1; fi; }

if [[ "${1:-}" == '--self-test' ]]; then
  valid_lan_ip 192.168.1.42; valid_lan_ip 10.0.0.1
  for bad in '' '127.0.0.1' '0.0.0.0' '224.0.0.1' '192.168.1.256' '192.168.1' 'a.b.c.d' '192.168.01.42' '192.168.1.42;' $'192.168.1.42\n1.1.1.1'; do must_fail valid_lan_ip "$bad"; done
  t="$(mktemp)"; printf '{\n  "listen": "192.168.1.42:8443",\n  "data_dir": "/var/lib/chillcenter/data"\n}\n' > "$t"
  [[ "$(render_config "$t" 192.168.1.42)" == $'{\n  "http_listen": "192.168.1.42:8080",\n  "listen": "192.168.1.42:8443",\n  "data_dir": "/var/lib/chillcenter/data"\n}' ]]
  printf '"listen": "x"\n' > "$t"; must_fail render_config "$t" 192.168.1.42
  printf '{"listen": "x"}\n' > "$t"; must_fail render_config "$t" 192.168.1.42
  rm -f "$t"; printf 'enable_http_self_test=PASS\n'; exit 0
fi

mode='enable'
case "${1:-}" in '') ;; --revert) mode='revert' ;; *) echo 'unknown argument' >&2; exit 2 ;; esac
: "${CHILLCENTER_LAN_IP:?set the Pi LAN address, e.g. 192.168.1.42}" "${CONFIG_SHA256:?sha256 of the configuration file expected right now}"
valid_lan_ip "$CHILLCENTER_LAN_IP" || { echo 'invalid LAN address' >&2; exit 2; }
[[ "$CONFIG_SHA256" =~ ^[0-9a-f]{64}$ ]] || { echo 'invalid CONFIG_SHA256' >&2; exit 2; }
lan_ip="$CHILLCENTER_LAN_IP"
previous_sha="${PREVIOUS_CONFIG_SHA256:-$CONFIG_SHA256}"   # enable: the file as it is now ; revert: the original, whose private copy is restored
[[ "$previous_sha" =~ ^[0-9a-f]{64}$ ]] || { echo 'invalid PREVIOUS_CONFIG_SHA256' >&2; exit 2; }
if [[ "$mode" == 'enable' ]]; then
  : "${NEW_BINARY_SHA256:?hash of the installed binary that understands http_listen}" "${CHILLCENTER_TLS_CERT_PATH:?absolute public certificate path}"
  [[ "$NEW_BINARY_SHA256" =~ ^[0-9a-f]{64}$ ]] || { echo 'invalid NEW_BINARY_SHA256' >&2; exit 2; }
  [[ "$CHILLCENTER_TLS_CERT_PATH" == /* && -f "$CHILLCENTER_TLS_CERT_PATH" && ! -L "$CHILLCENTER_TLS_CERT_PATH" ]] || { echo 'invalid public certificate path' >&2; exit 2; }
fi
[[ "$(id -u)" == '0' ]] || { echo 'must run as root' >&2; exit 2; }
for tool in systemctl ss curl sha256sum install mktemp awk grep stat cmp diff mv rm; do command -v "$tool" >/dev/null || { echo "required tool missing: $tool" >&2; exit 2; }; done
for path in /etc /etc/chillcenter "$config" /root "$rollback_dir" "$binary"; do [[ ! -L "$path" ]] || { echo "symlink refused: $path" >&2; exit 2; }; done
[[ -f "$config" && "$(stat -c %s "$config")" -le "$max_config_bytes" ]] || { echo 'configuration missing or too large' >&2; exit 2; }
[[ "$(sha256sum "$config" | awk '{print $1}')" == "$CONFIG_SHA256" ]] || { echo 'configuration is not the expected one' >&2; exit 2; }
[[ -d "$rollback_dir" && "$(stat -c '%a:%u:%g' "$rollback_dir")" == '700:0:0' ]] || { echo 'rollback directory missing or unsafe' >&2; exit 2; }
backup="$rollback_dir/server.json.${previous_sha:0:12}"
owner_mode="$(stat -c '%u:%g:%a' "$config")"; owner_uid="${owner_mode%%:*}"; rest="${owner_mode#*:}"; owner_gid="${rest%%:*}"; file_mode="${rest#*:}"
wait_listening() {  # $1 = port ; up to 30 s
  local i; for i in $(seq 1 60); do grep -qE "^LISTEN +[0-9]+ +[0-9]+ +${lan_ip//./\\.}:$1 " <<<"$(ss -Hltn)" && return 0; sleep 0.5; done; return 1
}

if [[ "$mode" == 'revert' ]]; then
  [[ -f "$backup" && ! -L "$backup" && "$(sha256sum "$backup" | awk '{print $1}')" == "$previous_sha" ]] || { echo "no valid backup $backup" >&2; exit 2; }
  tmp="$(mktemp /etc/chillcenter/.server.json.XXXXXX)"; trap 'rm -f -- "$tmp"' EXIT
  install -o "$owner_uid" -g "$owner_gid" -m "$file_mode" "$backup" "$tmp"; mv -f -- "$tmp" "$config"
  systemctl restart "$service"; wait_listening "$https_port"
  [[ "$(systemctl is-active "$service")" == 'active' ]]
  printf 'reverted_utc=%s\nconfig_sha256=%s\nservice_active=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(sha256sum "$config" | awk '{print $1}')" "$(systemctl is-active "$service")"
  exit 0
fi

# ---- preconditions: nothing is changed before all of them hold ----------------------------------------------------------
[[ "$(sha256sum "$binary" | awk '{print $1}')" == "$NEW_BINARY_SHA256" ]] || { echo 'installed binary is not the expected one' >&2; exit 2; }
! grep -q 'http_listen' "$config" || { echo 'http_listen already present' >&2; exit 2; }
[[ "$(systemctl is-active "$service" || true)" == 'active' ]] || { echo 'service is not active' >&2; exit 2; }
listening="$(ss -Hltn)"
grep -qE "^LISTEN +[0-9]+ +[0-9]+ +${lan_ip//./\\.}:${https_port} " <<<"$listening" || { echo "service does not listen on $lan_ip:$https_port" >&2; exit 2; }
if grep -qE "[:.]${http_port} " <<<"$listening"; then echo "something already uses port $http_port" >&2; exit 2; fi
established="$(ss -Htnp state established 2>/dev/null | grep -cF chill-server || true)"
[[ "$established" == 0 ]] || { echo "client connections are established ($established): not restarting now" >&2; exit 42; }
[[ ! -e "$backup" && ! -L "$backup" ]] || { echo "backup already exists: $backup" >&2; exit 2; }
echo "preconditions_ok lan=$lan_ip config=${CONFIG_SHA256:0:12} owner=$owner_uid:$owner_gid mode=$file_mode"

candidate="$(mktemp /etc/chillcenter/.server.json.XXXXXX)"; cleanup() { rm -f -- "$candidate" "$candidate.final" "$candidate.restore"; }; trap cleanup EXIT
render_config "$config" "$lan_ip" > "$candidate"
added="$(diff "$config" "$candidate" | grep -c '^>' || true)"; removed="$(diff "$config" "$candidate" | grep -c '^<' || true)"
[[ "$added" == 1 && "$removed" == 0 ]] || { echo "unexpected configuration diff ($added added, $removed removed)" >&2; exit 2; }
if command -v python3 >/dev/null; then python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$candidate"; fi
install -o 0 -g 0 -m 0600 "$config" "$backup"                          # private copy of the previous file, never overwritten
[[ "$(sha256sum "$backup" | awk '{print $1}')" == "$previous_sha" ]]    # the copy itself is checked against the pinned hash
install -o "$owner_uid" -g "$owner_gid" -m "$file_mode" "$candidate" "$candidate.final"
# Everything the recovery needs is prepared BEFORE the configuration is touched, so recovery is one atomic rename (review 6).
install -o "$owner_uid" -g "$owner_gid" -m "$file_mode" "$backup" "$candidate.restore"
[[ "$(sha256sum "$candidate.restore" | awk '{print $1}')" == "$previous_sha" ]]
revert_now() {
  trap - ERR HUP INT TERM; echo 'FAILURE: restoring the previous configuration' >&2
  mv -f -- "$candidate.restore" "$config"
  systemctl restart "$service" || true; wait_listening "$https_port" || true
  [[ "$(systemctl is-active "$service" || true)" == 'active' ]] && echo 'RECOVERY: service is active with the previous configuration' >&2
  rm -f -- "$candidate.final" "$candidate.restore"; exit 1
}
trap revert_now ERR HUP INT TERM   # a dropped SSH session or Ctrl-C inside the risky window restores the file too
mv -f -- "$candidate.final" "$config"
systemctl restart "$service"

# ---- verification ---------------------------------------------------------------------------------------------------------
wait_listening "$https_port"; wait_listening "$http_port"
[[ "$(systemctl is-active "$service")" == 'active' ]]
code="$(curl -q -sS --noproxy '*' --max-time 10 -o /dev/null -w '%{http_code}' "http://$lan_ip:$http_port/api/2026-10-14/server")"; [[ "$code" == 200 ]]
page="$(curl -q -sS --noproxy '*' --max-time 10 "http://$lan_ip:$http_port/")"; [[ "$page" == *'<title>Chill Center</title>'* ]]
code="$(curl -q -sS --noproxy '*' --max-time 10 --cacert "$CHILLCENTER_TLS_CERT_PATH" -o /dev/null -w '%{http_code}' "https://$lan_ip:$https_port/api/2026-10-14/server")"; [[ "$code" == 200 ]]
code="$(curl -q -sS --noproxy '*' --max-time 10 -o /dev/null -w '%{http_code}' -H 'Origin: http://attacker.example' "http://$lan_ip:$http_port/api/2026-10-14/server")"; [[ "$code" == 403 ]]
trap cleanup EXIT; trap - ERR HUP INT TERM
printf 'enabled_utc=%s\nhttp_listen=%s:%s\nconfig_sha256=%s\nprevious_config_sha256=%s\nbackup=%s\nservice_active=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$lan_ip" "$http_port" \
  "$(sha256sum "$config" | awk '{print $1}')" "$previous_sha" "$backup" "$(systemctl is-active "$service")"
