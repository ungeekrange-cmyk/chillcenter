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
web_build_dir(){ build <"$1/build.json"; }
safe_zip(){ python3 -I - "$1" <<'PY'
import stat,sys,zipfile
with zipfile.ZipFile(sys.argv[1]) as z:
 assert z.infolist()
 for x in z.infolist():
  assert x.filename and not x.filename.startswith('/') and '..' not in x.filename.split('/')
  mode=x.external_attr >> 16
  assert stat.S_IFMT(mode) in (0,stat.S_IFREG,stat.S_IFDIR)
PY
}
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
self_test(){ [[ "$(printf '{"build":"a1c12ba4a582"}'|build)" == a1c12ba4a582 ]]; ! printf '{"build":"x"}'|build >/dev/null 2>&1; printf 'pi_v07_legacy_operator_self_test=PASS\n'; }
mode="${1:-}"; [[ $# -eq 1 ]] || die usage 2
[[ "$mode" == --self-test ]] && { self_test; exit 0; }
[[ "$mode" == --preflight || "$mode" == --deploy || "$mode" == --recover ]] || die usage 2
: "${CC_SERVICE:?}" "${CC_BINARY:?}" "${CC_WEB_DIR:?}" "${CC_ROLLBACK_DIR:?}" "${CC_CERT:?}" "${CC_BASE_URL:?}" "${CC_EXPECTED_OLD_BINARY_SHA256:?}"
[[ "$(id -u)" == 0 && "$CC_BASE_URL" =~ ^https://[^/]+$ ]] || die root_or_https_required 2
for t in curl mktemp mv python3 runuser sha256sum ss stat systemctl unzip timeout setsid install find chown cmp; do command -v "$t" >/dev/null || die missing_tool 2; done
for p in "$CC_BINARY" "$CC_WEB_DIR" "$CC_ROLLBACK_DIR"; do [[ "$p" == /* ]] || die absolute_path_required 2; done
journal="$CC_ROLLBACK_DIR/v07-transition.json"

# Recovery is deliberately based on the durable receipt and the filesystem, never
# on shell flags.  It is safe to run repeatedly after TERM, SIGKILL, or power loss.
recover_transition(){
 python3 -I - "$journal" "$CC_BINARY" "$CC_WEB_DIR" <<'PY'
import hashlib,json,pathlib,sys
j,b,w=map(pathlib.Path,sys.argv[1:]); d=json.loads(j.read_bytes())
required={'old_binary','new_binary','old_web','new_web','old_hash','new_hash','old_web_build','new_web_build','state'}
assert required <= set(d) and d['state'] in {'prepared','stopping','swapping','started','rollback_complete'}
for k in ('old_binary','new_binary','old_web','new_web'):
 p=pathlib.Path(d[k]); assert p.is_absolute() and str(p).startswith(str(j.parent)+'/')
assert pathlib.Path(d['old_binary']).parent==j.parent and pathlib.Path(d['old_web']).parent==j.parent
assert len(d['old_hash'])==len(d['new_hash'])==64
PY
 local receipt_state old_bin old_web new_bin new_web old_hash old_build new_hash new_build current
 { IFS= read -r receipt_state; IFS= read -r old_bin; IFS= read -r old_web; IFS= read -r new_bin; IFS= read -r new_web; IFS= read -r old_hash; IFS= read -r old_build; IFS= read -r new_hash; IFS= read -r new_build; } < <(python3 -I - "$journal" <<'PY'
import json,sys
d=json.load(open(sys.argv[1])); print(d['state'],*[d[x] for x in ('old_binary','old_web','new_binary','new_web','old_hash','old_web_build','new_hash','new_web_build')],sep='\n')
PY
 )
 # A durable receipt may exist before the final idle gate.  It has touched no
 # service state, so recovery must prove that fact without stopping an active Pi.
 if [[ "$receipt_state" == prepared ]]; then
   [[ "$(hash "$CC_BINARY")" == "$old_hash" && "$(web_build_dir "$CC_WEB_DIR")" == "$old_build" ]] || die prepared_recovery_state_changed 1
   [[ "$(systemctl is-active "$CC_SERVICE" || true)" == active ]] || die prepared_recovery_service_inactive 1
   deadline=$((SECONDS + 60)); while (( SECONDS < deadline )); do tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server" 2>/dev/null | grep -qx 200 && break; sleep 1; done
   tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server" | grep -qx 200 || die prepared_recovery_tls_health_failed 1
   receipt_state=rollback_complete
 else
 systemctl stop "$CC_SERVICE" || die recovery_stop_failed 1
 # Restore the web tree only from an observed, unambiguous state.
 if [[ -d "$old_web" ]]; then
   if [[ -e "$CC_WEB_DIR" ]]; then
     current="$(web_build_dir "$CC_WEB_DIR")" || die recovery_web_state_unreadable 1
     [[ "$current" == "$new_build" ]] || die recovery_web_state_ambiguous 1
     mv -T "$CC_WEB_DIR" "$CC_ROLLBACK_DIR/web-failed-$(date -u +%Y%m%dT%H%M%SZ)" || die recovery_web_quarantine_failed 1
   fi
   mv -T "$old_web" "$CC_WEB_DIR" || die recovery_web_restore_failed 1
 elif [[ -d "$CC_WEB_DIR" && "$(web_build_dir "$CC_WEB_DIR")" == "$old_build" ]]; then :; else die recovery_web_old_copy_missing 1; fi
 # The same logic applies to the binary: never overwrite an unknown executable.
 if [[ -f "$old_bin" ]]; then
   if [[ -e "$CC_BINARY" ]]; then
     current="$(hash "$CC_BINARY")"
     if [[ "$current" == "$new_hash" ]]; then
       mv -T "$CC_BINARY" "$CC_ROLLBACK_DIR/server-failed-$(date -u +%Y%m%dT%H%M%SZ)" || die recovery_binary_quarantine_failed 1
     elif [[ "$current" == "$old_hash" ]]; then
       [[ "$(hash "$old_bin")" == "$old_hash" ]] || die recovery_binary_copy_invalid 1
       old_bin=""
     else die recovery_binary_state_ambiguous 1; fi
   fi
   [[ -z "$old_bin" ]] || mv -T "$old_bin" "$CC_BINARY" || die recovery_binary_restore_failed 1
 elif [[ -f "$CC_BINARY" && "$(hash "$CC_BINARY")" == "$old_hash" ]]; then :; else die recovery_binary_old_copy_missing 1; fi
 [[ "$(hash "$CC_BINARY")" == "$old_hash" && "$(web_build_dir "$CC_WEB_DIR")" == "$old_build" ]] || die recovery_verification_failed 1
 systemctl start "$CC_SERVICE" || die recovery_start_failed 1
 [[ "$(systemctl is-active "$CC_SERVICE" || true)" == active ]] || die recovery_service_inactive 1
 deadline=$((SECONDS + 60)); while (( SECONDS < deadline )); do tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server" 2>/dev/null | grep -qx 200 && break; sleep 1; done
 tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server" | grep -qx 200 || die recovery_tls_health_failed 1
 fi
 python3 -I - "$journal" <<'PY'
import json,os,sys
p=sys.argv[1]; d=json.load(open(p)); d['state']='rollback_complete'; f=open(p+'.new','w'); json.dump(d,f,sort_keys=True); f.flush(); os.fsync(f.fileno()); f.close(); os.replace(p+'.new',p); fd=os.open(os.path.dirname(p),os.O_RDONLY); os.fsync(fd); os.close(fd)
PY
 printf 'recovery=PASS\n'
 rm -f -- "$journal"
}
if [[ "$mode" == --recover ]]; then
 [[ -d "$CC_ROLLBACK_DIR" && ! -L "$CC_ROLLBACK_DIR" && -f "$journal" && ! -L "$journal" ]] || die recovery_journal_required 2
 recover_transition; exit 0
fi
for p in "$CC_BINARY" "$CC_CERT"; do regular "$p"; done
directory "$CC_WEB_DIR"; directory "$CC_ROLLBACK_DIR"
[[ "$(stat -c '%a:%u:%g' "$CC_ROLLBACK_DIR")" == 700:0:0 ]] || die private_rollback_required 2
[[ "$(stat -c '%d' "$CC_ROLLBACK_DIR")" == "$(stat -c '%d' "$CC_BINARY")" && "$(stat -c '%d' "$CC_ROLLBACK_DIR")" == "$(stat -c '%d' "$CC_WEB_DIR")" ]] || die rollback_must_share_target_filesystem 2
: "${CC_SERVICE_USER:?}" "${CC_CONFIG:?}" "${CC_DATA_DIR:?}" "${CC_SERVER_FILE:?}" "${CC_WEB_ZIP:?}" "${CC_SERVER_MANIFEST:?}" "${CC_SERVER_MANIFEST_SIG:?}" "${CC_WEB_MANIFEST:?}" "${CC_WEB_MANIFEST_SIG:?}" "${CC_RELEASE_PUB:?}" "${CC_EXPECTED_RELEASE_PUB_SHA256:?}" "${CC_EXPECTED_SERVER_MANIFEST_SHA256:?}" "${CC_EXPECTED_SERVER_MANIFEST_SIG_SHA256:?}" "${CC_EXPECTED_WEB_MANIFEST_SHA256:?}" "${CC_EXPECTED_WEB_MANIFEST_SIG_SHA256:?}" "${CC_EXPECTED_KEY_ID:?}" "${CC_EXPECTED_NEW_BINARY_SHA256:?}" "${CC_EXPECTED_WEB_SHA256:?}" "${CC_EXPECTED_VERSION:?}" "${CC_EXPECTED_WEB_BUILD:?}"
for p in "$CC_CONFIG" "$CC_SERVER_FILE" "$CC_WEB_ZIP" "$CC_SERVER_MANIFEST" "$CC_SERVER_MANIFEST_SIG" "$CC_WEB_MANIFEST" "$CC_WEB_MANIFEST_SIG" "$CC_RELEASE_PUB"; do [[ "$p" == /* ]] || die absolute_path_required 2; regular "$p"; done
directory "$CC_DATA_DIR"
[[ "$(hash "$CC_RELEASE_PUB")" == "$CC_EXPECTED_RELEASE_PUB_SHA256" ]] || die locally_verified_receipt_mismatch 2
[[ "$(hash "$CC_SERVER_MANIFEST")" == "$CC_EXPECTED_SERVER_MANIFEST_SHA256" && "$(hash "$CC_SERVER_MANIFEST_SIG")" == "$CC_EXPECTED_SERVER_MANIFEST_SIG_SHA256" && "$(hash "$CC_WEB_MANIFEST")" == "$CC_EXPECTED_WEB_MANIFEST_SHA256" && "$(hash "$CC_WEB_MANIFEST_SIG")" == "$CC_EXPECTED_WEB_MANIFEST_SIG_SHA256" ]] || die locally_verified_receipt_mismatch 2
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
fingerprint(){ { for p in "$CC_CONFIG" "$CC_CERT"; do stat -c '%n|%a|%u|%g|%s' "$p"; hash "$p"; done; for p in "$CC_DATA_DIR/library.json" "$CC_DATA_DIR/tmdb-token"; do if [ -e "$p" ]; then stat -c '%n|%a|%u|%g|%s' "$p"; hash "$p"; else printf '%s=absent\n' "$p"; fi; done; systemctl show "$CC_SERVICE" -p User -p Group -p ExecStart -p FragmentPath; }; }
fingerprint >"$pre/before.fingerprint"
[[ "$mode" != --preflight ]] || { printf 'preflight=PASS
fingerprint=%s
' "$pre/before.fingerprint"; exit 0; }
new_bin="$(mktemp "$CC_ROLLBACK_DIR/.new-server.XXXXXX")"; install -o root -g root -m 0755 "$CC_SERVER_FILE" "$new_bin"
new_web="$(mktemp -d "$CC_ROLLBACK_DIR/.new-web.XXXXXX")"; unzip -qq "$CC_WEB_ZIP" -d "$new_web"; [[ "$(build <"$new_web/build.json")" == "$CC_EXPECTED_WEB_BUILD" ]] || die web_build_mismatch 2
find "$new_web" -type d -exec chmod 0755 {} +; find "$new_web" -type f -exec chmod 0644 {} +; chown -R root:root "$new_web"
old_bin="$CC_ROLLBACK_DIR/chill-server-$(date -u +%Y%m%dT%H%M%SZ)"; old_web="$CC_ROLLBACK_DIR/web-$(date -u +%Y%m%dT%H%M%SZ)"
[[ ! -e "$journal" ]] || die unfinished_transition_requires_recover 1
old_web_build="$(web_build_dir "$CC_WEB_DIR")" || die old_web_build_unreadable 1
python3 -I - "$journal" "$old_bin" "$old_web" "$new_bin" "$new_web" "$CC_EXPECTED_OLD_BINARY_SHA256" "$CC_EXPECTED_NEW_BINARY_SHA256" "$old_web_build" "$CC_EXPECTED_WEB_BUILD" <<'PY'
import json,os,sys
p=sys.argv[1]; f=open(p+'.new','x'); json.dump(dict(state='prepared',old_binary=sys.argv[2],old_web=sys.argv[3],new_binary=sys.argv[4],new_web=sys.argv[5],old_hash=sys.argv[6],new_hash=sys.argv[7],old_web_build=sys.argv[8],new_web_build=sys.argv[9]),f,sort_keys=True); f.flush(); os.fsync(f.fileno()); f.close(); os.replace(p+'.new',p); d=os.open(os.path.dirname(p),os.O_RDONLY); os.fsync(d); os.close(d)
PY
state=running
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
python3 -I - "$journal" <<'PY'
import json,os,sys
p=sys.argv[1]; d=json.load(open(p)); assert d['state']=='prepared'; d['state']='stopping'; f=open(p+'.new','w'); json.dump(d,f,sort_keys=True); f.flush(); os.fsync(f.fileno()); f.close(); os.replace(p+'.new',p); fd=os.open(os.path.dirname(p),os.O_RDONLY); os.fsync(fd); os.close(fd)
PY
recover(){ rc=$?; set +e; trap '' HUP INT TERM; trap - EXIT; if [[ "$state" != complete && -f "$journal" ]]; then recover_transition || { printf 'recovery=FAILED journal=%s\n' "$journal" >&2; exit 1; }; fi; exit "$rc"; }
trap recover EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
systemctl stop "$CC_SERVICE"; state=stopped; ! systemctl is-active --quiet "$CC_SERVICE" || die service_failed_to_stop 1
out="$(timeout --signal=TERM --kill-after=20s 600s /bin/bash -c 'set -e; child=; cleanup(){ [ -z "$child" ] || { kill -TERM -- -"$child" 2>/dev/null || true; sleep 2; kill -KILL -- -"$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; }; }; trap "cleanup; exit 124" TERM INT HUP; cd "$1"; setsid runuser -u "$2" -- "$3" --backup --config "$4" </dev/null & child=$!; wait "$child"' -- /usr/lib/chillcenter "$CC_SERVICE_USER" "$CC_BINARY" "$CC_CONFIG")" || die backup_failed_or_timed_out 1
backup="${out#Sauvegarde terminée : }"; [[ "$out" != "$backup" && "$backup" == "$CC_DATA_DIR/backups/"* ]] || die backup_receipt_refused 1; backup_ok "$backup"
install -o root -g root -m 0755 "$CC_BINARY" "$old_bin"; [[ "$(hash "$old_bin")" == "$CC_EXPECTED_OLD_BINARY_SHA256" ]] || die old_binary_backup_mismatch 1
mv -T "$CC_WEB_DIR" "$old_web"; mv -T "$new_web" "$CC_WEB_DIR"; mv -T "$CC_BINARY" "$CC_ROLLBACK_DIR/server-replaced-$(date -u +%Y%m%dT%H%M%SZ)"; mv -T "$new_bin" "$CC_BINARY"
systemctl start "$CC_SERVICE"; state=started
deadline=$((SECONDS + 60)); while (( SECONDS < deadline )); do tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server" 2>/dev/null|grep -qx 200&&break; sleep 1; done
tls -o /dev/null -w '%{http_code}' "$CC_BASE_URL/api/2026-10-14/server"|grep -qx 200 || die new_server_not_ready 1
[[ "$(tls "$CC_BASE_URL/build.json"|build)" == "$CC_EXPECTED_WEB_BUILD" ]] || die new_web_not_served 1
fingerprint >"$pre/after.fingerprint"; cmp -s "$pre/before.fingerprint" "$pre/after.fingerprint" || die persistent_state_changed 1
state=complete; trap - EXIT HUP INT TERM
rm -f -- "$journal"
printf 'deployment=PASS
backup=%s
old_binary=%s
old_web=%s
' "$backup" "$old_bin" "$old_web"
