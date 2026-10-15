#!/usr/bin/env bash
# Simulation of scripts/pi-enable-http-operator.sh (ADR-0018) on a THROWAWAY Linux VM (Colima Ubuntu, aarch64, real systemd), as root.
# A real, restartable chillcenter.service runs the real server binary with a Pi-like configuration. Touches no real host.
#   usage (from the Mac):  colima ssh -- sudo -n <repo>/scripts/pi-enable-http-simulation.sh <repo>
set -uo pipefail
REPO="$1"; OP="$REPO/scripts/pi-enable-http-operator.sh"
NEWBIN="$REPO/target/aarch64-unknown-linux-gnu/release/chill-server"; WEB_TAR="$REPO/artifacts/web-pi/chillcenter-web.tar"
LAN=10.99.0.1; W=/tmp/cc-enable; RB=/root/chillcenter-rollback; CFG=/etc/chillcenter/server.json; UNIT=/etc/systemd/system/chillcenter.service
pass=0; fail=0
[[ "$(id -u)" == 0 ]] || { echo "must be root"; exit 2; }
ok() { pass=$((pass+1)); echo "PASS  $1"; }
ko() { fail=$((fail+1)); echo "FAIL  $1 — $2"; [[ -n "${OUT:-}" ]] && echo "$OUT" | tail -8 | sed 's/^/        | /'; }
expect() { if [[ "$2" == "$3" ]]; then ok "$1"; else ko "$1" "attendu '$3', obtenu '$2'"; fi; }
sha() { sha256sum "$1" | awk '{print $1}'; }
teardown() {
  systemctl stop chillcenter.service >/dev/null 2>&1; rm -f "$UNIT" /etc/systemd/system/multi-user.target.wants/chillcenter.service; systemctl daemon-reload >/dev/null 2>&1
  pkill -f "$W/blocker" 2>/dev/null; ip link del dum0 2>/dev/null; rm -rf "$W" "$RB" /etc/chillcenter /usr/lib/chillcenter
}
teardown; trap teardown EXIT
mkdir -p "$W/bin" "$W/data" /etc/chillcenter; chmod 700 "$W/data"; mkdir -m 0700 "$RB"
cp "$NEWBIN" "$W/bin/chill-server"; tar -xf "$WEB_TAR" -C "$W/bin"; NEW_SHA="$(sha "$W/bin/chill-server")"
mkdir -p /usr/lib/chillcenter; install -o root -g root -m 0755 "$W/bin/chill-server" /usr/lib/chillcenter/chill-server   # the path the operator script pins
ip link add dum0 type dummy; ip addr add "$LAN/24" dev dum0; ip link set dum0 up
getent group nogroup >/dev/null || groupadd nogroup
printf '{\n  "listen": "%s:8443",\n  "data_dir": "%s/data",\n  "tls_names": ["%s"]\n}\n' "$LAN" "$W" "$LAN" > "$CFG"
chown root:nogroup "$CFG"; chmod 640 "$CFG"; ORIG_SHA="$(sha "$CFG")"; cp "$CFG" "$W/original.json"
# the real binary refuses a working directory equal to its data root, like on the Pi (WorkingDirectory=/usr/lib/chillcenter)
printf '[Service]\nWorkingDirectory=%s/bin\nExecStart=%s/bin/chill-server --config %s\nRestart=no\n' "$W" "$W" "$CFG" > "$UNIT"
systemctl daemon-reload; systemctl start chillcenter
CERT="$W/data/tls/certificate.pem"
for _ in $(seq 1 60); do [[ -s "$CERT" ]] && curl -s --cacert "$CERT" -o /dev/null "https://$LAN:8443/api/2026-10-14/server" && break; sleep 0.5; done
pid() { systemctl show chillcenter -p MainPID --value; }
https() { curl -sS --max-time 8 --cacert "$CERT" -o /dev/null -w '%{http_code}' "https://$LAN:8443/api/2026-10-14/server" 2>/dev/null || true; }
http()  { curl -sS --max-time 8 "$@" -o /dev/null -w '%{http_code}' 2>/dev/null || true; }
run() { OUT="$(env CHILLCENTER_LAN_IP=$LAN NEW_BINARY_SHA256="${BIN_SHA:-$NEW_SHA}" CONFIG_SHA256="${CFG_PARAM:-$ORIG_SHA}" CHILLCENTER_TLS_CERT_PATH="$CERT" /bin/bash -s -- "$@" < "${SCRIPT:-$OP}" 2>&1)"; RC=$?; }
reset_cfg() { systemctl stop chillcenter; cp "$W/original.json" "$CFG"; chown root:nogroup "$CFG"; chmod 640 "$CFG"; rm -f "$RB"/server.json.*; systemctl start chillcenter; for _ in $(seq 1 40); do [[ "$(https)" == 200 ]] && return 0; sleep 0.5; done; return 1; }
tmp_left() { ls -A /etc/chillcenter | grep -c '^\.server\.json'; }
echo "== $(uname -m) $(. /etc/os-release; echo "$PRETTY_NAME"), systemd $(systemctl --version | head -1 | awk '{print $2}'), script $(sha "$OP")"
expect "E0 base : HTTPS répond, rien sur 8080" "$(https)/$(http "http://$LAN:8080/")" "200/000"

# ---- E1 nominal -----------------------------------------------------------------------------------------------------------------
before_pid="$(pid)"; run
expect "E1 activation : code 0" "$RC" 0
echo "$OUT" | grep -q "http_listen=$LAN:8080" && ok "E1 sortie : listener annoncé" || ko "E1 sortie" "absente"
[[ "$(pid)" != "$before_pid" && "$(systemctl is-active chillcenter)" == active ]] && ok "E1 service redémarré et actif" || ko "E1 service" "pid inchangé ou arrêté"
expect "E1 HTTP : API 200" "$(http "http://$LAN:8080/api/2026-10-14/server")" 200
expect "E1 HTTP : page d'accueil" "$(http "http://$LAN:8080/")" 200
expect "E1 HTTP : Origin étrangère refusée" "$(http -H 'Origin: http://attacker.example' "http://$LAN:8080/api/2026-10-14/server")" 403
expect "E1 HTTPS toujours opérationnel" "$(https)" 200
expect "E1 configuration : une seule ligne ajoutée" "$(diff "$W/original.json" "$CFG" | grep -c '^[<>]')" 1
expect "E1 propriétaire et droits conservés" "$(stat -c '%U:%G:%a' "$CFG")" "root:nogroup:640"
expect "E1 sauvegarde = original, 0600 root" "$(sha "$RB/server.json.${ORIG_SHA:0:12}")|$(stat -c '%a:%u' "$RB/server.json.${ORIG_SHA:0:12}")" "$ORIG_SHA|600:0"
expect "E1 aucun temporaire résiduel" "$(tmp_left)" 0

# ---- E2 refusals before any change ------------------------------------------------------------------------------------------------
reset_cfg; before_pid="$(pid)"; CFG_PARAM=$(printf 'f%.0s' $(seq 1 64)) run
expect "E2 mauvais hash de configuration : refus (2)" "$RC" 2
expect "E2 service et configuration intacts" "$(pid)|$(sha "$CFG")" "$before_pid|$ORIG_SHA"
BIN_SHA=$(printf 'e%.0s' $(seq 1 64)) run; unset BIN_SHA
expect "E2 mauvais hash de binaire : refus (2)" "$RC" 2
expect "E2 service intact" "$(pid)" "$before_pid"
OUT="$(env CHILLCENTER_LAN_IP='10.99.0.1;ls' NEW_BINARY_SHA256="$NEW_SHA" CONFIG_SHA256="$ORIG_SHA" CHILLCENTER_TLS_CERT_PATH="$CERT" /bin/bash -s -- < "$OP" 2>&1)"; RC=$?
expect "E2 adresse piégée : refus (2)" "$RC" 2
expect "E2 aucune sauvegarde créée" "$(ls "$RB" | wc -l | tr -d ' ')" 0

# ---- E3 port 8080 taken, or a client connected: refused before the restart --------------------------------------------------------------
cp "$(command -v python3)" "$W/blocker"
setsid "$W/blocker" -c "import socket,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); s.bind(('$LAN',8080)); s.listen(1); time.sleep(120)" >/dev/null 2>&1 &
sleep 1; before_pid="$(pid)"; run
expect "E3 port 8080 occupé : refus (2)" "$RC" 2
expect "E3 service et configuration intacts" "$(pid)|$(sha "$CFG")" "$before_pid|$ORIG_SHA"
pkill -f "$W/blocker"; sleep 0.5
exec 7<>/dev/tcp/$LAN/8443; sleep 0.5; run; exec 7>&- 7<&-
expect "E3 connexion cliente établie : refus (42)" "$RC" 42
expect "E3 service intact (connexion)" "$(pid)" "$before_pid"

# ---- E4 a failing verification puts the previous configuration back -------------------------------------------------------------------
sed 's/\[\[ "\$code" == 403 \]\]$/[[ "$code" == 200 ]]/' "$OP" > "$W/broken-verify.sh"; cmp -s "$OP" "$W/broken-verify.sh" && ko "E4 préparation du mutant" "sed sans effet"
SCRIPT="$W/broken-verify.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "E4 vérification en échec : sortie non nulle ($RC)" || ko "E4 code" "0"
echo "$OUT" | grep -q "RECOVERY: service is active" && ok "E4 message de récupération" || ko "E4 message" "absent"
expect "E4 configuration d'origine restaurée (octets)" "$(sha "$CFG")" "$ORIG_SHA"
expect "E4 HTTPS répond, HTTP coupé" "$(https)/$(http "http://$LAN:8080/")" "200/000"
expect "E4 propriétaire et droits restaurés (récupération automatique)" "$(stat -c '%U:%G:%a' "$CFG")" "root:nogroup:640"
expect "E4 aucun temporaire résiduel" "$(tmp_left)" 0
rm -f "$RB"/server.json.*

# ---- E5 the candidate configuration makes the service fail to start: put the previous one back -----------------------------------------
sed 's/"  \\"http_listen\\": \\"\$2:\$http_port\\","/"  \\"http_listen\\": \\"999.1.1.1:$http_port\\","/' "$OP" > "$W/broken-config.sh"
sed -i 's/line="  \\"http_listen\\": \\"\$2:\$http_port\\","/line="  \\"http_listen\\": \\"203.0.113.9:$http_port\\","/' "$W/broken-config.sh"
cmp -s "$OP" "$W/broken-config.sh" && ko "E5 préparation du mutant" "sed sans effet"
SCRIPT="$W/broken-config.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "E5 configuration refusée par le serveur : sortie non nulle ($RC)" || ko "E5 code" "0"
expect "E5 configuration d'origine restaurée" "$(sha "$CFG")" "$ORIG_SHA"
[[ "$(systemctl is-active chillcenter)" == active && "$(https)" == 200 ]] && ok "E5 service actif avec l'ancienne configuration" || ko "E5 service" "arrêté"
expect "E5 propriétaire et droits restaurés (récupération automatique)" "$(stat -c '%U:%G:%a' "$CFG")" "root:nogroup:640"
rm -f "$RB"/server.json.*

# ---- E5d same failure, but stderr is dead (dropped SSH session): recovery must still restore the file (review 7, finding 1) --------------------
reset_cfg
sed 's/^trap revert_now ERR HUP INT TERM.*$/&\nexec 2>\/dev\/full/' "$W/broken-config.sh" > "$W/broken-config-stderr.sh"; cmp -s "$W/broken-config.sh" "$W/broken-config-stderr.sh" && ko "E5d préparation du mutant" "sed sans effet"
SCRIPT="$W/broken-config-stderr.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "E5d stderr cassé + échec de démarrage : sortie non nulle ($RC)" || ko "E5d code" "0"
expect "E5d configuration d'origine restaurée malgré stderr cassé" "$(sha "$CFG")" "$ORIG_SHA"
[[ "$(systemctl is-active chillcenter)" == active && "$(https)" == 200 ]] && ok "E5d service actif avec l'ancienne configuration" || ko "E5d service" "arrêté"
rm -f "$RB"/server.json.*

# ---- E5b the guard "exactly one added line" refuses a doubled insertion before anything is touched (review 6, mutant M6) --------------------
reset_cfg; before_pid="$(pid)"
sed 's/{ print line; done=1 }/{ print line; print line; done=1 }/' "$OP" > "$W/broken-guard.sh"; cmp -s "$OP" "$W/broken-guard.sh" && ko "E5b préparation du mutant" "sed sans effet"
SCRIPT="$W/broken-guard.sh" run; unset SCRIPT
expect "E5b insertion doublée : refus (2)" "$RC" 2
expect "E5b service, configuration et sauvegardes intacts" "$(pid)|$(sha "$CFG")|$(ls "$RB" | wc -l | tr -d ' ')" "$before_pid|$ORIG_SHA|0"

# ---- E5c a TERM signal inside the risky window restores the configuration (review 6, finding 3) -------------------------------------------
reset_cfg
sed 's/^systemctl restart "\$service"$/systemctl restart "$service"; kill -TERM $$; sleep 5/' "$OP" > "$W/term-window.sh"; cmp -s "$OP" "$W/term-window.sh" && ko "E5c préparation du mutant" "sed sans effet"
SCRIPT="$W/term-window.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "E5c signal TERM dans la fenêtre : sortie non nulle ($RC)" || ko "E5c code" "0"
expect "E5c configuration d'origine restaurée par le signal" "$(sha "$CFG")" "$ORIG_SHA"
[[ "$(systemctl is-active chillcenter)" == active && "$(https)" == 200 ]] && ok "E5c service actif avec l'ancienne configuration" || ko "E5c service" "arrêté"
rm -f "$RB"/server.json.*

# ---- E6 second activation refused, then revert mode ----------------------------------------------------------------------------------------
reset_cfg; run; expect "E6 activation : code 0" "$RC" 0
NEW_CFG_SHA="$(sha "$CFG")"; before_pid="$(pid)"
CFG_PARAM="$NEW_CFG_SHA" run; expect "E6 http_listen déjà présent : refus (2)" "$RC" 2
expect "E6 service intact" "$(pid)" "$before_pid"
OUT="$(env CHILLCENTER_LAN_IP=$LAN CONFIG_SHA256="$ORIG_SHA" PREVIOUS_CONFIG_SHA256="$ORIG_SHA" /bin/bash -s -- --revert < "$OP" 2>&1)"; RC=$?
expect "E6 --revert avec un hash actuel erroné : refus (2)" "$RC" 2
OUT="$(env CHILLCENTER_LAN_IP=$LAN CONFIG_SHA256="$NEW_CFG_SHA" PREVIOUS_CONFIG_SHA256="$(printf 'd%.0s' $(seq 1 64))" /bin/bash -s -- --revert < "$OP" 2>&1)"; RC=$?
expect "E6 --revert sans sauvegarde valide : refus (2)" "$RC" 2
OUT="$(env CHILLCENTER_LAN_IP=$LAN CONFIG_SHA256="$NEW_CFG_SHA" PREVIOUS_CONFIG_SHA256="$ORIG_SHA" /bin/bash -s -- --revert < "$OP" 2>&1)"; RC=$?
expect "E6 --revert : code 0" "$RC" 0
expect "E6 --revert : configuration d'origine, droits conservés" "$(sha "$CFG")|$(stat -c '%U:%G:%a' "$CFG")" "$ORIG_SHA|root:nogroup:640"
expect "E6 --revert : HTTPS répond, HTTP coupé" "$(https)/$(http "http://$LAN:8080/")" "200/000"
expect "E6 aucun temporaire résiduel" "$(tmp_left)" 0

echo "== résultat : $pass PASS, $fail FAIL"; [[ "$fail" -eq 0 ]]
