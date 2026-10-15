#!/usr/bin/env bash
# Simulation of the 443 -> 8443 redirect (ADR-0017) on a THROWAWAY Linux VM (Colima Ubuntu, aarch64, real systemd and nftables), as root.
# A client in a separate network namespace reaches the real server binary + real web archive through a veth pair, so the
# prerouting redirect really applies. Touches no real host.
#   usage (from the Mac):  colima ssh -- sudo -n <repo>/scripts/pi-redirect-simulation.sh <repo>
set -uo pipefail
REPO="$1"; OP="$REPO/scripts/pi-redirect-443-operator.sh"
NEWBIN="$REPO/target/aarch64-unknown-linux-gnu/release/chill-server"; WEB_TAR="$REPO/artifacts/web-pi/chillcenter-web.tar"
LAN=10.99.0.1; CLIENT=10.99.0.2; NS=ccclient; W=/tmp/cc-redir; pass=0; fail=0
[[ "$(id -u)" == 0 ]] || { echo "must be root"; exit 2; }
ok() { pass=$((pass+1)); echo "PASS  $1"; }
ko() { fail=$((fail+1)); echo "FAIL  $1 — $2"; [[ -n "${OUT:-}" ]] && echo "$OUT" | tail -8 | sed 's/^/        | /'; }
expect() { if [[ "$2" == "$3" ]]; then ok "$1"; else ko "$1" "attendu '$3', obtenu '$2'"; fi; }
teardown() {
  systemctl disable --now chillcenter-redirect.service >/dev/null 2>&1; systemctl stop chillcenter.service >/dev/null 2>&1
  nft delete table ip chillcenter_web >/dev/null 2>&1; nft delete table inet cc_other >/dev/null 2>&1
  rm -f /etc/systemd/system/chillcenter-redirect.service /etc/chillcenter/redirect-443.nft /etc/chillcenter/.redirect-443.* /etc/systemd/system/.chillcenter-redirect.*
  rmdir /etc/chillcenter 2>/dev/null; systemctl daemon-reload >/dev/null 2>&1
  pkill -f "$W/blocker" 2>/dev/null; ip netns del "$NS" 2>/dev/null; ip link del vh0 2>/dev/null; rm -rf "$W"
}
teardown; trap teardown EXIT
mkdir -p "$W/bin" "$W/data" /etc/chillcenter; chmod 700 "$W/data"
cp "$NEWBIN" "$W/bin/chill-server"; tar -xf "$WEB_TAR" -C "$W/bin"
printf '{ "listen": "%s:8443", "data_dir": "%s/data", "tls_names": ["%s"] }\n' "$LAN" "$W" "$LAN" > "$W/server.json"
ip netns add "$NS"; ip link add vh0 type veth peer name vc0; ip link set vc0 netns "$NS"
ip addr add "$LAN/24" dev vh0; ip link set vh0 up
ip netns exec "$NS" ip addr add "$CLIENT/24" dev vc0; ip netns exec "$NS" ip link set vc0 up; ip netns exec "$NS" ip link set lo up
systemd-run --quiet --unit=chillcenter --collect --property=WorkingDirectory="$W/bin" "$W/bin/chill-server" --config "$W/server.json"
CERT="$W/data/tls/certificate.pem"
for _ in $(seq 1 60); do [[ -s "$CERT" ]] && ip netns exec "$NS" curl -s --cacert "$CERT" -o /dev/null "https://$LAN:8443/api/2026-10-14/server" && break; sleep 0.5; done
c() { ip netns exec "$NS" curl -sS --max-time 8 --cacert "$CERT" "$@"; }
code() { c -o /dev/null -w '%{http_code}' "$@" 2>/dev/null || true; }
other_table() { nft list tables | grep -c 'inet cc_other'; }
nft add table inet cc_other
run() { OUT="$(env CHILLCENTER_LAN_IP="${LAN_PARAM:-$LAN}" /bin/bash -s -- "$@" < "${SCRIPT:-$OP}" 2>&1)"; RC=$?; }
echo "== $(uname -m) $(. /etc/os-release; echo "$PRETTY_NAME"), kernel $(uname -r), $(nft --version), script $(sha256sum "$OP" | awk '{print $1}')"

# ---- R1 baseline: nothing on 443 -------------------------------------------------------------------------------------------------
expect "R1 avant : :8443 répond" "$(code "https://$LAN:8443/api/2026-10-14/server")" 200
expect "R1 avant : :443 ne répond pas" "$(code "https://$LAN/api/2026-10-14/server")" 000

# ---- R2 install ---------------------------------------------------------------------------------------------------------------------
run
expect "R2 installation : code 0" "$RC" 0
echo "$OUT" | grep -q "redirect=$LAN:443->8443" && ok "R2 sortie : redirection annoncée" || ko "R2 sortie" "absente"
expect "R2 API sur 443 (Host sans port, same-origin)" "$(code -H 'Sec-Fetch-Site: same-origin' "https://$LAN/api/2026-10-14/server")" 200
expect "R2 page d'accueil sur 443 (navigation saisie)" "$(code -H 'Sec-Fetch-Site: none' "https://$LAN/")" 200
expect "R2 cross-site toujours refusé sur 443" "$(code -H 'Sec-Fetch-Site: cross-site' "https://$LAN/api/2026-10-14/server")" 403
expect "R2 :8443 inchangé (clients Mac)" "$(code "https://$LAN:8443/api/2026-10-14/server")" 200
expect "R2 port 80 : rien" "$(code "http://$LAN/")" 000
spki() { openssl x509 -pubkey -noout | openssl pkey -pubin -outform DER | sha256sum | awk '{print $1}'; }
expect "R2 certificat servi sur 443 = certificat du serveur" "$(ip netns exec "$NS" openssl s_client -connect "$LAN:443" </dev/null 2>/dev/null | spki)" "$(spki < "$CERT")"
expect "R2 unité active et activée" "$(systemctl is-active chillcenter-redirect)/$(systemctl is-enabled chillcenter-redirect)" "active/enabled"
expect "R2 autre table nftables intacte" "$(other_table)" 1
expect "R2 service chillcenter intact" "$(systemctl is-active chillcenter)" active
expect "R2 droits des fichiers" "$(stat -c '%a:%u:%g' /etc/chillcenter/redirect-443.nft /etc/systemd/system/chillcenter-redirect.service | tr '\n' ' ')" "644:0:0 644:0:0 "

# ---- R3 the unit really loads/unloads the table (what happens at boot, at stop) --------------------------------------------------
systemctl stop chillcenter-redirect; expect "R3 arrêt de l'unité : 443 coupé" "$(code "https://$LAN/api/2026-10-14/server")" 000
expect "R3 arrêt de l'unité : :8443 intact" "$(code "https://$LAN:8443/api/2026-10-14/server")" 200
systemctl start chillcenter-redirect; expect "R3 redémarrage de l'unité : 443 revient" "$(code "https://$LAN/api/2026-10-14/server")" 200

# ---- R4 refusals before any change ------------------------------------------------------------------------------------------------------
run; expect "R4 seconde installation : refus (2)" "$RC" 2
expect "R4 …et la redirection fonctionne toujours" "$(code "https://$LAN/api/2026-10-14/server")" 200
LAN_PARAM=127.0.0.1 run; expect "R4 adresse loopback : refus (2)" "$RC" 2
LAN_PARAM="10.99.0.1;ls" run; expect "R4 adresse piégée : refus (2)" "$RC" 2

# ---- R5 remove -------------------------------------------------------------------------------------------------------------------------
run --remove
expect "R5 retrait : code 0" "$RC" 0
expect "R5 443 coupé" "$(code "https://$LAN/api/2026-10-14/server")" 000
[[ ! -e /etc/chillcenter/redirect-443.nft && ! -e /etc/systemd/system/chillcenter-redirect.service ]] && ok "R5 fichiers supprimés" || ko "R5 fichiers" "restent"
expect "R5 :8443 intact" "$(code "https://$LAN:8443/api/2026-10-14/server")" 200
expect "R5 autre table nftables intacte" "$(other_table)" 1
expect "R5 aucune table chillcenter_web" "$(nft list tables | grep -c chillcenter_web)" 0
run --remove; expect "R5 retrait idempotent : code 0" "$RC" 0

# ---- R6 port 443 already used: refused, nothing created -------------------------------------------------------------------------------
cp /usr/bin/python3 "$W/blocker" 2>/dev/null || cp "$(command -v python3)" "$W/blocker"
setsid "$W/blocker" -c "import socket,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); s.bind(('$LAN',443)); s.listen(1); time.sleep(120)" >/dev/null 2>&1 &
sleep 1; run
expect "R6 443 occupé : refus (2)" "$RC" 2
[[ ! -e /etc/chillcenter/redirect-443.nft && ! -e /etc/systemd/system/chillcenter-redirect.service ]] && ok "R6 rien créé" || ko "R6" "fichiers créés"
pkill -f "$W/blocker"; sleep 0.5
LAN_PARAM=10.99.0.77 run; expect "R6 adresse absente de l'hôte : refus (2)" "$RC" 2

# ---- R7 failure after enabling: the failure path removes everything --------------------------------------------------------------------
sed 's/redirect to :%s\\n\\t}\\n}\\n'"'"' "\$table" "\$1" "\$service_port"/redirect to :%s\\n\\t}\\n}\\n'"'"' "$table" "$1" 9999/' "$OP" > "$W/broken-render.sh"
if cmp -s "$OP" "$W/broken-render.sh"; then ko "R7 préparation du mutant" "sed sans effet"; fi
SCRIPT="$W/broken-render.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "R7 vérification en échec : sortie non nulle ($RC)" || ko "R7 code" "0"
echo "$OUT" | grep -q "FAILURE: removing everything" && ok "R7 message de défaillance" || ko "R7 message" "absent"
[[ ! -e /etc/chillcenter/redirect-443.nft && ! -e /etc/systemd/system/chillcenter-redirect.service && "$(nft list tables | grep -c chillcenter_web)" == 0 ]] && ok "R7 tout retiré (fichiers, unité, table)" || ko "R7 résidus" "présents"
expect "R7 service chillcenter toujours actif" "$(systemctl is-active chillcenter)" active
expect "R7 :8443 intact" "$(code "https://$LAN:8443/api/2026-10-14/server")" 200
expect "R7 aucun temporaire résiduel" "$(ls -A /etc/chillcenter /etc/systemd/system | grep -c '^\.\(redirect-443\|chillcenter-redirect\)')" 0

# ---- R8 dry-run failure (kernel/syntax) before anything is installed ---------------------------------------------------------------------
sed 's/ip daddr %s tcp dport 443/ip daddr %s tcp dport 443 bogus_keyword/' "$OP" > "$W/broken-syntax.sh"
SCRIPT="$W/broken-syntax.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "R8 règle invalide : sortie non nulle ($RC)" || ko "R8 code" "0"
[[ ! -e /etc/chillcenter/redirect-443.nft && ! -e /etc/systemd/system/chillcenter-redirect.service ]] && ok "R8 aucun fichier installé (essai à blanc)" || ko "R8" "fichiers installés"
expect "R8 aucun temporaire résiduel" "$(ls -A /etc/chillcenter /etc/systemd/system | grep -c '^\.\(redirect-443\|chillcenter-redirect\)')" 0

echo "== résultat : $pass PASS, $fail FAIL"; [[ "$fail" -eq 0 ]]
