#!/usr/bin/env bash
# READ-ONLY probe of the Pi network/firewall state, to design the 443 -> 8443 redirect (ADR-0017). Writes nothing, changes nothing.
# Run as root through stdin:   ssh <master> 'sudo -n /bin/bash -s --' < scripts/pi-firewall-probe-readonly.sh
set -u
echo "probe_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"; echo "run_as_uid=$(id -u)"
. /etc/os-release 2>/dev/null; echo "os=${PRETTY_NAME:-unknown} kernel=$(uname -r) arch=$(uname -m)"
echo "addresses: $(ip -o -4 addr show scope global 2>/dev/null | awk '{print $2"="$4}' | tr '\n' ' ')"
echo "listening_80_443_8443: $(ss -Hltn 2>/dev/null | awk '{print $4}' | grep -E ':(80|443|8443)$' | tr '\n' ' ')"
for t in nft iptables ip6tables ufw firewall-cmd; do if command -v "$t" >/dev/null; then echo "tool_$t=$(command -v "$t")"; else echo "tool_$t=absent"; fi; done
command -v nft >/dev/null && echo "nft_version=$(nft --version 2>&1 | head -1)"
command -v iptables >/dev/null && echo "iptables_version=$(iptables --version 2>&1 | head -1)"
for u in nftables ufw netfilter-persistent iptables firewalld; do echo "unit_$u: enabled=$(systemctl is-enabled "$u" 2>&1 | head -1) active=$(systemctl is-active "$u" 2>&1 | head -1)"; done
echo "modules: $(lsmod 2>/dev/null | awk '{print $1}' | grep -E '^(nf_tables|nft_nat|nft_redir|nf_nat|xt_REDIRECT|iptable_nat)$' | tr '\n' ' ')"
echo "--- nft list ruleset (rules only) ---"; nft list ruleset 2>&1 | head -80
echo "--- iptables-save nat (if any) ---"; iptables-save -t nat 2>&1 | head -40
echo "--- ufw ---"; ufw status 2>&1 | head -20
for f in /etc/nftables.conf /etc/iptables/rules.v4 /etc/ufw/ufw.conf; do if [ -e "$f" ]; then echo "file_present=$f sha256=$(sha256sum "$f" | awk '{print $1}') bytes=$(stat -c %s "$f")"; else echo "file_absent=$f"; fi; done
ls -1 /etc/nftables.d 2>/dev/null | sed 's/^/nftables.d: /'
echo "ip_forward=$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null)"
