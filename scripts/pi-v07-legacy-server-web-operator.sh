#!/usr/bin/env bash
# Legacy v0.7 bridge. No first-install, updater/timer, unit, config, TLS, account,
# mount, NAS or update-policy change. Review exact bytes and authorize target/SHA first.
set -Eeuo pipefail
trap '' PIPE
export PATH=/usr/sbin:/usr/bin:/sbin:/bin
die(){ printf '%s
' "$1" >&2; exit "$2"; }
hash(){ sha256sum -- "$1" | awk '{print $1}'; }
regular(){ [[ -f "$1" && ! -L "$1" ]] || die regular_file_required 2; }
directory(){ [[ -d "$1" && ! -L "$1" ]] || die directory_required 2; }
tls(){ curl -q -sS --noproxy '*' --max-time 10 --cacert "$CC_CERT" "$@" </dev/null; }
build(){ python3 -I -c 'import json,sys; v=json.load(sys.stdin)["build"]; assert isinstance(v,str) and len(v)==12 and all(x in "0123456789abcdef" for x in v); print(v)'; }
safe_zip(){ regular "$1"; unzip -Z1 "$1" | while IFS= read -r x; do [[ -n "$x" && "$x" != /* && "$x" != *..* ]]; done; ! unzip -Z -v "$1" | grep -Eq 'Unix file attributes .*: 120[0-7]{3}'; }
backup_ok(){ python3 -I - "$1" "$CC_DATA_DIR" <<'PY'
import hashlib,json,os,pathlib,stat,sys
root,data=map(pathlib.Path,sys.argv[1:]); s=os.lstat(root)
assert stat.S_ISDIR(s.st_mode) and not stat.S_ISLNK(s.st_mode) and stat.S_IMODE(s.st_mode)&0o077==0
m=json.loads((root/'manifest.json').read_bytes()); assert m['format_version']==1 and m['schema_version']==3 and m['data_root']==str(data)
d={x['path']:x for x in m['entries']}; assert {'catalog.sqlite','server-config.json','secrets/tls/certificate.pem','secrets/tls/service-key.pem'}<=d.keys()
a=set()
for p in root.rglob('*'):
 if p.is_dir(): continue
 z=os.lstat(p); assert stat.S_ISREG(z.st_mode) and not stat.S_ISLNK(z.st_mode); a.add(str(p.relative_to(root)))
assert a==set(d)|{'manifest.json'} and not (root/'catalog.sqlite-wal').exists() and not (root/'catalog.sqlite-shm').exists()
for n,x in d.items(): p=root/n; assert p.stat().st_size==x['size'] and hashlib.sha256(p.read_bytes()).hexdigest()==x['sha256']
PY
}
self_test(){ [[ "$(printf '{"build":"a1c12ba4a582"}'|build)" == a1c12ba4a582 ]]; ! printf '{"build":"x"}'|build >/dev/null 2>&1; printf 'pi_v07_legacy_operator_self_test=PASS
'; }
[[ "$1" != --self-test ]] || { self_test; exit 0; }
: "$CC_SERVICE" "$CC_SERVICE_USER" "$CC_BINARY" "$CC_WEB_DIR" "$CC_CONFIG" "$CC_DATA_DIR" "$CC_ROLLBACK_DIR" "$CC_CERT" "$CC_BASE_URL" "$CC_SERVER_FILE" "$CC_WEB_ZIP" "$CC_SERVER_MANIFEST" "$CC_SERVER_MANIFEST_SIG" "$CC_WEB_MANIFEST" "$CC_WEB_MANIFEST_SIG" "$CC_RELEASE_PUB" "$CC_EXPECTED_RELEASE_PUB_SHA256" "$CC_EXPECTED_KEY_ID" "$CC_EXPECTED_OLD_BINARY_SHA256" "$CC_EXPECTED_NEW_BINARY_SHA256" "$CC_EXPECTED_WEB_SHA256" "$CC_EXPECTED_VERSION" "$CC_EXPECTED_WEB_BUILD"
[[ "$(id -u)" == 0 && "$CC_BASE_URL" =~ ^https://[^/]+$ ]] || die root_or_https_required 2
for t in curl minisign mktemp mv python3 runuser sha256sum ss stat systemctl unzip zipinfo; do command -v "$t" >/dev/null || die missing_tool 2; done
for p in "$CC_BINARY" "$CC_CONFIG" "$CC_CERT" "$CC_SERVER_FILE" "$CC_WEB_ZIP" "$CC_SERVER_MANIFEST" "$CC_SERVER_MANIFEST_SIG" "$CC_WEB_MANIFEST" "$CC_WEB_MANIFEST_SIG" "$CC_RELEASE_PUB"; do [[ "$p" == /* ]] || die absolute_path_required 2; regular "$p"; done
for p in "$CC_WEB_DIR" "$CC_DATA_DIR" "$CC_ROLLBACK_DIR"; do [[ "$p" == /* ]] || die absolute_path_required 2; directory "$p"; done
[[ "$(stat -c '%a:%u:%g' "$CC_ROLLBACK_DIR")" == 700:0:0 && "$(hash "$CC_RELEASE_PUB")" == "$CC_EXPECTED_RELEASE_PUB_SHA256" ]] || die private_rollback_or_key_mismatch 2
minisign -Vm "$CC_SERVER_MANIFEST" -x "$CC_SERVER_MANIFEST_SIG" -p "$CC_RELEASE_PUB" >/dev/null
minisign -Vm "$CC_WEB_MANIFEST" -x "$CC_WEB_MANIFEST_SIG" -p "$CC_RELEASE_PUB" >/dev/null
python3 -I - "$CC_SERVER_MANIFEST" "$CC_WEB_MANIFEST" "$CC_EXPECTED_VERSION" "$CC_EXPECTED_NEW_BINARY_SHA256" "$CC_EXPECTED_WEB_SHA256" "$CC_EXPECTED_KEY_ID" <<'PY'
import json,pathlib,sys
s,w=[json.loads(pathlib.Path(x).read_bytes()) for x in sys.argv[1:3]]; v,b,z,k=sys.argv[3:]
assert s['schema_version']==w['schema_version']==1 and s['component']=='server' and w['component']=='web'
assert s['version']==w['version']==v and s['key_id']==w['key_id']==k
assert s['compatibility']['api']==w['compatibility']['api']=={'min':'2026.10.14','max':'2026.10.14'}
assert all(s['compatibility']['database'][x]==3 for x in ('current_schema','target_schema','read_min','read_max','write_min','write_max','rollback_min','rollback_max'))
assert w['compatibility']['required_server']['min']==v
assert any(x['target']=='linux-aarch64' and x['sha256']==b for x in s['assets'])
assert any(x['target']=='web' and x['sha256']==z for x in w['assets'])
PY
[[ "$(hash "$CC_BINARY")" == "$CC_EXPECTED_OLD_BINARY_SHA256" && "$(hash "$CC_SERVER_FILE")" == "$CC_EXPECTED_NEW_BINARY_SHA256" && "$(hash "$CC_WEB_ZIP")" == "$CC_EXPECTED_WEB_SHA256" ]] || die artifact_hash_mismatch 2
safe_zip "$CC_WEB_ZIP" || die unsafe_web_zip 2
[[ "$(systemctl is-active "$CC_SERVICE" || true)" == active ]] || die service_not_active 2
tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server"|grep -qx 200 || die strict_tls_preflight_failed 2
tls "$CC_BASE_URL/api/2026-10-14/admin/scan"|python3 -I -c 'import json,sys; assert json.load(sys.stdin).get("running") is False' || die scan_active_or_unknown 42
python3 -I - "$CC_DATA_DIR/tmdb-jobs.json" <<'PY'
import json,os,stat,sys
try:s=os.lstat(sys.argv[1])
except FileNotFoundError:raise SystemExit(0)
assert stat.S_ISREG(s.st_mode) and s.st_size<=2097152
for x in json.load(open(sys.argv[1],'rb')).values():assert x.get('status') in {'completed','failed','cancelled'}
PY
sockets="$(ss -Htnp state established 2>/dev/null)" || die socket_inspection_failed 2
[[ -z "$(grep -F "$(basename "$CC_BINARY")" <<<"$sockets" || true)" ]] || die active_client_socket_refusing_stop 42
python3 -I - "$CC_DATA_DIR/catalog.sqlite" <<'PY'
import sqlite3,sys
c=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro&immutable=1',uri=True)
assert c.execute('pragma user_version').fetchone()[0]==3 and c.execute('pragma quick_check').fetchone()[0]=='ok'
PY
pre="$(mktemp -d "$CC_ROLLBACK_DIR/pre-v07.XXXXXX")"; chmod 0700 "$pre"
{ for p in "$CC_BINARY" "$CC_CONFIG" "$CC_CERT" "$CC_DATA_DIR/library.json" "$CC_WEB_DIR/build.json"; do stat -c '%n|%a|%u|%g|%s|%Y' "$p"; hash "$p"; done; systemctl show "$CC_SERVICE" -p User -p Group -p ExecStart -p FragmentPath; } >"$pre/before.fingerprint"
[[ "$1" != --preflight ]] || { printf 'preflight=PASS
fingerprint=%s
' "$pre/before.fingerprint"; exit 0; }
new_bin="$(mktemp "$CC_ROLLBACK_DIR/.new-server.XXXXXX")"; install -o root -g root -m 0755 "$CC_SERVER_FILE" "$new_bin"
new_web="$(mktemp -d "$CC_ROLLBACK_DIR/.new-web.XXXXXX")"; unzip -qq "$CC_WEB_ZIP" -d "$new_web"; [[ "$(build <"$new_web/build.json")" == "$CC_EXPECTED_WEB_BUILD" ]] || die web_build_mismatch 2
find "$new_web" -type d -exec chmod 0755 {} +; find "$new_web" -type f -exec chmod 0644 {} +; chown -R root:root "$new_web"
old_bin="$CC_ROLLBACK_DIR/chill-server-$(date -u +%Y%m%dT%H%M%SZ)"; old_web="$CC_ROLLBACK_DIR/web-$(date -u +%Y%m%dT%H%M%SZ)"; state=running; bin_swapped=0; web_swapped=0
recover(){ rc=$?; set +e; trap '' HUP INT TERM; trap - EXIT; [[ "$state" == complete ]] || { systemctl stop "$CC_SERVICE"||true; ((web_swapped))&&mv -T "$CC_WEB_DIR" "$CC_ROLLBACK_DIR/web-failed-$(date -u +%Y%m%dT%H%M%SZ)"||true; [[ -d "$old_web" ]]&&mv -T "$old_web" "$CC_WEB_DIR"||true; ((bin_swapped))&&install -o root -g root -m 0755 "$old_bin" "$CC_BINARY"||true; systemctl start "$CC_SERVICE"||true; printf 'recovery_attempted=1
' >&2; }; exit "$rc"; }
trap recover EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
systemctl stop "$CC_SERVICE"; state=stopped; ! systemctl is-active --quiet "$CC_SERVICE" || die service_failed_to_stop 1
out="$(runuser -u "$CC_SERVICE_USER" -- /bin/bash -c 'cd "$1"&&exec "$2" --backup --config "$3"' -- /usr/lib/chillcenter "$CC_BINARY" "$CC_CONFIG" </dev/null)"
backup="${out#Sauvegarde terminée : }"; [[ "$out" != "$backup" && "$backup" == "$CC_DATA_DIR/backups/"* ]] || die backup_receipt_refused 1; backup_ok "$backup"
install -o root -g root -m 0755 "$CC_BINARY" "$old_bin"; [[ "$(hash "$old_bin")" == "$CC_EXPECTED_OLD_BINARY_SHA256" ]] || die old_binary_backup_mismatch 1
mv -T "$CC_WEB_DIR" "$old_web"; mv -T "$new_web" "$CC_WEB_DIR"; web_swapped=1; mv -T "$new_bin" "$CC_BINARY"; bin_swapped=1
systemctl start "$CC_SERVICE"; state=started
for n in $(seq 1 30); do tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server" 2>/dev/null|grep -qx 200&&break; sleep 1; done
tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server"|grep -qx 200 || die new_server_not_ready 1
[[ "$(tls "$CC_BASE_URL/build.json"|build)" == "$CC_EXPECTED_WEB_BUILD" ]] || die new_web_not_served 1
state=complete; trap - EXIT HUP INT TERM
printf 'deployment=PASS
backup=%s
old_binary=%s
old_web=%s
' "$backup" "$old_bin" "$old_web"
