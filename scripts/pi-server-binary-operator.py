#!/usr/bin/env python3
"""Manual, root-only, server-binary-only Pi deployment operator.

It intentionally has no downloader, manifest reader, configuration writer, or
network discovery.  An administrator supplies reviewed local bytes and pins.
"""
import argparse, hashlib, json, os, pathlib, re, shutil, signal, sqlite3
import stat, subprocess, sys, time, urllib.parse, urllib.request, uuid

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
    os.fchmod(fd,0o600)
    with os.fdopen(fd,"w") as f: json.dump(obj,f,sort_keys=True); f.flush(); os.fsync(f.fileno())
    os.replace(tmp,path); d=os.open(path.parent,os.O_RDONLY); os.fsync(d); os.close(d)
def journal_load(p, require_root=False):
    regular(p); d=json.loads(p.read_bytes())
    s=os.stat(p)
    if stat.S_IMODE(s.st_mode)!=0o600 or (require_root and s.st_uid!=0): fail("journal_not_root_0600")
    if set(d)!={"state","service","binary","old_copy","new_stage","old_sha","new_sha","maintenance_nonce"} or d["state"] not in {"prepared","quiescing","stopping","swapped","complete","rollback_complete"}: fail("invalid_journal")
    if not pathlib.Path(d["binary"]).is_absolute(): fail("journal_binary_path_refused")
    for k in ("old_copy","new_stage"):
        if not pathlib.Path(d[k]).is_absolute() or pathlib.Path(d[k]).parent != p.parent: fail("journal_path_refused")
    if not all(re.fullmatch(r"[0-9a-f]{64}",d[k]) for k in ("old_sha","new_sha")): fail("journal_hash_refused")
    if not re.fullmatch(r"[0-9a-f]{32}",d["maintenance_nonce"]): fail("journal_nonce_refused")
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
        req=urllib.request.Request(url+path,headers={"Connection":"close"})
        with urllib.request.urlopen(req,context=ctx,timeout=10) as r:
            if r.status!=200: fail("strict_tls_http_failed")
            return json.loads(r.read(1024*1024+1).decode()) if r.length is None or r.length<=1024*1024 else fail("response_too_large")
    except Refusal: raise
    except Exception as e: fail("strict_tls_failed")
def v071(url,cert):
    deadline=time.monotonic()+60; d={}
    while time.monotonic()<deadline:
        try:
            d=strict_json(url,cert,"/api/2026-10-14/server")
            if d.get("version")=="0.7.1": break
        except Refusal: pass
        time.sleep(1)
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
        cols=[x[1] for x in mem.execute('pragma table_info("'+name.replace('"','""')+'")')]
        order=",".join('"'+x.replace('"','""')+'"' for x in cols)
        for row in mem.execute("select * from "+'"'+name.replace('"','""')+'"'+(" order by "+order if order else "")):
            h.update(repr(row).encode()+b"\n")
    src.close(); mem.close(); return {"schema":version,"tables":tables,"digest":h.hexdigest()}
def file_invariant(p):
    if not p.exists(): return "absent"
    regular(p); s=os.stat(p); return (sha(p),stat.S_IMODE(s.st_mode),s.st_uid,s.st_gid,s.st_size)
def web_digest(root):
    regular_entries=[]
    for p in sorted(root.rglob("*"),key=lambda x:str(x.relative_to(root))):
        s=os.lstat(p); rel=str(p.relative_to(root))
        if stat.S_ISLNK(s.st_mode) or not (stat.S_ISREG(s.st_mode) or stat.S_ISDIR(s.st_mode)): fail("web_tree_unsafe")
        regular_entries.append((rel,stat.S_IFMT(s.st_mode),stat.S_IMODE(s.st_mode),s.st_uid,s.st_gid,sha(p) if p.is_file() else ""))
    return hashlib.sha256(repr(regular_entries).encode()).hexdigest()
def invariant(paths, service, web):
    # Hashes are compared in-process and never printed; systemctl cat excludes runtime PID fields.
    a={str(p):file_invariant(p) for p in paths}
    a["unit"]=hashlib.sha256(run("systemctl","cat",service).stdout.encode()).hexdigest()
    a["web"]=web_digest(web)
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
def acquire_maintenance(args, nonce):
    flag=pathlib.Path("/run/chillcenter-update/maintenance")
    parent=flag.parent
    if not parent.exists(): parent.mkdir(mode=0o755)
    ps=os.stat(parent)
    if ps.st_uid!=0 or not stat.S_ISDIR(ps.st_mode) or stat.S_ISLNK(os.lstat(parent).st_mode): fail("maintenance_parent_invalid")
    tmp=parent/(".manual-maintenance-"+nonce)
    fd=os.open(tmp,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    try:
        os.write(fd,(nonce+"\n").encode()); os.fsync(fd)
    finally: os.close(fd)
    try: os.link(tmp,flag)
    except FileExistsError: tmp.unlink(); fail("preexisting_maintenance_refused")
    fd=os.open(parent,os.O_RDONLY); os.fsync(fd); os.close(fd); tmp.unlink()
    s=os.stat(flag)
    if s.st_uid!=0 or stat.S_IMODE(s.st_mode)!=0o600: fail("maintenance_ownership_invalid")
    return (s.st_dev,s.st_ino,nonce)
def release_maintenance(token, strict=True):
    flag=pathlib.Path("/run/chillcenter-update/maintenance")
    try: s=os.lstat(flag)
    except FileNotFoundError: return
    if (s.st_dev,s.st_ino)!=token[:2] or not stat.S_ISREG(s.st_mode) or s.st_uid!=0 or stat.S_IMODE(s.st_mode)!=0o600 or flag.read_bytes()!= (token[2]+"\n").encode():
        if strict: fail("maintenance_ownership_unproven")
        return False
    flag.unlink()
    return True
def quiescent(args, token):
    """Admission is closed; require the server's fresh generation-bound pulse."""
    generation=str(os.stat("/run/chillcenter-update/maintenance").st_mtime_ns)
    deadline=time.monotonic()+12
    proof=args.data_dir/"updates/quiescence.json"
    while time.monotonic()<deadline:
        status=strict_json(args.base_url,args.cert,"/api/2026-10-14/admin/updates")
        try:
            regular(proof); q=json.loads(proof.read_bytes())
            fresh=time.time()-int(q["updated_unix"])<=3
            ok=q.get("maintenance_generation_ns")==generation and q.get("quiescing") is True and q.get("busy") is False and q.get("active_mutations")==0
        except Exception: fresh=ok=False
        if status.get("busy") is False and status.get("active_mutations")==0 and status.get("quiescing") is True and fresh and ok: return
        time.sleep(.5)
    fail("generation_bound_quiescence_unproven")
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
    root=pathlib.Path(m.group(1)); return root,validate_backup(root,args.data_dir)
def durable_replace(src,dst):
    """Same-filesystem rename with both file and parent directory durability."""
    os.replace(src,dst)
    for p in {pathlib.Path(src).parent,pathlib.Path(dst).parent}:
        fd=os.open(p,os.O_RDONLY); os.fsync(fd); os.close(fd)
def validate_backup(root,data):
    s=os.lstat(root)
    if not stat.S_ISDIR(s.st_mode) or stat.S_ISLNK(s.st_mode) or stat.S_IMODE(s.st_mode)&0o077: fail("backup_not_private")
    manifest=root/"manifest.json"; regular(manifest); d=json.loads(manifest.read_bytes())
    if d.get("format_version")!=1 or d.get("schema_version")!=3 or d.get("data_root")!=str(data): fail("backup_manifest_invalid")
    entries=d.get("entries");
    if not isinstance(entries,list) or not entries: fail("backup_entries_invalid")
    wanted={x.get("path"):x for x in entries if isinstance(x,dict)}
    required={"catalog.sqlite","server-config.json","secrets/tls/certificate.pem","secrets/tls/service-key.pem"}
    if not required <= set(wanted): fail("backup_required_entries_missing")
    actual=set()
    for p in root.rglob("*"):
        ls=os.lstat(p)
        if stat.S_ISLNK(ls.st_mode): fail("backup_symlink_refused")
        if stat.S_ISDIR(ls.st_mode): continue
        regular(p); rel=str(p.relative_to(root)); actual.add(rel); x=wanted.get(rel)
        if rel=="manifest.json": continue
        if not x or x.get("size")!=p.stat().st_size or x.get("sha256")!=sha(p): fail("backup_hash_invalid")
    if actual != set(wanted)|{"manifest.json"}: fail("backup_tree_invalid")
    return snapshot_db(root/"catalog.sqlite")
def validate_recovery(args):
    for x in (args.binary,args.config,args.cert,args.key,args.data_dir,args.web_dir,args.rollback_dir):
        if not x.is_absolute(): fail("absolute_paths_required")
    private_dir(args.rollback_dir); regular(args.config); regular(args.cert); regular(args.key)
    j=args.rollback_dir/"binary-transition.json"; d=journal_load(j,require_root=True)
    if d["binary"]!=str(args.binary) or d["service"]!=args.service or d["old_sha"]!=args.old_sha or d["new_sha"]!=args.new_sha: fail("journal_cli_binding_refused")
    if args.binary.exists(): regular(args.binary)
    elif d["state"] not in {"stopping","swapped"}: fail("missing_binary_recovery_refused")
    return d
def owned_maintenance(nonce):
    flag=pathlib.Path("/run/chillcenter-update/maintenance")
    try: s=os.lstat(flag)
    except FileNotFoundError: return None
    token=(s.st_dev,s.st_ino,nonce)
    if not stat.S_ISREG(s.st_mode) or s.st_uid!=0 or stat.S_IMODE(s.st_mode)!=0o600 or flag.read_bytes()!=(nonce+"\n").encode(): fail("maintenance_ownership_unproven")
    return token
def old_health(args, expected):
    if sha(args.binary)!=expected or not active(args.service) or not isinstance(strict_json(args.base_url,args.cert,"/api/2026-10-14/server"),dict): fail("rollback_health_failed")
def recover(args):
    j=args.rollback_dir/"binary-transition.json"; d=validate_recovery(args)
    binary=pathlib.Path(d["binary"]); old=pathlib.Path(d["old_copy"]); stage=pathlib.Path(d["new_stage"])
    current=sha(binary) if binary.exists() else None
    token=owned_maintenance(d["maintenance_nonce"])
    if d["state"] in {"complete","rollback_complete"}:
        old_health(args,d["old_sha"] if d["state"]=="rollback_complete" else d["new_sha"])
        if token: release_maintenance(token)
        print("recovery=PASS"); return
    if d["state"] in {"prepared","quiescing"}:
        old_health(args,d["old_sha"])
        if token: release_maintenance(token)
    else:
        # A new process can be serving from a deleted/replaced inode: stop first.
        if active(args.service): service(args.service,"stop")
        if current==d["new_sha"]:
            if not old.exists() or sha(old)!=d["old_sha"]: fail("rollback_copy_missing")
            durable_replace(binary,args.rollback_dir/("failed-"+str(int(time.time())))); durable_replace(old,binary)
        elif current is None:
            if not old.exists() or sha(old)!=d["old_sha"]: fail("rollback_copy_missing")
            durable_replace(old,binary)
        elif current!=d["old_sha"]: fail("unknown_binary_not_overwritten")
        if sha(binary)!=d["old_sha"]: fail("rollback_hash_failed")
        service(args.service,"start")
        old_health(args,d["old_sha"])
        if token: release_maintenance(token)
    atomic_json(j,{**d,"state":"rollback_complete"}); print("recovery=PASS")
def main():
    p=argparse.ArgumentParser(); p.add_argument("mode",choices=("preflight","deploy","recover")); p.add_argument("--base-url",required=True); p.add_argument("--service",required=True); p.add_argument("--service-user",default="chillcenter"); p.add_argument("--binary",type=pathlib.Path,required=True); p.add_argument("--candidate",type=pathlib.Path); p.add_argument("--config",type=pathlib.Path,required=True); p.add_argument("--cert",type=pathlib.Path,required=True); p.add_argument("--key",type=pathlib.Path,required=True); p.add_argument("--data-dir",type=pathlib.Path,required=True); p.add_argument("--web-dir",type=pathlib.Path,required=True); p.add_argument("--rollback-dir",type=pathlib.Path,required=True); p.add_argument("--old-sha",required=True); p.add_argument("--new-sha")
    a=p.parse_args()
    u=urllib.parse.urlsplit(a.base_url)
    if os.geteuid()!=0 or u.scheme!="https" or not u.hostname or u.username or u.password or u.path or u.query or u.fragment: fail("root_or_https_base_required")
    urllib.request.install_opener(urllib.request.build_opener(urllib.request.ProxyHandler({})))
    if not a.new_sha: fail("new_sha_required")
    if not all(re.fullmatch(r"[0-9a-f]{64}",x or "") for x in (a.old_sha,a.new_sha)): fail("sha_pin_required")
    if a.mode=="recover": recover(a); return
    if not a.candidate: fail("candidate_required")
    for x in (a.binary,a.candidate,a.config,a.cert,a.key,a.data_dir,a.web_dir,a.rollback_dir):
        if not x.is_absolute(): fail("absolute_paths_required")
    private_dir(a.rollback_dir); regular(a.binary); regular(a.candidate); regular(a.config); regular(a.cert); regular(a.key); elf_aarch64(a.candidate)
    if sha(a.binary)!=a.old_sha or sha(a.candidate)!=a.new_sha: fail("exact_hash_pin_mismatch")
    quiet(a); db=a.data_dir/"catalog.sqlite"; before=snapshot_db(db); inv=invariant([a.config,a.cert,a.key,a.data_dir/"tmdb-token",a.data_dir/"library.json"],a.service,a.web_dir)
    if a.mode=="preflight": print("preflight=PASS"); return
    j=a.rollback_dir/"binary-transition.json"
    if j.exists(): fail("unfinished_transition_requires_recover")
    stage=a.rollback_dir/("new-"+a.new_sha[:16]); old=a.rollback_dir/("old-"+a.old_sha[:16])
    if stage.exists() or old.exists(): fail("rollback_name_already_exists")
    nonce=uuid.uuid4().hex
    atomic_json(j,{"state":"prepared","service":a.service,"binary":str(a.binary),"old_copy":str(old),"new_stage":str(stage),"old_sha":a.old_sha,"new_sha":a.new_sha,"maintenance_nonce":nonce})
    shutil.copyfile(a.candidate,stage); fd=os.open(stage,os.O_RDONLY); os.fsync(fd); os.close(fd); os.chown(stage,0,0); os.chmod(stage,0o755)
    if sha(stage)!=a.new_sha: fail("stage_hash_mismatch")
    # Final gate is deliberately after staging/journalling but before service stop.
    token=None
    try:
        quiet(a)
        if sha(a.binary)!=a.old_sha or sha(stage)!=a.new_sha or invariant([a.config,a.cert,a.key,a.data_dir/"tmdb-token",a.data_dir/"library.json"],a.service,a.web_dir)!=inv: fail("final_pre_stop_invariant_changed")
        token=acquire_maintenance(a,nonce); atomic_json(j,{**journal_load(j),"state":"quiescing"}); quiescent(a,token)
        atomic_json(j,{**journal_load(j),"state":"stopping"}); service(a.service,"stop")
        if active(a.service): fail("service_failed_to_stop")
        backup_path,saved=backup(a)
        if saved!=before: fail("backup_not_pre_swap_baseline")
        durable_replace(a.binary,old); durable_replace(stage,a.binary); atomic_json(j,{**journal_load(j),"state":"swapped"}); service(a.service,"start")
        if not active(a.service): fail("service_failed_to_start")
        v071(a.base_url,a.cert)
        if snapshot_db(db)!=before or invariant([a.config,a.cert,a.key,a.data_dir/"tmdb-token",a.data_dir/"library.json"],a.service,a.web_dir)!=inv: fail("persistent_invariant_changed")
    except Exception:
        recover(a); raise
    finally:
        if token is not None: release_maintenance(token)
    atomic_json(j,{**journal_load(j),"state":"complete"}); print("deployment=PASS")
if __name__=="__main__":
    try: main()
    except (Refusal, subprocess.TimeoutExpired, OSError, sqlite3.Error) as e: print("REFUSED:"+str(e),file=sys.stderr); sys.exit(2)
