#!/usr/bin/env bash
# Review-only operator script (ADR-0018): redirect TCP 80 -> 8080 (the plain-HTTP listener of the web client) for the Pi's LAN address,
# in a dedicated nftables table loaded by a dedicated systemd unit. Mechanically derived from pi-redirect-443-operator.sh (reviewed in
# review 5): same structure, other names and ports. Requires the service to already listen on <LAN_IP>:8080 (http_listen). DO NOT EXECUTE before an independent reviewer has approved THESE EXACT BYTES (sha256 of
# this file) and the user has given explicit go for the Pi. Runs as root on the Pi, fed through stdin:
#   install: ssh <strict batch control connection> 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 /bin/bash -s --'          < pi-redirect-80-operator.sh
#   remove : ssh <strict batch control connection> 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 /bin/bash -s -- --remove' < pi-redirect-80-operator.sh
#   check  : ssh <strict batch control connection> 'sudo -n /bin/bash -s -- --self-test'                                  < pi-redirect-80-operator.sh
#
# Authority (nothing else): create /etc/chillcenter/redirect-80.nft and /etc/systemd/system/chillcenter-redirect-http.service,
# enable that one unit, and thereby create the nftables table `ip chillcenter_http`. Not touched: the chillcenter service, its
# unit, configuration, certificate and data; /etc/nftables.conf and nftables.service (the default file starts with
# `flush ruleset`, so it is neither edited nor enabled); every other firewall table. Traffic generated on the Pi itself is not
# redirected (prerouting only): local probes keep using :8080.
set -Eeuo pipefail
trap '' PIPE
trap 'echo "ERR line $LINENO: $BASH_COMMAND" >&2' ERR
nft_bin='/usr/sbin/nft'; table='chillcenter_http'; service_port=8080
rules='/etc/chillcenter/redirect-80.nft'; unit_name='chillcenter-redirect-http.service'; unit="/etc/systemd/system/$unit_name"

valid_lan_ip() {
  local ip="$1" octet
  [[ "$ip" =~ ^(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})$ ]] || return 1
  for octet in "${BASH_REMATCH[@]:1}"; do (( octet <= 255 )) || return 1; done
  [[ "${BASH_REMATCH[1]}" != 0 && "${BASH_REMATCH[1]}" != 127 && "${BASH_REMATCH[1]}" -lt 224 ]] || return 1
}
render_rules() {
  printf 'table ip %s {\n\tchain prerouting {\n\t\ttype nat hook prerouting priority dstnat; policy accept;\n\t\tip daddr %s tcp dport 80 redirect to :%s\n\t}\n}\n' "$table" "$1" "$service_port"
}
render_unit() {
  printf '[Unit]\nDescription=Chill Center: redirect TCP 80 to %s (ADR-0018)\nDocumentation=file:/usr/share/doc/chillcenter/\n\n[Service]\nType=oneshot\nRemainAfterExit=yes\nExecStart=%s -f %s\nExecStop=%s delete table ip %s\n\n[Install]\nWantedBy=multi-user.target\n' \
    "$service_port" "$nft_bin" "$rules" "$nft_bin" "$table"
}
must_fail() { if "$@" >/dev/null 2>&1; then echo "SELF-TEST FAILED (accepted what must be refused): $*" >&2; exit 1; fi; }

if [[ "${1:-}" == '--self-test' ]]; then
  valid_lan_ip 192.168.1.42; valid_lan_ip 10.0.0.1; valid_lan_ip 172.16.254.3
  for bad in '' '127.0.0.1' '0.0.0.0' '0.1.2.3' '224.0.0.1' '255.255.255.255' '192.168.1.256' '192.168.1' '192.168.1.42.1' 'a.b.c.d' '192.168.01.42' '192.168.1.42 ' ' 192.168.1.42' '192.168.1.42;' '192.168.1.42
1.1.1.1'; do must_fail valid_lan_ip "$bad"; done
  [[ "$(render_rules 192.168.1.42)" == $'table ip chillcenter_http {\n\tchain prerouting {\n\t\ttype nat hook prerouting priority dstnat; policy accept;\n\t\tip daddr 192.168.1.42 tcp dport 80 redirect to :8080\n\t}\n}' ]]
  [[ "$(render_unit)" == *$'ExecStart=/usr/sbin/nft -f /etc/chillcenter/redirect-80.nft\nExecStop=/usr/sbin/nft delete table ip chillcenter_http\n'* ]]
  printf 'redirect_self_test=PASS\n'; exit 0
fi

mode='install'
case "${1:-}" in '') ;; --remove) mode='remove' ;; *) echo 'unknown argument' >&2; exit 2 ;; esac
: "${CHILLCENTER_LAN_IP:?set the Pi LAN address, e.g. 192.168.1.42}"
valid_lan_ip "$CHILLCENTER_LAN_IP" || { echo 'invalid LAN address' >&2; exit 2; }
lan_ip="$CHILLCENTER_LAN_IP"
[[ "$(id -u)" == '0' ]] || { echo 'must run as root' >&2; exit 2; }
for tool in systemctl ss install mktemp sha256sum awk grep rm; do command -v "$tool" >/dev/null || { echo "required tool missing: $tool" >&2; exit 2; }; done
[[ -x "$nft_bin" ]] || { echo 'nft missing' >&2; exit 2; }
for path in /etc /etc/chillcenter /etc/systemd /etc/systemd/system "$rules" "$unit"; do [[ ! -L "$path" ]] || { echo "symlink refused: $path" >&2; exit 2; }; done
[[ -d /etc/chillcenter ]] || { echo '/etc/chillcenter missing' >&2; exit 2; }

remove_all() {  # best effort and idempotent: used by --remove and by the failure path
  systemctl disable --now "$unit_name" >/dev/null 2>&1 || true
  "$nft_bin" delete table ip "$table" >/dev/null 2>&1 || true
  rm -f -- "$rules" "$unit"
  systemctl daemon-reload >/dev/null 2>&1 || true
}
if [[ "$mode" == 'remove' ]]; then
  remove_all
  [[ -z "$("$nft_bin" list tables 2>/dev/null | grep -F "table ip $table" || true)" && ! -e "$rules" && ! -e "$unit" ]] || { echo 'removal incomplete' >&2; exit 1; }
  printf 'removed_utc=%s\nservice_active=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(systemctl is-active chillcenter || true)"
  exit 0
fi

# ---- preconditions: nothing is changed before all of them hold ----------------------------------------------------------
[[ "$(systemctl is-active chillcenter || true)" == 'active' ]] || { echo 'chillcenter service is not active' >&2; exit 2; }
listening="$(ss -Hltn)"
grep -qE "^LISTEN +[0-9]+ +[0-9]+ +${lan_ip//./\\.}:${service_port} " <<<"$listening" || { echo "nothing listens on $lan_ip:$service_port" >&2; exit 2; }
if grep -qE '[:.]80 ' <<<"$listening"; then echo 'something already listens on port 80' >&2; exit 2; fi
[[ -n "$(ip -o -4 addr show 2>/dev/null | awk -v a="$lan_ip/" 'index($4, a) == 1 {print $4}')" ]] || { echo "$lan_ip is not an address of this host" >&2; exit 2; }
if "$nft_bin" list tables 2>/dev/null | grep -qF "table ip $table"; then echo "nftables table $table already exists" >&2; exit 2; fi
[[ ! -e "$rules" && ! -e "$unit" ]] || { echo 'redirect files already exist (use --remove first)' >&2; exit 2; }
[[ "$(systemctl is-enabled "$unit_name" 2>&1 || true)" != 'enabled' ]] || { echo 'unit already enabled' >&2; exit 2; }
echo "preconditions_ok lan=$lan_ip nft=$("$nft_bin" --version | head -1)"

tmp_rules="$(mktemp /etc/chillcenter/.redirect-80.XXXXXX)"; tmp_unit="$(mktemp /etc/systemd/system/.chillcenter-redirect-http.XXXXXX)"
cleanup_tmp() { rm -f -- "$tmp_rules" "$tmp_unit"; }
trap cleanup_tmp EXIT   # a failed dry run (set -e) must not leave temporaries behind
render_rules "$lan_ip" > "$tmp_rules"; render_unit > "$tmp_unit"
"$nft_bin" -c -f "$tmp_rules"                      # syntax and kernel-support dry run: changes nothing
install -o root -g root -m 0644 "$tmp_rules" "$rules"; install -o root -g root -m 0644 "$tmp_unit" "$unit"
cleanup_tmp
failed() { set +e; trap - ERR; echo 'FAILURE: removing everything this script created' >&2; remove_all; [[ "$(systemctl is-active chillcenter || true)" == 'active' ]] && echo 'RECOVERY: chillcenter is still active' >&2; exit 1; }
trap failed ERR
systemctl daemon-reload
systemctl enable --now "$unit_name"

# ---- verification ---------------------------------------------------------------------------------------------------------
[[ "$(systemctl is-active "$unit_name")" == 'active' ]]
ruleset="$("$nft_bin" list table ip "$table")"
[[ "$ruleset" == *"ip daddr $lan_ip tcp dport 80 redirect to :$service_port"* ]]
[[ "$(systemctl is-active chillcenter)" == 'active' ]]
grep -qE "^LISTEN +[0-9]+ +[0-9]+ +${lan_ip//./\\.}:${service_port} " <<<"$(ss -Hltn)"
trap - ERR
printf 'installed_utc=%s\nredirect=%s:80->%s\nrules_sha256=%s\nunit_sha256=%s\nunit_state=%s/%s\nservice_active=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$lan_ip" "$service_port" \
  "$(sha256sum "$rules" | awk '{print $1}')" "$(sha256sum "$unit" | awk '{print $1}')" "$(systemctl is-enabled "$unit_name")" "$(systemctl is-active "$unit_name")" "$(systemctl is-active chillcenter)"
