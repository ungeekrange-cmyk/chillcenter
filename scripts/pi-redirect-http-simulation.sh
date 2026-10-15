#!/usr/bin/env bash
# Simulation of the 80 -> 8080 redirect of the plain-HTTP listener (ADR-0018; derived from pi-redirect-simulation.sh) on a THROWAWAY Linux VM (Colima Ubuntu, aarch64, real systemd and nftables), as root.
# A client in a separate network namespace reaches the real server binary + real web archive through a veth pair, so the
# prerouting redirect really applies. Touches no real host.
#   usage (from the Mac):  colima ssh -- sudo -n <repo>/scripts/pi-redirect-http-simulation.sh <repo>
set -uo pipefail
REPO="$1"; OP="$REPO/scripts/pi-redirect-80-operator.sh"
NEWBIN="$REPO/target/aarch64-unknown-linux-gnu/release/chill-server"; WEB_TAR="$REPO/artifacts/web-pi/chillcenter-web.tar"
LAN=10.99.0.1; CLIENT=10.99.0.2; NS=ccclient; W=/tmp/cc-redir; pass=0; fail=0
[[ "$(id -u)" == 0 ]] || { echo "must be root"; exit 2; }
ok() { pass=$((pass+1)); echo "PASS  $1"; }
ko() { fail=$((fail+1)); echo "FAIL  $1 — $2"; [[ -n "${OUT:-}" ]] && echo "$OUT" | tail -8 | sed 's/^/        | /'; }
expect() { if [[ "$2" == "$3" ]]; then ok "$1"; else ko "$1" "attendu '$3', obtenu '$2'"; fi; }
teardown() {
  systemctl disable --now chillcenter-redirect-http.service >/dev/null 2>&1; systemctl stop chillcenter.service >/dev/null 2>&1
  nft delete table ip chillcenter_http >/dev/null 2>&1; nft delete table inet cc_other >/dev/null 2>&1
  rm -f /etc/systemd/system/chillcenter-redirect-http.service /etc/chillcenter/redirect-80.nft /etc/chillcenter/.redirect-80.* /etc/systemd/system/.chillcenter-redirect-http.*
  rmdir /etc/chillcenter 2>/dev/null; systemctl daemon-reload >/dev/null 2>&1
  pkill -f "$W/blocker" 2>/dev/null; ip netns del "$NS" 2>/dev/null; ip link del vh0 2>/dev/null; rm -rf "$W"
}
teardown; trap teardown EXIT
mkdir -p "$W/bin" "$W/data" /etc/chillcenter; chmod 700 "$W/data"
cp "$NEWBIN" "$W/bin/chill-server"; tar -xf "$WEB_TAR" -C "$W/bin"
printf '{ "listen": "%s:8443", "http_listen": "%s:8080", "data_dir": "%s/data", "tls_names": ["%s"] }\n' "$LAN" "$LAN" "$W" "$LAN" > "$W/server.json"
ip netns add "$NS"; ip link add vh0 type veth peer name vc0; ip link set vc0 netns "$NS"
ip addr add "$LAN/24" dev vh0; ip link set vh0 up
ip netns exec "$NS" ip addr add "$CLIENT/24" dev vc0; ip netns exec "$NS" ip link set vc0 up; ip netns exec "$NS" ip link set lo up
systemd-run --quiet --unit=chillcenter --collect --property=WorkingDirectory="$W/bin" "$W/bin/chill-server" --config "$W/server.json"
CERT="$W/data/tls/certificate.pem"
for _ in $(seq 1 60); do ip netns exec "$NS" curl -s -o /dev/null "http://$LAN:8080/api/2026-10-14/server" && break; sleep 0.5; done
c() { ip netns exec "$NS" curl -sS --max-time 8 "$@"; }
curl_https() { ip netns exec "$NS" curl -sS --max-time 8 -k -o /dev/null -w '%{http_code}' "https://$LAN:8443/api/2026-10-14/server" 2>/dev/null || true; }
code() { c -o /dev/null -w '%{http_code}' "$@" 2>/dev/null || true; }
other_table() { nft list tables | grep -c 'inet cc_other'; }
nft add table inet cc_other
run() { OUT="$(env CHILLCENTER_LAN_IP="${LAN_PARAM:-$LAN}" /bin/bash -s -- "$@" < "${SCRIPT:-$OP}" 2>&1)"; RC=$?; }
echo "== $(uname -m) $(. /etc/os-release; echo "$PRETTY_NAME"), kernel $(uname -r), $(nft --version), script $(sha256sum "$OP" | awk '{print $1}')"

# ---- R1 baseline: nothing on 80 -------------------------------------------------------------------------------------------------
expect "R1 avant : :8080 répond" "$(code "http://$LAN:8080/api/2026-10-14/server")" 200
expect "R1 avant : :80 ne répond pas" "$(code "http://$LAN/api/2026-10-14/server")" 000

# ---- R2 install ---------------------------------------------------------------------------------------------------------------------
run
expect "R2 installation : code 0" "$RC" 0
echo "$OUT" | grep -q "redirect=$LAN:80->8080" && ok "R2 sortie : redirection annoncée" || ko "R2 sortie" "absente"
expect "R2 API sur 80 (Host sans port, same-origin)" "$(code -H "Origin: http://$LAN" "http://$LAN/api/2026-10-14/server")" 200
expect "R2 page d'accueil sur 80 (navigation saisie)" "$(code "http://$LAN/")" 200
expect "R2 cross-site toujours refusé sur 80" "$(code -H 'Sec-Fetch-Site: cross-site' "http://$LAN/api/2026-10-14/server")" 403
expect "R2 HTTPS :8443 intact (client Mac)" "$(curl_https)" 200
expect "R2 :8080 inchangé (client HTTP direct)" "$(code "http://$LAN:8080/api/2026-10-14/server")" 200
expect "R2 unité active et activée" "$(systemctl is-active chillcenter-redirect-http)/$(systemctl is-enabled chillcenter-redirect-http)" "active/enabled"
expect "R2 autre table nftables intacte" "$(other_table)" 1
expect "R2 service chillcenter intact" "$(systemctl is-active chillcenter)" active
expect "R2 droits des fichiers" "$(stat -c '%a:%u:%g' /etc/chillcenter/redirect-80.nft /etc/systemd/system/chillcenter-redirect-http.service | tr '\n' ' ')" "644:0:0 644:0:0 "

# ---- R3 the unit really loads/unloads the table (what happens at boot, at stop) --------------------------------------------------
systemctl stop chillcenter-redirect-http; expect "R3 arrêt de l'unité : 80 coupé" "$(code "http://$LAN/api/2026-10-14/server")" 000
expect "R3 arrêt de l'unité : :8080 intact" "$(code "http://$LAN:8080/api/2026-10-14/server")" 200
systemctl start chillcenter-redirect-http; expect "R3 redémarrage de l'unité : 80 revient" "$(code "http://$LAN/api/2026-10-14/server")" 200

# ---- R3b only the configured address is redirected (review 5, mutant m1c): another local address keeps refusing 80 -------------------
ip addr add 10.99.0.9/24 dev vh0
expect "R3b autre adresse locale : 80 non redirigé" "$(ip netns exec "$NS" curl -s --max-time 8 -o /dev/null -w '%{http_code}' "http://10.99.0.9/api/2026-10-14/server" 2>/dev/null || true)" 000
ip addr del 10.99.0.9/24 dev vh0

# ---- R4 refusals before any change ------------------------------------------------------------------------------------------------------
run; expect "R4 seconde installation : refus (2)" "$RC" 2
expect "R4 …et la redirection fonctionne toujours" "$(code "http://$LAN/api/2026-10-14/server")" 200
LAN_PARAM=127.0.0.1 run; expect "R4 adresse loopback : refus (2)" "$RC" 2
LAN_PARAM="10.99.0.1;ls" run; expect "R4 adresse piégée : refus (2)" "$RC" 2

# ---- R5 remove -------------------------------------------------------------------------------------------------------------------------
run --remove
expect "R5 retrait : code 0" "$RC" 0
expect "R5 80 coupé" "$(code "http://$LAN/api/2026-10-14/server")" 000
[[ ! -e /etc/chillcenter/redirect-80.nft && ! -e /etc/systemd/system/chillcenter-redirect-http.service ]] && ok "R5 fichiers supprimés" || ko "R5 fichiers" "restent"
expect "R5 :8080 intact" "$(code "http://$LAN:8080/api/2026-10-14/server")" 200
expect "R5 autre table nftables intacte" "$(other_table)" 1
expect "R5 aucune table chillcenter_http" "$(nft list tables | grep -c chillcenter_http)" 0
run --remove; expect "R5 retrait idempotent : code 0" "$RC" 0

# ---- R6 port 80 already used: refused, nothing created -------------------------------------------------------------------------------
cp /usr/bin/python3 "$W/blocker" 2>/dev/null || cp "$(command -v python3)" "$W/blocker"
setsid "$W/blocker" -c "import socket,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); s.bind(('$LAN',80)); s.listen(1); time.sleep(120)" >/dev/null 2>&1 &
sleep 1; run
expect "R6 80 occupé : refus (2)" "$RC" 2
[[ ! -e /etc/chillcenter/redirect-80.nft && ! -e /etc/systemd/system/chillcenter-redirect-http.service ]] && ok "R6 rien créé" || ko "R6" "fichiers créés"
pkill -f "$W/blocker"; sleep 0.5
LAN_PARAM=10.99.0.77 run; expect "R6 adresse absente de l'hôte : refus (2)" "$RC" 2

# ---- R7 failure after enabling: the failure path removes everything --------------------------------------------------------------------
sed 's/redirect to :%s\\n\\t}\\n}\\n'"'"' "\$table" "\$1" "\$service_port"/redirect to :%s\\n\\t}\\n}\\n'"'"' "$table" "$1" 9999/' "$OP" > "$W/broken-render.sh"
if cmp -s "$OP" "$W/broken-render.sh"; then ko "R7 préparation du mutant" "sed sans effet"; fi
SCRIPT="$W/broken-render.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "R7 vérification en échec : sortie non nulle ($RC)" || ko "R7 code" "0"
echo "$OUT" | grep -q "FAILURE: removing everything" && ok "R7 message de défaillance" || ko "R7 message" "absent"
[[ ! -e /etc/chillcenter/redirect-80.nft && ! -e /etc/systemd/system/chillcenter-redirect-http.service && "$(nft list tables | grep -c chillcenter_http)" == 0 ]] && ok "R7 tout retiré (fichiers, unité, table)" || ko "R7 résidus" "présents"
expect "R7 service chillcenter toujours actif" "$(systemctl is-active chillcenter)" active
expect "R7 :8080 intact" "$(code "http://$LAN:8080/api/2026-10-14/server")" 200
expect "R7 aucun temporaire résiduel" "$(ls -A /etc/chillcenter /etc/systemd/system | grep -c '^\.\(redirect-80\|chillcenter-redirect-http\)')" 0

# ---- R8 dry-run failure (kernel/syntax) before anything is installed ---------------------------------------------------------------------
sed 's/ip daddr %s tcp dport 80/ip daddr %s tcp dport 80 bogus_keyword/' "$OP" > "$W/broken-syntax.sh"
SCRIPT="$W/broken-syntax.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "R8 règle invalide : sortie non nulle ($RC)" || ko "R8 code" "0"
[[ ! -e /etc/chillcenter/redirect-80.nft && ! -e /etc/systemd/system/chillcenter-redirect-http.service ]] && ok "R8 aucun fichier installé (essai à blanc)" || ko "R8" "fichiers installés"
expect "R8 aucun temporaire résiduel" "$(ls -A /etc/chillcenter /etc/systemd/system | grep -c '^\.\(redirect-80\|chillcenter-redirect-http\)')" 0

# ---- R9 chillcenter stopped: refused before anything (review 5, mutant m16) --------------------------------------------------------------
systemctl stop chillcenter; run
expect "R9 service arrêté : refus (2)" "$RC" 2
[[ ! -e /etc/chillcenter/redirect-80.nft && ! -e /etc/systemd/system/chillcenter-redirect-http.service ]] && ok "R9 rien créé" || ko "R9" "fichiers créés"

echo "== résultat : $pass PASS, $fail FAIL"; [[ "$fail" -eq 0 ]]
