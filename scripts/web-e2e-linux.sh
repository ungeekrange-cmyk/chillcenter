#!/usr/bin/env bash
# Runs inside the Colima Linux/aarch64 VM: starts the REAL release binary with the REAL web archive next to it
# (loopback only, temporary data dir) and replays the checks of scripts/pi-update-web-operator.sh.
set -uo pipefail
REPO="$1"; WORK="$(mktemp -d /tmp/cc-e2e.XXXXXX)"
mkdir -p "$WORK/bin" "$WORK/data"; chmod 700 "$WORK/data"
cp "$REPO/target/aarch64-unknown-linux-gnu/release/chill-server" "$WORK/bin/chill-server"
tar -xf "$REPO/artifacts/web-pi/chillcenter-web.tar" -C "$WORK/bin"
cat > "$WORK/server.json" <<EOF
{ "listen": "127.0.0.1:18443", "http_listen": "127.0.0.1:18080", "data_dir": "$WORK/data", "tls_names": ["127.0.0.1", "localhost"] }
EOF
chmod 640 "$WORK/server.json"
echo "uname: $(uname -m) $(. /etc/os-release; echo "$PRETTY_NAME"), glibc $(ldd --version | head -1 | awk '{print $NF}')"
"$WORK/bin/chill-server" --config "$WORK/server.json" > "$WORK/server.log" 2>&1 &
PID=$!
CERT="$WORK/data/tls/certificate.pem"
for _ in $(seq 1 40); do [ -s "$CERT" ] && curl -s --cacert "$CERT" -o /dev/null "https://127.0.0.1:18443/api/2026-10-14/server" && break; sleep 0.5; done
B=https://127.0.0.1:18443; pass=0; failn=0
check() { local name="$1" want="$2" got="$3"; if [ "$got" = "$want" ]; then pass=$((pass+1)); echo "PASS $name ($got)"; else failn=$((failn+1)); echo "FAIL $name: attendu $want, obtenu $got"; fi; }
c() { curl -sS --max-time 10 --cacert "$CERT" "$@"; }
check "api sans en-tête (clients natifs)"        200 "$(c -o /dev/null -w '%{http_code}' $B/api/2026-10-14/server)"
check "api same-origin (client web)"             200 "$(c -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: same-origin' $B/api/2026-10-14/server)"
check "api same-origin + Origin identique"       200 "$(c -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: same-origin' -H 'Origin: https://127.0.0.1:18443' $B/api/2026-10-14/server)"
check "Host sans port (443 redirigé, ADR-0017)"  200 "$(c -o /dev/null -w '%{http_code}' -H 'Host: 127.0.0.1' -H 'Sec-Fetch-Site: same-origin' $B/api/2026-10-14/server)"
check "Host port 443 explicite"                  200 "$(c -o /dev/null -w '%{http_code}' -H 'Host: 127.0.0.1:443' $B/api/2026-10-14/server)"
check "Host autre port refusé"                   403 "$(c -o /dev/null -w '%{http_code}' -H 'Host: 127.0.0.1:9443' $B/api/2026-10-14/server)"
check "api cross-site"                           403 "$(c -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: cross-site' $B/api/2026-10-14/server)"
check "api same-site"                            403 "$(c -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: same-site' $B/api/2026-10-14/server)"
check "api Origin étranger"                      403 "$(c -o /dev/null -w '%{http_code}' -H 'Origin: https://evil.example' $B/api/2026-10-14/server)"
check "api navigation saisie (none)"             403 "$(c -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: none' $B/api/2026-10-14/server)"
check "page d'accueil (navigation saisie)"       200 "$(c -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: none' $B/)"
check "titre de la page"                         yes "$(c $B/ | grep -q '<title>Chill Center</title>' && echo yes || echo no)"
check "CSP sur la page"                          yes "$(c -D - -o /dev/null $B/ | grep -qi '^content-security-policy:' && echo yes || echo no)"
check "wasm : type MIME"                         application/wasm "$(c -o /dev/null -w '%{content_type}' $B/vendor/mpeg4/mpeg4-decoder.wasm)"
check "wasm : taille servie"                     652895 "$(c -o /dev/null -w '%{size_download}' $B/vendor/mpeg4/mpeg4-decoder.wasm)"
check "service worker (no-cache)"                yes "$(c -D - -o /dev/null $B/sw.js | grep -qi '^cache-control: no-cache' && echo yes || echo no)"
check "fichier caché .env"                       404 "$(c -o /dev/null -w '%{http_code}' $B/.env)"
check "traversée ../"                            404 "$(c --path-as-is -o /dev/null -w '%{http_code}' $B/../server.json)"
check "traversée encodée %2e%2e"                 404 "$(c --path-as-is -o /dev/null -w '%{http_code}' "$B/%2e%2e/server.json")"
check "SHA256SUMS non servi hors liste MIME"     404 "$(c -o /dev/null -w '%{http_code}' $B/SHA256SUMS)"
check "méthode POST sur fichier statique"        405 "$(c -X POST -o /dev/null -w '%{http_code}' $B/index.html)"
check "HEAD sur page"                            200 "$(c -I -o /dev/null -w '%{http_code}' $B/index.html)"
check "hôte inattendu refusé"                    403 "$(c -o /dev/null -w '%{http_code}' -H 'Host: attacker.example:18443' $B/)"
# ---- plain-HTTP listener (ADR-0018): same web client and API, but only its own origin and ports ----------------------------------------
H=http://127.0.0.1:18080; h() { curl -sS --max-time 10 "$@"; }
check "http : page d'accueil"                    200 "$(h -o /dev/null -w '%{http_code}' $H/)"
check "http : décodeur wasm (type MIME)"         "application/wasm" "$(h -o /dev/null -w '%{content_type}' $H/vendor/mpeg4/mpeg4-decoder.wasm)"
check "http : API sans en-tête"                  200 "$(h -o /dev/null -w '%{http_code}' $H/api/2026-10-14/server)"
check "http : Origin propre sans Sec-Fetch"      200 "$(h -o /dev/null -w '%{http_code}' -H 'Origin: http://127.0.0.1:18080' $H/api/2026-10-14/server)"
check "http : Origin étrangère"                  403 "$(h -o /dev/null -w '%{http_code}' -H 'Origin: http://attacker.example' $H/api/2026-10-14/server)"
check "http : Origin en https refusée"           403 "$(h -o /dev/null -w '%{http_code}' -H 'Origin: https://127.0.0.1:18080' $H/api/2026-10-14/server)"
check "http : cross-site"                        403 "$(h -o /dev/null -w '%{http_code}' -H 'Sec-Fetch-Site: cross-site' $H/api/2026-10-14/server)"
check "http : Host sans port (80 redirigé)"      200 "$(h -o /dev/null -w '%{http_code}' -H 'Host: 127.0.0.1' $H/api/2026-10-14/server)"
check "http : Host du port HTTPS refusé"         403 "$(h -o /dev/null -w '%{http_code}' -H 'Host: 127.0.0.1:18443' $H/api/2026-10-14/server)"
check "http : hôte inattendu refusé"             403 "$(h -o /dev/null -w '%{http_code}' -H 'Host: attacker.example:18080' $H/)"
check "http : écriture, Origin propre (garde passée, corps vide refusé)" 422 "$(h -o /dev/null -w '%{http_code}' -X PUT -H 'Content-Type: application/json' -H 'Origin: http://127.0.0.1:18080' -d '{}' $H/api/2026-10-14/admin/library)"
check "http : écriture avec Origin étrangère"    403 "$(h -o /dev/null -w '%{http_code}' -X PUT -H 'Content-Type: application/json' -H 'Origin: http://attacker.example' -d '{}' $H/api/2026-10-14/admin/library)"
check "https : Origin seule toujours refusée"    403 "$(c -o /dev/null -w '%{http_code}' -H 'Origin: https://127.0.0.1:18443' $B/api/2026-10-14/server)"
check "http : secret TMDB réservé au HTTPS"      403 "$(h -o /dev/null -w '%{http_code}' $H/api/2026-10-14/admin/tmdb)"
check "http : exports réservés au HTTPS"         403 "$(h -o /dev/null -w '%{http_code}' $H/api/2026-10-14/admin/exports/x)"
check "https : TMDB toujours disponible"         200 "$(c -o /dev/null -w '%{http_code}' $B/api/2026-10-14/admin/tmdb)"
# 16 silent TCP connections must not starve the listener (review 6, finding 1): the header timeout (5 s) closes them.
python3 - <<PY
import socket, time, sys, urllib.request
socks = [socket.create_connection(("127.0.0.1", 18080)) for _ in range(16)]
time.sleep(7)
closed = 0
for s in socks:
    s.settimeout(1)
    try:
        if s.recv(1) == b"": closed += 1
    except (ConnectionResetError, BrokenPipeError): closed += 1
    except socket.timeout: pass
try:
    ok = urllib.request.urlopen("http://127.0.0.1:18080/api/2026-10-14/server", timeout=5).status == 200
except Exception as e:
    ok = False
print("idle_closed=%d/16 probe_ok=%s" % (closed, ok))
sys.exit(0 if ok and closed == 16 else 1)
PY
check "http : 16 connexions muettes fermées, service disponible" 0 "$?"
kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null
echo "résultat: $pass PASS, $failn FAIL"; rm -rf "$WORK"; [ "$failn" -eq 0 ]
