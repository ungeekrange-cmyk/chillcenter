#!/usr/bin/env bash
# READ-ONLY preflight for the web deployment (docs/deployment/WEB-CLIENT-PI-RUNBOOK.md, step 3).
# Writes nothing, changes nothing, prints no secret (public certificate metadata and hashes only; the TLS private key is never read).
# Run as root on the Pi through stdin:   ssh <master> 'sudo -n /bin/bash -s --' < scripts/pi-preflight-readonly.sh
set -u
bin=/usr/lib/chillcenter/chill-server; cert=/var/lib/chillcenter/data/tls/certificate.pem
echo "preflight_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "run_as_uid=$(id -u)"
. /etc/os-release 2>/dev/null; echo "os=${PRETTY_NAME:-unknown} arch=$(uname -m)"
echo "tools: $(tar --version | head -1) | $(sha256sum --version | head -1) | $(curl --version | head -1 | cut -d' ' -f1-2) | $(openssl version)"
for t in curl ss openssl runuser seq install mktemp sha256sum tar find du stat df awk; do command -v "$t" >/dev/null || echo "MISSING_TOOL=$t"; done
echo "installed_binary_sha256=$(sha256sum "$bin" | awk '{print $1}')   (attendu = valeur de OLD_BINARY_SHA256 : cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3 pour ce déploiement)"
echo "service_active=$(systemctl is-active chillcenter)"
systemctl show chillcenter -p MainPID -p NRestarts -p Restart -p TimeoutStopUSec -p ActiveEnterTimestamp 2>/dev/null
echo "chillcenter_uid=$(id -u chillcenter)   (attendu 997)"
echo "established_chill_server_sockets=$(ss -Htnp state established 2>/dev/null | grep -c -F chill-server)   (attendu 0)"
echo "listening: $(ss -Hltnp 2>/dev/null | grep -F chill-server | awk '{print $4}' | tr '\n' ' ')   (attendu une adresse en :8443)"
echo "config_sha256=$(sha256sum /etc/chillcenter/server.json | awk '{print $1}')"
echo "unit_sha256=$(sha256sum /usr/lib/systemd/system/chillcenter.service | awk '{print $1}')"
echo "cert_spki_sha256=$(openssl x509 -in "$cert" -pubkey -noout | openssl pkey -pubin -outform DER | sha256sum | awk '{print $1}')   (attendu 3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df)"
echo "cert_validity: $(openssl x509 -in "$cert" -noout -startdate -enddate | tr '\n' ' ')"
stat -c 'rollback_dir=%a:%u:%g   (attendu 700:0:0)' /root/chillcenter-rollback
echo "rollback_dir_content:"; ls -1 /root/chillcenter-rollback | sed 's/^/  /'
if [ -e /usr/lib/chillcenter/web ]; then echo "web_dir=PRESENT (sera conservé dans le répertoire de rollback)"; else echo "web_dir=absent"; fi
echo "incoming_files_present: $(ls /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar 2>/dev/null | tr '\n' ' ')"
echo "free_kb (Mio libres ≥ 64 requis) :"; df -Pk /usr/lib/chillcenter /var/lib/chillcenter/data /root/chillcenter-rollback /var/tmp | awk 'NR>1{print "  " $6 " " $4 " Kio"}'
echo "memory_mib: $(free -m | awk '/^Mem:/{print "total " $2 " available " $7}')"
echo "backups_present: $(ls -1 /var/lib/chillcenter/data/backups 2>/dev/null | wc -l)"
