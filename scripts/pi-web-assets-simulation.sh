#!/usr/bin/env bash
# Simulation of scripts/pi-update-web-assets-operator.sh on a THROWAWAY Linux VM (Colima Ubuntu, aarch64, real systemd), as root.
# The real server binary runs as chillcenter.service with the real previous web build installed; the script is fed through stdin.
#   usage (from the Mac):  colima ssh -- sudo -n <repo>/scripts/pi-web-assets-simulation.sh <repo>
set -uo pipefail
REPO="$1"; OP="$REPO/scripts/pi-update-web-assets-operator.sh"
NEWBIN="$REPO/target/aarch64-unknown-linux-gnu/release/chill-server"; WEB_TAR="$REPO/artifacts/web-pi/chillcenter-web.tar"
LAN=10.99.0.1; W=/tmp/cc-assets; RB=/root/chillcenter-rollback; UNIT=/usr/lib/systemd/system/chillcenter.service; BIN=/usr/lib/chillcenter/chill-server; WEB=/usr/lib/chillcenter/web
pass=0; fail=0
[[ "$(id -u)" == 0 ]] || { echo "must be root"; exit 2; }
ok() { pass=$((pass+1)); echo "PASS  $1"; }
ko() { fail=$((fail+1)); echo "FAIL  $1 — $2"; [[ -n "${OUT:-}" ]] && echo "$OUT" | tail -8 | sed 's/^/        | /'; }
expect() { if [[ "$2" == "$3" ]]; then ok "$1"; else ko "$1" "attendu '$3', obtenu '$2'"; fi; }
sha() { sha256sum "$1" | awk '{print $1}'; }
tree() { ( cd "$1" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}' ); }
teardown() {
  systemctl stop chillcenter.service >/dev/null 2>&1; rm -f "$UNIT"; systemctl daemon-reload >/dev/null 2>&1
  ip link del dum0 2>/dev/null; rm -rf "$W" "$RB" /etc/chillcenter /usr/lib/chillcenter /var/tmp/chillcenter-web.tar
}
teardown; trap teardown EXIT
mkdir -p "$W/new" "$W/data" /etc/chillcenter /usr/lib/chillcenter; chmod 700 "$W/data"; mkdir -m 0700 "$RB"
ip link add dum0 type dummy; ip addr add "$LAN/24" dev dum0; ip link set dum0 up
install -o root -g root -m 0755 "$NEWBIN" "$BIN"; BIN_SHA="$(sha "$BIN")"
tar -xf "$WEB_TAR" -C "$W/new"; NEW_ID="$(grep -o '"build": *"[0-9a-f]\{12\}"' "$W/new/web/build.json" | grep -o '[0-9a-f]\{12\}')"
# A previous build: the new tree with another id everywhere and a coherent SHA256SUMS.
make_variant() {  # $1 = id, $2 = output tar
  rm -rf "$W/var"; mkdir -p "$W/var"; cp -R "$W/new/web" "$W/var/web"
  sed -i "s/$NEW_ID/$1/g" "$W/var/web/index.html" "$W/var/web/build.json" "$W/var/web/sw.js"; echo "variant $1" > "$W/var/web/VARIANT.txt"
  ( cd "$W/var/web" && find . -type f ! -path ./SHA256SUMS -printf '%P\n' | LC_ALL=C sort | xargs sha256sum > SHA256SUMS )
  tar -C "$W/var" -cf "$2" web
}
OLD_ID=aaaaaaaaaaaa; make_variant "$OLD_ID" "$W/old.tar"; tar -xf "$W/old.tar" -C /usr/lib/chillcenter; chown -R root:root "$WEB"
printf '{\n  "listen": "%s:8443",\n  "http_listen": "%s:8080",\n  "data_dir": "%s/data",\n  "tls_names": ["%s"]\n}\n' "$LAN" "$LAN" "$W" "$LAN" > /etc/chillcenter/server.json; chmod 640 /etc/chillcenter/server.json
printf '[Service]\nWorkingDirectory=/usr/lib/chillcenter\nExecStart=%s --config /etc/chillcenter/server.json\nRestart=no\n' "$BIN" > "$UNIT"
systemctl daemon-reload; systemctl start chillcenter
CERT="$W/data/tls/certificate.pem"
for _ in $(seq 1 60); do [[ -s "$CERT" ]] && curl -s --cacert "$CERT" -o /dev/null "https://$LAN:8443/api/2026-10-14/server" && break; sleep 0.5; done
pid() { systemctl show chillcenter -p MainPID --value; }
build_on() { curl -sS --max-time 8 "${2:-}" "$1/build.json" 2>/dev/null | grep -o '"build": *"[0-9a-f]\{12\}"' | grep -o '[0-9a-f]\{12\}'; }
served() { curl -sS --max-time 8 --cacert "$CERT" "https://$LAN:8443/build.json" 2>/dev/null | grep -o '"build": *"[0-9a-f]\{12\}"' | grep -o '[0-9a-f]\{12\}'; }
served_http() { curl -sS --max-time 8 "http://$LAN:8080/build.json" 2>/dev/null | grep -o '"build": *"[0-9a-f]\{12\}"' | grep -o '[0-9a-f]\{12\}'; }
stage_tar() { install -m 0600 "${1:-$WEB_TAR}" /var/tmp/chillcenter-web.tar; }
run() { OUT="$(env INSTALLED_BINARY_SHA256="${BIN_PARAM:-$BIN_SHA}" WEB_ARCHIVE_SHA256="${ARCHIVE_SHA:-$(sha /var/tmp/chillcenter-web.tar)}" EXPECTED_BUILD_ID="${ID_PARAM:-$NEW_ID}" CHILLCENTER_TLS_CERT_PATH="$CERT" CHILLCENTER_BASE_URL="https://$LAN:8443" CHILLCENTER_HTTP_URL="http://$LAN:8080" /bin/bash -s -- < "${SCRIPT:-$OP}" 2>&1)"; RC=$?; }
reset_web() { rm -rf "$WEB" "$RB"/web-* /usr/lib/chillcenter/.web-new.*; tar -xf "$W/old.tar" -C /usr/lib/chillcenter; chown -R root:root "$WEB"; }
leftovers() { ls -A /usr/lib/chillcenter | grep -c '^\.web-new'; }
OLD_TREE="$(tree "$WEB")"
echo "== $(uname -m) $(. /etc/os-release; echo "$PRETTY_NAME"), script $(sha "$OP")"
expect "W0 base : ancien build servi en HTTPS et HTTP" "$(served)/$(served_http)" "$OLD_ID/$OLD_ID"

# ---- W1 nominal ----------------------------------------------------------------------------------------------------------------------
stage_tar; before_pid="$(pid)"; run
expect "W1 mise à jour : code 0" "$RC" 0
expect "W1 nouveau build servi (HTTPS et HTTP)" "$(served)/$(served_http)" "$NEW_ID/$NEW_ID"
expect "W1 le service n'a pas été touché (même processus)" "$(pid)" "$before_pid"
[[ "$(ls -d "$RB"/web-* 2>/dev/null | wc -l | tr -d ' ')" == 1 && "$(tree "$(ls -d "$RB"/web-* | head -1)")" == "$OLD_TREE" ]] && ok "W1 ancien dossier conservé à l'identique dans le rollback" || ko "W1 rollback" "absent ou différent"
echo "$OUT" | grep -q "previous_build=$OLD_ID" && echo "$OUT" | grep -q "installed_build=$NEW_ID" && ok "W1 sortie : builds avant/après" || ko "W1 sortie" "incomplète"
expect "W1 aucun dossier temporaire" "$(leftovers)" 0
expect "W1 droits du dossier web" "$(stat -c '%a:%u:%g' "$WEB")" "755:0:0"

# ---- W2 refusals before any change -----------------------------------------------------------------------------------------------------
reset_web; stage_tar; before_pid="$(pid)"
ARCHIVE_SHA=$(printf 'f%.0s' $(seq 1 64)) run; unset ARCHIVE_SHA; expect "W2 mauvais hash d'archive : refus (2)" "$RC" 2
BIN_PARAM=$(printf 'e%.0s' $(seq 1 64)) run; unset BIN_PARAM; expect "W2 mauvais hash de binaire : refus (2)" "$RC" 2
ID_PARAM=bbbbbbbbbbbb run; unset ID_PARAM; expect "W2 identifiant de build différent de l'archive : refus (2)" "$RC" 2
expect "W2 dossier web et service intacts" "$(tree "$WEB")|$(pid)" "$OLD_TREE|$before_pid"
ID_PARAM=$OLD_ID run; unset ID_PARAM; expect "W2 build déjà installé : refus (2)" "$RC" 2
mv "$WEB" "$W/web-away"; run; expect "W2 pas de dossier web à remplacer : refus (2)" "$RC" 2; mv "$W/web-away" "$WEB"

# ---- W3 hostile or inconsistent archives -------------------------------------------------------------------------------------------------
rm -rf "$W/h"; mkdir -p "$W/h"; cp -R "$W/new/web" "$W/h/web"; ln -s /etc/passwd "$W/h/web/link"; tar -C "$W/h" -cf "$W/hostile.tar" web; stage_tar "$W/hostile.tar"; run
expect "W3 archive avec lien symbolique : refus (2)" "$RC" 2
rm -rf "$W/h"; mkdir -p "$W/h"; cp -R "$W/new/web" "$W/h/web"; echo extra > "$W/h/web/unlisted.txt"; tar -C "$W/h" -cf "$W/uncovered.tar" web; stage_tar "$W/uncovered.tar"; run
expect "W3 fichier hors SHA256SUMS : refus (2)" "$RC" 2
rm -rf "$W/h"; mkdir -p "$W/h"; cp -R "$W/new/web" "$W/h/web"; echo tamper >> "$W/h/web/app.js"; tar -C "$W/h" -cf "$W/tampered.tar" web; stage_tar "$W/tampered.tar"; run
[[ "$RC" != 0 ]] && ok "W3 fichier altéré : refus (somme de contrôle)" || ko "W3" "accepté"
expect "W3 dossier web intact après tous les refus" "$(tree "$WEB")" "$OLD_TREE"
expect "W3 aucun dossier temporaire" "$(leftovers)" 0

# ---- W4 a failing verification puts the previous directory back (service untouched) -----------------------------------------------------
stage_tar; before_pid="$(pid)"
sed 's/page" == \*"?v=\$EXPECTED_BUILD_ID"\*/page" == *"?v=impossible"*/' "$OP" > "$W/broken-verify.sh"; cmp -s "$OP" "$W/broken-verify.sh" && ko "W4 préparation du mutant" "sed sans effet"
SCRIPT="$W/broken-verify.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "W4 vérification en échec : sortie non nulle ($RC)" || ko "W4 code" "0"
echo "$OUT" | grep -q "RECOVERY: previous web directory restored" && ok "W4 message de récupération" || ko "W4 message" "absent"
expect "W4 ancien dossier remis à l'identique" "$(tree "$WEB")" "$OLD_TREE"
expect "W4 ancien build servi, service intact" "$(served)|$(pid)" "$OLD_ID|$before_pid"
expect "W4 aucun dossier temporaire" "$(leftovers)" 0
rm -rf "$RB"/web-*

# ---- W5 same failure with a dead stderr: restoration must not depend on it (review 7) -------------------------------------------------------
sed 's/^trap recover_on_failure EXIT$/trap recover_on_failure EXIT\nexec 2>\/dev\/full/' "$W/broken-verify.sh" > "$W/broken-stderr.sh"; cmp -s "$W/broken-verify.sh" "$W/broken-stderr.sh" && ko "W5 préparation du mutant" "sed sans effet"
SCRIPT="$W/broken-stderr.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "W5 stderr cassé + échec : sortie non nulle ($RC)" || ko "W5 code" "0"
expect "W5 ancien dossier remis malgré stderr cassé" "$(tree "$WEB")" "$OLD_TREE"
rm -rf "$RB"/web-*

# ---- W6 a TERM inside the risky window restores too ----------------------------------------------------------------------------------------
sed 's/^mv -T "\$web_stage\/web" "\$web_dir"; web_swapped=1; rmdir "\$web_stage"; web_stage='"''"'$/&\nkill -TERM $$; sleep 5/' "$OP" > "$W/term-window.sh"; cmp -s "$OP" "$W/term-window.sh" && ko "W6 préparation du mutant" "sed sans effet"
SCRIPT="$W/term-window.sh" run; unset SCRIPT
[[ "$RC" != 0 ]] && ok "W6 signal TERM dans la fenêtre : sortie non nulle ($RC)" || ko "W6 code" "0"
expect "W6 ancien dossier remis après TERM" "$(tree "$WEB")" "$OLD_TREE"
expect "W6 service intact" "$(systemctl is-active chillcenter)" active
rm -rf "$RB"/web-*

# ---- W7 service stopped: refused ------------------------------------------------------------------------------------------------------------
systemctl stop chillcenter; stage_tar; run
expect "W7 service arrêté : refus (2)" "$RC" 2
expect "W7 dossier web intact" "$(tree "$WEB")" "$OLD_TREE"

echo "== résultat : $pass PASS, $fail FAIL"; [[ "$fail" -eq 0 ]]
