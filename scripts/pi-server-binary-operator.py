#!/usr/bin/env python3
"""Manual, root-only, server-binary-only Pi deployment operator.

It intentionally has no downloader, manifest reader, configuration writer, or
network discovery.  An administrator supplies reviewed local bytes and pins.
"""
import argparse, hashlib, json, os, pathlib, re, shutil, signal, socket, sqlite3
import stat, subprocess, sys, tempfile, time, urllib.request

OLD_DEFAULT = "68d526cd3f3af7cee9b07b06c62f46260656dda7cccc4c12994dee85431771e5"

class Refusal(RuntimeError): pass
def fail(s): raise Refusal(s)
def sha(p):
    h=hashlib.sha256()
    with open(p,"rb") as f:
        for b in iter(lambda:f.read(1024*1024),b""): h.update(b)
    return h.hexdigest()
def regular(p):
    s=os.lstat(p)
    if not stat.S_ISREG(s.st_mode) or stat.S_ISLNK(s.st_mode): fail("regular_file_required")
def private_dir(p):
    s=os.lstat(p)
    if not stat.S_ISDIR(s.st_mode) or stat.S_ISLNK(s.st_mode) or stat.S_IMODE(s.st_mode)!=0o700 or s.st_uid!=0: fail("private_root_0700_required")
def atomic_json(path, obj):
    tmp=path.with_name(path.name+".new")
    fd=os.open(tmp, os.O_WRONLY|os.O_CREAT|os.O_EXCL, 0o600)
    with os.fdopen(fd,"w") as f: json.dump(obj,f,sort_keys=True); f.flush(); os.fsync(f.fileno())
    os.replace(tmp,path); d=os.open(path.parent,os.O_RDONLY); os.fsync(d); os.close(d)
def journal_load(p):
    regular(p); d=json.loads(p.read_bytes())
    if set(d)!={"state","binary","old_copy","new_stage","old_sha","new_sha"} or d["state"] not in {"prepared","stopped","swapped"}: fail("invalid_journal")
    for k in ("binary","old_copy","new_stage"):
        if not pathlib.Path(d[k]).is_absolute() or pathlib.Path(d[k]).parent != p.parent: fail("journal_path_refused")
    if not all(re.fullmatch(r"[0-9a-f]{64}",d[k]) for k in ("old_sha","new_sha")): fail("journal_hash_refused")
    return d
def run(*cmd, timeout=60, check=True, **kw):
    x=subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout, **kw)
    if check and x.returncode: fail("command_failed:"+cmd[0])
    return x
def active(service): return run("systemctl","is-active",service,check=False).stdout.strip()=="active"
def service(service, action): run("systemctl",action,service,timeout=60)
def elf_aarch64(p):
    regular(p)
    with open(p,"rb") as f:
        if f.read(4)!=b"\x7fELF": fail("candidate_not_elf")
    out=run("readelf","-h",str(p)).stdout
    if "ELF64" not in out or "AArch64" not in out: fail("candidate_not_reviewed_aarch64_elf")
def strict_json(url, cert, path):
    ctx=__import__("ssl").create_default_context(cafile=str(cert))
    try:
        with urllib.request.urlopen(urllib.request.Request(url+path),context=ctx,timeout=10) as r:
            if r.status!=200: fail("strict_tls_http_failed")
            return json.loads(r.read())
    except Refusal: raise
    except Exception as e: fail("strict_tls_failed")
def v071(url,cert):
    d=strict_json(url,cert,"/api/2026-10-14/server")
    if d.get("version")!="0.7.1": fail("version_not_0_7_1")
    ctx=__import__("ssl").create_default_context(cafile=str(cert))
    try: urllib.request.urlopen(url+"/api/v1/server",context=ctx,timeout=10)
    except urllib.error.HTTPError as e:
        if e.code==404:return
    except Exception: pass
    fail("legacy_v1_not_404")
def snapshot_db(db):
    # SQLite backup yields a coherent snapshot; dump/row metadata detects semantic drift.
    src=sqlite3.connect("file:"+str(db)+"?mode=ro",uri=True); mem=sqlite3.connect(":memory:"); src.backup(mem)
    version=mem.execute("pragma user_version").fetchone()[0]
    if version!=3 or mem.execute("pragma quick_check").fetchone()[0]!="ok": fail("database_schema_or_quickcheck_failed")
    h=hashlib.sha256(); tables=[]
    for name,sql in mem.execute("select name,sql from sqlite_master where type='table' order by name"):
        tables.append((name,mem.execute('select count(*) from "'+name.replace('"','""')+'"').fetchone()[0])); h.update((name+"\0"+(sql or "")+"\n").encode())
        for row in mem.execute("select * from "+'"'+name.replace('"','""')+'"'):
            h.update(repr(row).encode()+b"\n")
    src.close(); mem.close(); return {"schema":version,"tables":tables,"digest":h.hexdigest()}
def invariant(paths, service):
    # Hashes are compared in-process and never printed; systemctl cat excludes runtime PID fields.
    a={str(p):sha(p) for p in paths if p.exists()}
    a["unit"]=hashlib.sha256(run("systemctl","cat",service).stdout.encode()).hexdigest()
    return a
def quiet(args):
    if not active(args.service): fail("service_not_active")
    if not isinstance(strict_json(args.base_url,args.cert,"/api/2026-10-14/server"),dict): fail("dated_api_invalid")
    if strict_json(args.base_url,args.cert,"/api/2026-10-14/admin/scan").get("running") is not False: fail("scan_active_or_unknown")
    jobs=args.data_dir/"tmdb-jobs.json"
    if jobs.exists():
        regular(jobs); d=json.loads(jobs.read_bytes())
        if not isinstance(d,dict) or any(not isinstance(x,dict) or x.get("status") not in {"completed","failed","cancelled","inactive"} for x in d.values()): fail("tmdb_jobs_not_inactive")
    leases=args.data_dir/"leases"
    if leases.exists() and any(leases.iterdir()): fail("active_lease_refusing_stop")
    ss=run("ss","-Htnp",timeout=20).stdout
    if args.binary.name in ss: fail("active_client_socket_refusing_stop")
def backup(args):
    # A dedicated process group lets the timeout path terminate the backup and
    # every ordinary descendant, rather than leaving a runuser child behind.
    cmd=["runuser","-u",args.service_user,"--",str(args.binary),"--backup","--config",str(args.config)]
    child=subprocess.Popen(cmd,cwd="/usr/lib/chillcenter",stdin=subprocess.DEVNULL,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    try: out,err=child.communicate(timeout=600)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid,signal.SIGTERM)
        try: child.communicate(timeout=20)
        except subprocess.TimeoutExpired: os.killpg(child.pid,signal.SIGKILL); child.communicate()
        fail("backup_failed_or_timed_out")
    if child.returncode: fail("backup_failed_or_timed_out")
    m=re.fullmatch(r"Sauvegarde terminée : (.+)",out.strip())
    if not m: fail("backup_receipt_refused")
    root=pathlib.Path(m.group(1)); validate_backup(root,args.data_dir); return root
def validate_backup(root,data):
    s=os.lstat(root)
    if not stat.S_ISDIR(s.st_mode) or stat.S_ISLNK(s.st_mode) or stat.S_IMODE(s.st_mode)&0o077: fail("backup_not_private")
    manifest=root/"manifest.json"; regular(manifest); d=json.loads(manifest.read_bytes())
    if d.get("format_version")!=1 or d.get("schema_version")!=3 or d.get("data_root")!=str(data): fail("backup_manifest_invalid")
    entries=d.get("entries");
    if not isinstance(entries,list) or not entries: fail("backup_entries_invalid")
    wanted={x.get("path"):x for x in entries if isinstance(x,dict)}
    actual=set()
    for p in root.rglob("*"):
        if p.is_dir(): continue
        regular(p); rel=str(p.relative_to(root)); actual.add(rel); x=wanted.get(rel)
        if rel=="manifest.json": continue
        if not x or x.get("size")!=p.stat().st_size or x.get("sha256")!=sha(p): fail("backup_hash_invalid")
    if actual != set(wanted)|{"manifest.json"}: fail("backup_tree_invalid")
def recover(args):
    j=args.rollback_dir/"binary-transition.json"; d=journal_load(j)
    binary=pathlib.Path(d["binary"]); old=pathlib.Path(d["old_copy"]); stage=pathlib.Path(d["new_stage"])
    current=sha(binary) if binary.exists() else None
    if d["state"]=="prepared":
        if current!=d["old_sha"] or not active(args.service): fail("prepared_recovery_unsafe")
    else:
        if current==d["new_sha"]:
            if not old.exists() or sha(old)!=d["old_sha"]: fail("rollback_copy_missing")
            os.replace(binary,args.rollback_dir/("failed-"+str(int(time.time())))); os.replace(old,binary)
        elif current!=d["old_sha"]: fail("unknown_binary_not_overwritten")
        if not active(args.service): service(args.service,"start")
    j.unlink(); print("recovery=PASS")
def main():
    p=argparse.ArgumentParser(); p.add_argument("mode",choices=("preflight","deploy","recover")); p.add_argument("--base-url",required=True); p.add_argument("--service",required=True); p.add_argument("--service-user",default="chillcenter"); p.add_argument("--binary",type=pathlib.Path,required=True); p.add_argument("--candidate",type=pathlib.Path); p.add_argument("--config",type=pathlib.Path,required=True); p.add_argument("--cert",type=pathlib.Path,required=True); p.add_argument("--data-dir",type=pathlib.Path,required=True); p.add_argument("--rollback-dir",type=pathlib.Path,required=True); p.add_argument("--old-sha",default=OLD_DEFAULT); p.add_argument("--new-sha")
    a=p.parse_args()
    if os.geteuid()!=0 or not re.fullmatch(r"https://[^/]+",a.base_url): fail("root_or_https_base_required")
    if a.mode=="recover": recover(a); return
    if not a.candidate or not a.new_sha: fail("candidate_and_new_sha_required")
    if not all(re.fullmatch(r"[0-9a-f]{64}",x or "") for x in (a.old_sha,a.new_sha)): fail("sha_pin_required")
    for x in (a.binary,a.candidate,a.config,a.cert,a.data_dir,a.rollback_dir):
        if not x.is_absolute(): fail("absolute_paths_required")
    private_dir(a.rollback_dir); regular(a.binary); regular(a.candidate); regular(a.config); regular(a.cert); elf_aarch64(a.candidate)
    if sha(a.binary)!=a.old_sha or sha(a.candidate)!=a.new_sha: fail("exact_hash_pin_mismatch")
    quiet(a); db=a.data_dir/"catalog.sqlite"; before=snapshot_db(db); inv=invariant([a.config,a.cert,a.data_dir/"tmdb-token"],a.service)
    if a.mode=="preflight": print("preflight=PASS"); return
    j=a.rollback_dir/"binary-transition.json"
    if j.exists(): fail("unfinished_transition_requires_recover")
    stage=a.rollback_dir/("new-"+a.new_sha[:16]); old=a.rollback_dir/("old-"+a.old_sha[:16])
    if stage.exists() or old.exists(): fail("rollback_name_already_exists")
    shutil.copyfile(a.candidate,stage); os.chown(stage,0,0); os.chmod(stage,0o755)
    if sha(stage)!=a.new_sha: fail("stage_hash_mismatch")
    atomic_json(j,{"state":"prepared","binary":str(a.binary),"old_copy":str(old),"new_stage":str(stage),"old_sha":a.old_sha,"new_sha":a.new_sha})
    # Final gate is deliberately after staging/journalling but before service stop.
    quiet(a); atomic_json(j,{**journal_load(j),"state":"stopped"}); service(a.service,"stop")
    try:
        if active(a.service): fail("service_failed_to_stop")
        backup(a); os.replace(a.binary,old); os.replace(stage,a.binary); atomic_json(j,{**journal_load(j),"state":"swapped"}); service(a.service,"start")
        if not active(a.service): fail("service_failed_to_start")
        v071(a.base_url,a.cert)
        if snapshot_db(db)!=before or invariant([a.config,a.cert,a.data_dir/"tmdb-token"],a.service)!=inv: fail("persistent_invariant_changed")
    except Exception:
        recover(a); raise
    j.unlink(); print("deployment=PASS")
if __name__=="__main__":
    try: main()
    except (Refusal, subprocess.TimeoutExpired, OSError, sqlite3.Error) as e: print("REFUSED:"+str(e),file=sys.stderr); sys.exit(2)
