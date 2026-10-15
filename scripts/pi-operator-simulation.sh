#!/usr/bin/env bash
# Simulation of the Pi deployment on a THROWAWAY Linux VM (Colima Ubuntu, aarch64), as root.
# It recreates the Pi layout (uid 997, /usr/lib/chillcenter, /etc/chillcenter, rollback dir, data dir), runs the REAL
# previous binary (1eb24e97…) as the "service" through a fake systemctl, then feeds scripts/pi-update-web-operator.sh
# through stdin exactly like the real procedure and checks success and failure paths. Touches no real host.
#   usage (from the Mac):  colima ssh -- sudo -n <repo>/scripts/pi-operator-simulation.sh <repo> <old-release.tar.gz>
set -uo pipefail
REPO="$1"; OLD_TARBALL="$2"
OP="$REPO/scripts/pi-update-web-operator.sh"
NEWBIN_SRC="$REPO/target/aarch64-unknown-linux-gnu/release/chill-server"; WEB_TAR="$REPO/artifacts/web-pi/chillcenter-web.tar"
OLD_SHA=1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181
BIN=/usr/lib/chillcenter/chill-server; WEB=/usr/lib/chillcenter/web; RB=/root/chillcenter-rollback; CFG=/etc/chillcenter/server.json
UNIT=/usr/lib/systemd/system/chillcenter.service; BASE=https://127.0.0.1:18443; CERT=/var/lib/chillcenter/data/tls/certificate.pem
FAKE=/opt/fake-cc-bin; WORK="$(mktemp -d /tmp/cc-sim.XXXXXX)"; pass=0; fail=0
[[ "$(id -u)" == 0 ]] || { echo "must be root"; exit 2; }
sha() { sha256sum "$1" | awk '{print $1}'; }
ok() { pass=$((pass+1)); echo "PASS  $1"; }
ko() { fail=$((fail+1)); echo "FAIL  $1 — $2"; }
expect() { if [[ "$2" == "$3" ]]; then ok "$1"; else ko "$1" "attendu '$3', obtenu '$2'"; fi; }

# ---- the Pi layout ------------------------------------------------------------------------------------
if getent passwd 997 >/dev/null && [[ "$(getent passwd 997 | cut -d: -f1)" != chillcenter ]]; then echo "uid 997 déjà pris par un autre compte : simulation impossible"; exit 2; fi
getent group chillcenter >/dev/null || groupadd -g 983 chillcenter
getent passwd chillcenter >/dev/null || useradd -r -u 997 -g 983 -M -s /usr/sbin/nologin chillcenter
mkdir -p /usr/lib/chillcenter /etc/chillcenter /var/lib/chillcenter/data /usr/lib/systemd/system "$FAKE" /run/fake-cc /var/tmp
chmod 755 /usr/lib/chillcenter; chown root:root /usr/lib/chillcenter
chown chillcenter:chillcenter /var/lib/chillcenter /var/lib/chillcenter/data; chmod 700 /var/lib/chillcenter /var/lib/chillcenter/data
cat > "$CFG" <<'EOF'
{ "listen": "127.0.0.1:18443", "data_dir": "/var/lib/chillcenter/data", "tls_names": ["127.0.0.1", "localhost"] }
EOF
chown root:chillcenter "$CFG"; chmod 640 "$CFG"
printf '[Service]\nExecStart=/usr/lib/chillcenter/chill-server\n' > "$UNIT"
# the real previous release binary
mkdir -p "$WORK/old"; tar -xzf "$OLD_TARBALL" -C "$WORK/old"
OLDBIN="$(find "$WORK/old" -type f -name chill-server | head -1)"
[[ "$(sha "$OLDBIN")" == "$OLD_SHA" ]] || { echo "ancien binaire inattendu : $(sha "$OLDBIN")"; exit 2; }
NEW_SHA="$(sha "$NEWBIN_SRC")"; WEB_SHA="$(sha "$WEB_TAR")"
# fake systemctl: only what the operator script calls, driving the real binary as the service user
cat > "$FAKE/systemctl" <<'EOF'
#!/usr/bin/env bash
svc_pid() { pgrep -f '^/usr/lib/chillcenter/chill-server --config' | head -1; }
case "$1" in
  is-active) [[ -n "$(svc_pid)" ]] ;;
  stop) pid="$(svc_pid)"; [[ -n "$pid" ]] && kill "$pid"; for _ in $(seq 1 50); do [[ -z "$(svc_pid)" ]] && exit 0; sleep 0.1; done; exit 1 ;;
  start) cd /usr/lib/chillcenter; setsid runuser -u chillcenter -- /usr/lib/chillcenter/chill-server --config /etc/chillcenter/server.json >/run/fake-cc/service.log 2>&1 </dev/null &
         for _ in $(seq 1 50); do [[ -n "$(svc_pid)" ]] && exit 0; sleep 0.1; done; exit 1 ;;
  show) svc_pid ;;
  *) exit 1 ;;
esac
EOF
chmod +x "$FAKE/systemctl"
export PATH="$FAKE:$PATH"
svc_pid() { pgrep -f '^/usr/lib/chillcenter/chill-server --config' | head -1; }
stop_service() { systemctl stop chillcenter >/dev/null 2>&1; pkill -f '^/usr/lib/chillcenter/chill-server --config' 2>/dev/null; sleep 0.3; }
wait_api() { for _ in $(seq 1 60); do curl -s --cacert "$CERT" -o /dev/null "$BASE/api/2026-10-14/server" </dev/null && return 0; sleep 0.5; done; return 1; }

# ---- helpers -----------------------------------------------------------------------------------------
reset_state() {  # old binary running, no web dir, rollback dir with the R7 stand-in only
  stop_service; rm -rf "$WEB" /usr/lib/chillcenter/.web-new.* /usr/lib/chillcenter/.chill-server-* "$RB"
  install -o root -g root -m 0755 "$OLDBIN" "$BIN"
  mkdir -m 0700 "$RB"; echo "R7 stand-in" > "$RB/chill-server-r7-9a17ffe55e3d"
  rm -rf /var/lib/chillcenter/data/backups
  systemctl start chillcenter; wait_api || { echo "service de base ne démarre pas"; cat /run/fake-cc/service.log; exit 2; }
}
stage() { install -m 0600 "$NEWBIN_SRC" /var/tmp/chill-server-web.new; install -m 0600 "${1:-$WEB_TAR}" /var/tmp/chillcenter-web.tar; }
run_operator() {  # $1 base url, $2 web archive sha  ->  sets RC and OUT ; script is fed through stdin like the real procedure
  OUT="$(env PATH="$PATH" NEW_BINARY_SHA256="$NEW_SHA" WEB_ARCHIVE_SHA256="${2:-$WEB_SHA}" CHILLCENTER_TLS_CERT_PATH="$CERT" CHILLCENTER_BASE_URL="${1:-$BASE}" /bin/bash -s -- < "$OP" 2>&1)"; RC=$?
}
backups() { ls /var/lib/chillcenter/data/backups 2>/dev/null | wc -l | tr -d ' '; }
broken_archive() {  # a valid, hash-consistent archive that lacks the wasm file => post-install verification must fail
  rm -rf "$WORK/b"; mkdir -p "$WORK/b"; tar -xf "$WEB_TAR" -C "$WORK/b"
  rm -f "$WORK/b/web/vendor/mpeg4/mpeg4-decoder.wasm"
  ( cd "$WORK/b/web" && find . -type f ! -path ./SHA256SUMS -printf '%P\n' | LC_ALL=C sort | xargs sha256sum > SHA256SUMS )
  tar -C "$WORK/b" -cf "$WORK/broken.tar" web; echo "$WORK/broken.tar"
}
unchanged_world() { # config, unit, certificate untouched
  [[ "$(sha "$CFG")" == "$CFG_SHA" && "$(sha "$UNIT")" == "$UNIT_SHA" && "$(sha "$CERT")" == "$CERT_SHA" ]]
}
reset_state; CFG_SHA="$(sha "$CFG")"; UNIT_SHA="$(sha "$UNIT")"; CERT_SHA="$(sha "$CERT")"

echo "== Environnement : $(uname -m) $(. /etc/os-release; echo "$PRETTY_NAME"), tar $(tar --version | head -1), coreutils $(sha256sum --version | head -1 | awk '{print $NF}'), curl $(curl --version | head -1 | awk '{print $2}')"
echo "== ancien binaire $OLD_SHA ; nouveau $NEW_SHA ; archive web $WEB_SHA ; script $(sha "$OP")"

# ---- A. nominal deployment ---------------------------------------------------------------------------
reset_state; stage; before_pid="$(svc_pid)"; run_operator
expect "A  déploiement nominal : code de sortie 0" "$RC" 0
expect "A  binaire installé = nouveau" "$(sha "$BIN")" "$NEW_SHA"
[[ -d "$WEB" && "$(stat -c '%a:%u:%g' "$WEB")" == "755:0:0" && -f "$WEB/index.html" ]] && ok "A  dossier web installé, root:root 755" || ko "A  dossier web" "absent ou droits inattendus"
expect "A  copie de rollback = ancien binaire" "$(sha "$RB/chill-server-1eb24e97bcdf" 2>/dev/null)" "$OLD_SHA"
[[ -n "$(svc_pid)" && "$(svc_pid)" != "$before_pid" ]] && ok "A  service actif avec un nouveau processus" || ko "A  service" "pid inchangé ou arrêté"
expect "A  une sauvegarde privée créée" "$(backups)" 1
unchanged_world && ok "A  config, unité et certificat inchangés" || ko "A  invariants" "modifiés"
echo "$OUT" | grep -q "^installed_hash=$NEW_SHA" && ok "A  sortie : hash installé" || ko "A  sortie" "$OUT"
[[ "$(curl -s --cacert "$CERT" -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: cross-site' "$BASE/api/2026-10-14/server")" == 403 ]] && ok "A  cross-site refusé après déploiement" || ko "A  cross-site" "non refusé"
[[ -z "$(ls /var/tmp/.chill* /usr/lib/chillcenter/.web-new.* /usr/lib/chillcenter/.chill-server-web.* 2>/dev/null)" ]] && ok "A  aucun fichier temporaire résiduel" || ko "A  résidus" "$(ls -a /usr/lib/chillcenter)"

# ---- B. wrong URL: refused BEFORE the service is stopped (review S3) ------------------------------------
reset_state; stage; before_pid="$(svc_pid)"; b0="$(backups)"; run_operator "https://127.0.0.1:18444"
expect "B  mauvaise URL : sortie en erreur (2)" "$RC" 2
expect "B  service intact (même processus)" "$(svc_pid)" "$before_pid"
expect "B  ancien binaire toujours installé" "$(sha "$BIN")" "$OLD_SHA"
expect "B  aucune sauvegarde créée, aucun web" "$(backups)|$([[ -e $WEB ]] && echo web || echo none)" "$b0|none"

# ---- C. failure after the swap: automatic restoration (review S2) -----------------------------------------
reset_state; BR="$(broken_archive)"; stage "$BR"; run_operator "$BASE" "$(sha "$BR")"
[[ "$RC" != 0 ]] && ok "C  vérification en échec : sortie non nulle ($RC)" || ko "C  code de sortie" "0 alors qu'un fichier manque"
expect "C  ancien binaire restauré" "$(sha "$BIN")" "$OLD_SHA"
[[ ! -e "$WEB" ]] && ok "C  dossier web retiré (n'existait pas avant)" || ko "C  web" "toujours présent"
[[ -n "$(svc_pid)" ]] && wait_api && ok "C  service actif et répond (ancien binaire)" || ko "C  service" "arrêté"
echo "$OUT" | grep -q "RECOVERY: service is active" && ok "C  message de récupération affiché" || ko "C  message" "absent"
unchanged_world && ok "C  config, unité et certificat inchangés" || ko "C  invariants" "modifiés"

# ---- D. failure when a web directory already exists: it must come back (review S5) --------------------------
reset_state; mkdir -p "$WEB"; echo "version précédente" > "$WEB/index.html"; echo precedent > "$WEB/MARQUEUR"; chmod -R go-w "$WEB"
BR="$(broken_archive)"; stage "$BR"; run_operator "$BASE" "$(sha "$BR")"
[[ "$RC" != 0 ]] && ok "D  échec attendu : sortie non nulle" || ko "D  code" "0"
[[ "$(cat "$WEB/MARQUEUR" 2>/dev/null)" == precedent ]] && ok "D  ancien dossier web remis en place" || ko "D  web précédent" "non restauré : $(ls "$WEB" "$RB" 2>&1 | tr '\n' ' ')"
expect "D  ancien binaire restauré" "$(sha "$BIN")" "$OLD_SHA"
[[ -n "$(svc_pid)" ]] && ok "D  service actif" || ko "D  service" "arrêté"

# ---- E. redeploy over an existing web directory + reuse of an identical rollback copy (review S4) ------------
reset_state; mkdir -p "$WEB"; echo old > "$WEB/MARQUEUR"; install -o root -g root -m 0755 "$OLDBIN" "$RB/chill-server-1eb24e97bcdf"; stage; run_operator
expect "E  redéploiement : code 0" "$RC" 0
[[ -f "$WEB/index.html" && ! -e "$WEB/MARQUEUR" ]] && ok "E  nouveau web installé" || ko "E  web" "contenu inattendu"
ls "$RB"/web-*/MARQUEUR >/dev/null 2>&1 && ok "E  ancien web conservé dans le répertoire de rollback" || ko "E  web précédent" "perdu"

# ---- F. rollback copy with unexpected content: refused BEFORE the stop (review S4) -------------------------
reset_state; echo corrompu > "$RB/chill-server-1eb24e97bcdf"; stage; before_pid="$(svc_pid)"; run_operator
expect "F  copie de rollback douteuse : refus (2)" "$RC" 2
expect "F  service intact" "$(svc_pid)" "$before_pid"

# ---- G. an established client connection blocks the stop ---------------------------------------------------
reset_state; stage; before_pid="$(svc_pid)"; exec 7<>/dev/tcp/127.0.0.1/18443; sleep 0.5; run_operator; exec 7>&- 7<&-
expect "G  connexion cliente établie : refus (42)" "$RC" 42
expect "G  service intact" "$(svc_pid)" "$before_pid"
expect "G  ancien binaire toujours installé" "$(sha "$BIN")" "$OLD_SHA"

# ---- H. hostile archive (symlink member) refused before anything happens --------------------------------------
reset_state; rm -rf "$WORK/h"; mkdir -p "$WORK/h/web"; echo x > "$WORK/h/web/index.html"; ln -s /etc/passwd "$WORK/h/web/link"; ( cd "$WORK/h/web" && sha256sum index.html > SHA256SUMS ); tar -C "$WORK/h" -cf "$WORK/hostile.tar" web
stage "$WORK/hostile.tar"; before_pid="$(svc_pid)"; run_operator "$BASE" "$(sha "$WORK/hostile.tar")"
expect "H  archive avec lien symbolique : refus (2)" "$RC" 2
expect "H  service intact" "$(svc_pid)" "$before_pid"

# ---- teardown ---------------------------------------------------------------------------------------------
stop_service; rm -rf "$WEB" "$RB" /var/lib/chillcenter /usr/lib/chillcenter "$CFG" /etc/chillcenter "$UNIT" /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar "$FAKE" /run/fake-cc "$WORK"
userdel chillcenter 2>/dev/null; groupdel chillcenter 2>/dev/null
echo "== résultat : $pass PASS, $fail FAIL"; [[ "$fail" -eq 0 ]]
