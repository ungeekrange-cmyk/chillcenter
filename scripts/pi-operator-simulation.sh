#!/usr/bin/env bash
# Simulation of the Pi deployment on a THROWAWAY Linux VM (Colima Ubuntu, aarch64), as root.
# It recreates the Pi layout (uid 997, /usr/lib/chillcenter, /etc/chillcenter, rollback dir, data dir), runs the REAL
# previous binary (the one given as OLD_SHA, default cef54cd0… = build c610ea9 on the Pi) as the "service" through a fake systemctl, then feeds scripts/pi-update-web-operator.sh
# through stdin exactly like the real procedure and checks success and failure paths. Touches no real host.
#   usage (from the Mac):  colima ssh -- sudo -n <repo>/scripts/pi-operator-simulation.sh <repo> <old-release.tar.gz> [expected-old-sha256]
set -uo pipefail
REPO="$1"; OLD_TARBALL="$2"
OP="$REPO/scripts/pi-update-web-operator.sh"; ORIG_OP="$OP"
NEWBIN_SRC="$REPO/target/aarch64-unknown-linux-gnu/release/chill-server"; WEB_TAR="$REPO/artifacts/web-pi/chillcenter-web.tar"
OLD_SHA="${3:-cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3}"; OLD_RB="chill-server-${OLD_SHA:0:12}"
BIN=/usr/lib/chillcenter/chill-server; WEB=/usr/lib/chillcenter/web; RB=/root/chillcenter-rollback; CFG=/etc/chillcenter/server.json
UNIT=/usr/lib/systemd/system/chillcenter.service; BASE=https://127.0.0.1:18443; CERT=/var/lib/chillcenter/data/tls/certificate.pem
FAKE=/opt/fake-cc-bin; WORK="$(mktemp -d /tmp/cc-sim.XXXXXX)"; pass=0; fail=0
[[ "$(id -u)" == 0 ]] || { echo "must be root"; exit 2; }
teardown() {
  pkill -f '^/usr/lib/chillcenter/chill-server --config' 2>/dev/null; pkill -f 'runuser -u chillcenter' 2>/dev/null; sleep 0.5
  rm -rf "$WEB" "$RB" /var/lib/chillcenter /usr/lib/chillcenter "$CFG" /etc/chillcenter "$UNIT" /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar "$FAKE" /run/fake-cc "${WORK:-/nonexistent}"
  userdel chillcenter 2>/dev/null; groupdel chillcenter 2>/dev/null
}
teardown; WORK="$(mktemp -d /tmp/cc-sim.XXXXXX)"   # purge anything left by an interrupted run, then start clean
trap teardown EXIT
sha() { sha256sum "$1" | awk '{print $1}'; }
ok() { pass=$((pass+1)); echo "PASS  $1"; }
ko() { fail=$((fail+1)); echo "FAIL  $1 — $2"; [[ -n "${OUT:-}" ]] && echo "$OUT" | tail -${SIM_TAIL:-12} | sed "s/^/        | /"; }
expect() { if [[ "$2" == "$3" ]]; then ok "$1"; else ko "$1" "attendu '$3', obtenu '$2'"; fi; }

# ---- the Pi layout ------------------------------------------------------------------------------------
# The Pi's service account is uid 997 / gid 983. If those ids are taken on this VM, the ONLY change made to the tested
# script is a mechanical substitution of that number in a COPY (reported below); everything else runs byte for byte.
SIM_UID=997; SIM_GID=983
if getent passwd 997 >/dev/null && [[ "$(getent passwd 997 | cut -d: -f1)" != chillcenter ]]; then SIM_UID=4997; SIM_GID=4983; fi
getent group chillcenter >/dev/null || groupadd -g "$SIM_GID" chillcenter
getent passwd chillcenter >/dev/null || useradd -r -u "$SIM_UID" -g "$SIM_GID" -M -s /usr/sbin/nologin chillcenter
if [[ "$SIM_UID" != 997 ]]; then
  sed -e "s/'997'/'$SIM_UID'/g" -e "s/700:997/700:$SIM_UID/g" "$OP" > "$WORK/operator-copy.sh"
  echo "NOTE : uid 997 indisponible sur la VM ; copie du script avec uid $SIM_UID (seule différence : $(diff <(cat "$OP") "$WORK/operator-copy.sh" | grep -c '^>') lignes)"
  OP="$WORK/operator-copy.sh"
fi
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
  stop) pid="$(svc_pid)"; [[ -n "$pid" ]] && kill "$pid"; for _ in $(seq 1 900); do [[ -z "$(svc_pid)" ]] && exit 0; sleep 0.1; done; exit 1 ;;  # systemd waits up to TimeoutStopSec (90 s)
  start) cd /usr/lib/chillcenter; setsid env --default-signal=HUP,INT,TERM,PIPE runuser -u chillcenter -- /usr/lib/chillcenter/chill-server --config /etc/chillcenter/server.json >/run/fake-cc/service.log 2>&1 </dev/null &
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
  systemctl start chillcenter; wait_api || { echo "service de base ne démarre pas"; cat /run/fake-cc/service.log; ss -ltnp | grep 18443; exit 2; }
}
stage() { install -m 0600 "$NEWBIN_SRC" /var/tmp/chill-server-web.new; install -m 0600 "${1:-$WEB_TAR}" /var/tmp/chillcenter-web.tar; }
run_operator() {  # $1 base url, $2 web archive sha  ->  sets RC and OUT ; script is fed through stdin like the real procedure
  OUT="$(env PATH="$PATH" OLD_BINARY_SHA256="${OLD_PARAM-$OLD_SHA}" NEW_BINARY_SHA256="$NEW_SHA" WEB_ARCHIVE_SHA256="${2:-$WEB_SHA}" CHILLCENTER_TLS_CERT_PATH="$CERT" CHILLCENTER_BASE_URL="${1:-$BASE}" /bin/bash -s${SIM_TRACE:+x} -- < "$OP" 2>&1)"; RC=$?
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
echo "== ancien binaire $OLD_SHA ; nouveau $NEW_SHA ; archive web $WEB_SHA ; script testé $(sha "$OP") (original du dépôt : $(sha "$ORIG_OP"))"

# ---- A. nominal deployment ---------------------------------------------------------------------------
reset_state; stage; before_pid="$(svc_pid)"; run_operator
expect "A  déploiement nominal : code de sortie 0" "$RC" 0
expect "A  binaire installé = nouveau" "$(sha "$BIN")" "$NEW_SHA"
[[ -d "$WEB" && "$(stat -c '%a:%u:%g' "$WEB")" == "755:0:0" && -f "$WEB/index.html" ]] && ok "A  dossier web installé, root:root 755" || ko "A  dossier web" "absent ou droits inattendus"
expect "A  copie de rollback = ancien binaire" "$(sha "$RB/$OLD_RB" 2>/dev/null)" "$OLD_SHA"
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
reset_state; mkdir -p "$WEB"; echo old > "$WEB/MARQUEUR"; install -o root -g root -m 0755 "$OLDBIN" "$RB/$OLD_RB"; stage; run_operator
expect "E  redéploiement : code 0" "$RC" 0
[[ -f "$WEB/index.html" && ! -e "$WEB/MARQUEUR" ]] && ok "E  nouveau web installé" || ko "E  web" "contenu inattendu"
ls "$RB"/web-*/MARQUEUR >/dev/null 2>&1 && ok "E  ancien web conservé dans le répertoire de rollback" || ko "E  web précédent" "perdu"

# ---- F. rollback copy with unexpected content: refused BEFORE the stop (review S4) -------------------------
reset_state; echo corrompu > "$RB/$OLD_RB"; stage; before_pid="$(svc_pid)"; run_operator
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

# ---- I. lost output channel (SSH dropped) while a verification fails: SIGPIPE must not kill the recovery (review 2, N1) --------
reset_state; BR="$(broken_archive)"; stage "$BR"
env PATH="$PATH" OLD_BINARY_SHA256="$OLD_SHA" NEW_BINARY_SHA256="$NEW_SHA" WEB_ARCHIVE_SHA256="$(sha "$BR")" CHILLCENTER_TLS_CERT_PATH="$CERT" CHILLCENTER_BASE_URL="$BASE" /bin/bash -s -- < "$OP" 2>&1 | { read -r _first; exit 0; }
RC=${PIPESTATUS[0]}
[[ "$RC" != 0 && "$RC" != 141 ]] && ok "I  sortie non nulle et pas 141 (SIGPIPE) : $RC" || ko "I  code" "$RC"
expect "I  ancien binaire restauré malgré la sortie fermée" "$(sha "$BIN")" "$OLD_SHA"
[[ ! -e "$WEB" ]] && ok "I  dossier web retiré" || ko "I  web" "toujours présent"
[[ -n "$(svc_pid)" ]] && wait_api && ok "I  service actif et répond" || ko "I  service" "arrêté"

# ---- J. backup fails AFTER the stop: the old service must come back -----------------------------------------------------------
reset_state; stage; mkdir -p /var/lib/chillcenter/data/backups; chown root:root /var/lib/chillcenter/data/backups; chmod 0555 /var/lib/chillcenter/data/backups
before_pid="$(svc_pid)"; run_operator
[[ "$RC" != 0 ]] && ok "J  sauvegarde impossible : sortie non nulle ($RC)" || ko "J  code" "0"
expect "J  ancien binaire intact" "$(sha "$BIN")" "$OLD_SHA"
[[ -n "$(svc_pid)" ]] && ok "J  service redémarré après l'arrêt" || ko "J  service" "reste arrêté"
echo "$OUT" | grep -q "ERR line" && ok "J  la sortie nomme la commande en échec (ERR line …)" || ko "J  diagnostic" "pas de ligne ERR"

# ---- N. service already stopped: refused before anything, and the script never starts it ----------------------------------------
reset_state; stage; stop_service; run_operator
expect "N  service arrêté : refus (2)" "$RC" 2
[[ -z "$(svc_pid)" ]] && ok "N  le script n'a pas démarré le service" || ko "N  service" "démarré"
expect "N  ancien binaire intact, aucun web" "$(sha "$BIN")|$([[ -e $WEB ]] && echo web || echo none)" "$OLD_SHA|none"

# ---- P. the old-binary hash is a mandatory, exact parameter (V8) ----------------------------------------------------------------
reset_state; stage; before_pid="$(svc_pid)"; OLD_PARAM=""; run_operator; unset OLD_PARAM
expect "P  OLD_BINARY_SHA256 vide : refus (1, comme toute variable obligatoire absente)" "$RC" 1
echo "$OUT" | grep -q "OLD_BINARY_SHA256: hash of the binary currently installed" && ok "P  message nommant OLD_BINARY_SHA256" || ko "P  message" "absent"
expect "P  service intact (paramètre vide)" "$(svc_pid)" "$before_pid"
OLD_PARAM="1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181"; run_operator; unset OLD_PARAM
expect "P  mauvais ancien hash : refus (2) avant tout arrêt" "$RC" 2
echo "$OUT" | grep -q "installed binary is not the expected one" && ok "P  message : le binaire installé n'est pas celui attendu" || ko "P  message" "absent"
expect "P  service intact (mauvais hash)" "$(svc_pid)" "$before_pid"
expect "P  aucun web, aucune sauvegarde" "$([[ -e $WEB ]] && echo web || echo none)|$(backups)" "none|0"

# ---- teardown (also runs on any early exit) ------------------------------------------------------------
echo "== résultat : $pass PASS, $fail FAIL"; [[ "$fail" -eq 0 ]]
