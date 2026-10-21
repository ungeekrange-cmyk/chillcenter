#!/usr/bin/env python3
"""Disposable-Colima root/systemd boundary test entrypoint for updater fixes.

Never run on a Pi, NAS, developer workstation, or an already-installed host.
It makes only the documented Chill Center install roots writable and refuses
anything other than the dedicated v07 Colima VM.
"""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time
import uuid
import hashlib
import datetime
import threading
import zipfile
import io
import socket
import ssl
import urllib.request

ROOTS = [pathlib.Path(p) for p in (
    "/usr/lib/chillcenter", "/etc/chillcenter", "/var/lib/chillcenter",
)]
SYSTEMD = pathlib.Path("/usr/lib/systemd/system")

def die(message):
    raise SystemExit("FAIL: " + message)

def run(*args, check=True, **kwargs):
    print("+", " ".join(map(str, args)))
    return subprocess.run(args, check=check, text=True, **kwargs)

def require_guard(resume=False):
    if os.environ.get("CHILL_UPDATE_DISPOSABLE_VM") != "1":
        die("set CHILL_UPDATE_DISPOSABLE_VM=1 explicitly")
    if os.geteuid() != 0:
        die("must run as root in the disposable VM")
    hostname = run("/usr/bin/hostname", capture_output=True).stdout.strip()
    if hostname != "colima-v07-security":
        die("hostname guard refused %r" % hostname)
    if pathlib.Path("/run/systemd/system").exists() is False:
        die("PID 1 is not systemd")
    if resume:
        config = json.loads(pathlib.Path('/etc/chillcenter/server.json').read_text())
        require(config['data_dir'] == '/var/lib/chillcenter/data', 'not our isolated fixture')
        return
    for path in ROOTS:
        if path.exists() or path.is_symlink():
            die("fixture host is not pristine: %s" % path)
    for unit in ("chillcenter.service", "chillcenter-update.service"):
        if (SYSTEMD / unit).exists() or (pathlib.Path("/etc/systemd/system") / unit).exists():
            die("fixture host has existing unit: " + unit)

def mode(path):
    return oct(path.stat().st_mode & 0o777)

def require(condition, message):
    if not condition:
        die(message)

def install_boundary_fixture(updater, server, repo):
    """Install real binaries and production units; no network and no private key."""
    user = "chillcenter"
    run("/usr/sbin/useradd", "--system", "--user-group", "--no-create-home", "--shell", "/usr/sbin/nologin", user)
    uid = int(run("/usr/bin/id", "-u", user, capture_output=True).stdout)
    gid = int(run("/usr/bin/id", "-g", user, capture_output=True).stdout)
    for root in ROOTS:
        root.mkdir(parents=True, mode=0o755)
    for p in ("server/releases/0.7.0-1", "web/releases", "themes/releases"):
        (ROOTS[0] / p).mkdir(parents=True, mode=0o755)
    for p in ("data", "data/updates", "data/backups"):
        q = ROOTS[2] / p; q.mkdir(parents=True, mode=0o700); os.chown(q, uid, gid)
    for p in ("trust", "cache"):
        pathlib.Path('/var/lib/chillcenter-update', p).mkdir(parents=True, mode=0o700)
    shutil.copy2(updater, ROOTS[0] / "chill-update")
    shutil.copy2(server, ROOTS[0] / "server/releases/0.7.0-1/payload")
    os.chmod(ROOTS[0] / "chill-update", 0o755); os.chmod(ROOTS[0] / "server/releases/0.7.0-1/payload", 0o755)
    (ROOTS[0] / "server/current").symlink_to("releases/0.7.0-1")
    # An intentionally synthetic public key is generated only in this VM. It
    # is never a production signing key and no manifest is fetched from a network.
    run("/usr/bin/minisign", "-G", "-s", "/etc/chillcenter/release.key", "-p", "/etc/chillcenter/release.pub", "-W")
    os.chmod("/etc/chillcenter/release.key", 0o600)
    config = {"listen":"127.0.0.1:18443", "data_dir":"/var/lib/chillcenter/data", "tls_names":["localhost","127.0.0.1"]}
    pathlib.Path("/etc/chillcenter/server.json").write_text(json.dumps(config))
    os.chown("/etc/chillcenter/server.json", 0, gid); os.chmod("/etc/chillcenter/server.json", 0o640)
    # Recovery commands only need trusted config paths; TLS is created by the real server.
    updates = {"schema_version":1,"key_id":"synthetic-v07","public_key":"/etc/chillcenter/release.pub","state_root":"/var/lib/chillcenter/data/updates","trust_root":"/var/lib/chillcenter/trust","cache_root":"/var/lib/chillcenter/cache","backup_root":"/var/lib/chillcenter/data/backups","maintenance_flag":"/run/chillcenter-update/maintenance","quiescence_status":"/var/lib/chillcenter/data/updates/quiescence.json","server_config":"/etc/chillcenter/server.json","service_name":"chillcenter.service","service_uid":uid,"service_gid":gid,"health_url":"https://localhost:18443/api/2026-10-14/server","status_url":"https://localhost:18443/api/2026-10-14/admin/updates","health_resolve":"127.0.0.1:18443","health_ca_certificate":"/var/lib/chillcenter/data/tls/certificate.pem","component_roots":{"server":"/usr/lib/chillcenter/server","web":"/usr/lib/chillcenter/web","themes":"/usr/lib/chillcenter/themes"},"targets":{"server":"linux-aarch64","web":"web","themes":"universal"},"bootstrap_versions":{"server":"0.7.0","web":"0.0.0","themes":"0.0.0"},"api_version":"2026.10.14","database_schema":3,"renderer_version":1,"theme_format":1}
    pathlib.Path("/etc/chillcenter/updates.json").write_text(json.dumps(updates))
    os.chmod("/etc/chillcenter/updates.json", 0o640)
    for name in ("chillcenter.service", "chillcenter-update.service"):
        shutil.copy2(repo / "deploy/updates" / name, SYSTEMD / name)
    run("/usr/bin/systemctl", "daemon-reload")
    return uid

def refresh_fixture(repo, updater, server):
    # May resume only this guarded disposable VM, never an arbitrary host.
    run('/usr/bin/systemctl', 'stop', 'chillcenter.service')
    baseline = ROOTS[0] / 'server/releases/0.7.0-1/payload'
    if baseline.with_name('payload.real').exists():
        baseline.unlink(); baseline.with_name('payload.real').rename(baseline)
    shutil.copy2(server, baseline); baseline.chmod(0o755)
    for name in ('chillcenter.service', 'chillcenter-update.service'):
        shutil.copy2(repo / 'deploy/updates' / name, SYSTEMD / name)
    shutil.copy2(updater, ROOTS[0] / 'chill-update')
    config_path = pathlib.Path('/etc/chillcenter/updates.json')
    config = json.loads(config_path.read_text())
    for item in ('trust', 'cache'):
        directory = pathlib.Path('/var/lib/chillcenter-update', item)
        directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        config[item + '_root'] = str(directory)
    config['health_ca_certificate'] = '/etc/chillcenter/health-ca.pem'
    config_path.write_text(json.dumps(config))
    # Match bootstrap ownership; no service-user-writable root configuration.
    gid = int(run('/usr/bin/id', '-g', 'chillcenter', capture_output=True).stdout)
    os.chown('/etc/chillcenter', 0, gid); os.chmod('/etc/chillcenter', 0o750)
    os.chown('/etc/chillcenter/server.json', 0, gid)
    run('/usr/bin/systemctl', 'daemon-reload')

def activation_and_failure_boundaries():
    root = ROOTS[0] / 'server'
    old, new = 'releases/0.7.0-1', 'releases/0.7.0-2'
    marker = root / 'transaction.json'
    auth = pathlib.Path('/run/chillcenter-update/activate-server')
    pathlib.Path('/run/chillcenter-update').mkdir(exist_ok=True, mode=0o755)
    def candidate(unresponsive=False):
        run('/usr/bin/systemctl', 'stop', 'chillcenter.service')
        slot = root / new
        slot.mkdir(exist_ok=True, mode=0o755)
        shutil.copy2(root / old / 'payload', slot / 'payload')
        if unresponsive:
            (slot / 'payload').write_text('#!/bin/sh\ntrap "" TERM\nwhile :; do /bin/sleep 1; done\n')
        os.chmod(slot / 'payload', 0o755)
        operation = str(uuid.uuid4())
        marker.write_text(json.dumps({'schema_version':1, 'operation_id':operation, 'previous_target':old, 'candidate_target':new}))
        os.chmod(marker, 0o600)
        (root / 'current').unlink(); (root / 'current').symlink_to(new)
        # Exactly the production helper's process identity, held live while
        # systemd runs its real +ExecStartPre command in its real sandbox.
        stat = pathlib.Path('/proc/self/stat').read_text().rsplit(') ',1)[1].split()
        identity = pathlib.Path('/proc/sys/kernel/random/boot_id').read_text().strip()+':'+stat[19]
        auth.write_text(json.dumps({'schema_version':1, 'operation_id':operation, 'candidate_target':new,
            'pid':os.getpid(), 'process_identity':identity, 'created_unix':int(time.time())}))
        os.chmod(auth, 0o600)
        run('/usr/bin/systemctl', 'start', 'chillcenter.service')
        require(os.readlink(root / 'current') == new, 'valid live authorization was rejected under actual systemd sandbox')
        require(not auth.exists(), 'authorization was not consumed exactly once')
    candidate()
    # Candidate is already active; an uncommitted transaction MUST stop it,
    # restore the old slot and restart, not merely call start on an active PID.
    before = run('/usr/bin/systemctl', 'show', 'chillcenter.service', '-p', 'MainPID', '--value', capture_output=True).stdout.strip()
    pathlib.Path('/run/chillcenter-update/maintenance').write_text('synthetic interrupted operation')
    run('/usr/lib/chillcenter/chill-update', 'rescue')
    after = run('/usr/bin/systemctl', 'show', 'chillcenter.service', '-p', 'MainPID', '--value', capture_output=True).stdout.strip()
    require(before != after and after != '0', 'rescue retained the active uncommitted process')
    require(os.readlink(root / 'current') == old, 'rescue failed to restore old slot')
    require(not pathlib.Path('/run/chillcenter-update/maintenance').exists(), 'rescue retained maintenance flag')
    require(not marker.exists(), 'rescue retained transaction')
    # Replay after a crash between candidate deletion and marker cleanup.
    marker.write_text(json.dumps({'schema_version':1, 'operation_id':str(uuid.uuid4()), 'previous_target':old, 'candidate_target':new}))
    run('/usr/bin/systemctl', 'restart', 'chillcenter.service')
    require(not marker.exists(), 'idempotent recovery failed after candidate was already deleted')
    # A second prestart cannot reuse the consumed permit.
    candidate()
    run('/usr/bin/systemctl', 'restart', 'chillcenter.service')
    require(os.readlink(root / 'current') == old, 'one-shot authorization was reusable')
    print('PASS actual sandbox: live one-shot token, active-candidate rescue, maintenance cleanup, idempotent rollback')
    return candidate

def wait_for(predicate, message, seconds=25):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate(): return
        time.sleep(.1)
    die(message)

def offline_cache_and_kill_boundaries(server, candidate):
    """Seed only VM-owned synthetic signed bytes; exercise the real fixed CLI."""
    helper = '/usr/lib/chillcenter/chill-update'
    state = json.loads(run(helper, 'inspect', capture_output=True).stdout)['state']
    for component in ('server', 'web', 'themes'):
        revision = str(state['policies'][component]['revision'])
        run(helper, 'policy', component, 'manual', '2', '0', revision)
    data = pathlib.Path('/var/lib/chillcenter/data')
    root = pathlib.Path('/usr/lib/chillcenter/server')
    payload = server.read_bytes()
    now = datetime.datetime.now(datetime.timezone.utc)
    manifest = {'schema_version':1, 'component':'server', 'version':'0.7.1', 'release_sequence':2,
        'release_tag':'server-v0.7.1', 'source_commit':'a'*40, 'key_id':'synthetic-v07',
        'issued_at':(now-datetime.timedelta(minutes=1)).isoformat(), 'expires_at':(now+datetime.timedelta(days=1)).isoformat(),
        'compatibility':{'os_arch':['linux-aarch64'], 'api':{'min':'2026.10.14','max':'2026.10.14'},
            'database':{'current_schema':3,'target_schema':3,'read_min':3,'read_max':3,'write_min':3,'write_max':3,'rollback_min':3,'rollback_max':3}},
        'assets':[{'target':'linux-aarch64','url':'https://github.com/ungeekrange-cmyk/chillcenter/releases/download/server-v0.7.1/chill-server',
            'byte_length':len(payload),'sha256':hashlib.sha256(payload).hexdigest(),'format':'raw'}]}
    encoded = json.dumps(manifest, indent=2).encode()
    digest = hashlib.sha256(encoded).hexdigest()
    cache = pathlib.Path('/var/lib/chillcenter-update/cache/server')
    slot = cache / digest; slot.mkdir(parents=True, exist_ok=True, mode=0o700)
    (slot/'manifest.json').write_bytes(encoded); (slot/'payload').write_bytes(payload)
    run('/usr/bin/minisign','-S','-s','/etc/chillcenter/release.key','-m',str(slot/'manifest.json'),'-x',str(slot/'manifest.json.minisig'))
    (cache/'available.json').write_text(json.dumps({'schema_version':1,'operation_id':str(uuid.uuid4()),'component':'server',
        'target':'linux-aarch64','release_tag':'server-v0.7.1','cache_key':digest}))
    # The old executable remains trusted/root-owned. Its offline backup branch
    # deliberately blocks so TERM/KILL hits the actual post-stop backup window.
    old = root/'releases/0.7.0-1/payload'
    real = old.with_name('payload.real')
    if not real.exists(): old.rename(real)
    old.write_text('#!/bin/sh\nif [ "${1:-}" = --backup ]; then\n echo $$ > /var/lib/chillcenter/data/backups/backup-child-pid\n touch /var/lib/chillcenter/data/backups/backup-entered\n exec /bin/sleep 120\nfi\nexec /usr/lib/chillcenter/server/releases/0.7.0-1/payload.real "$@"\n')
    old.chmod(0o755)
    entered = data/'backups/backup-entered'
    def active():
        return run('/usr/bin/systemctl','is-active','--quiet','chillcenter.service',check=False,capture_output=True).returncode == 0
    for signal in ('KILL','TERM'):
        entered.unlink(missing_ok=True)
        run(helper,'intent','server','install')
        run('/usr/bin/systemctl','reset-failed','chillcenter-update.service',check=False)
        run('/usr/bin/systemctl','start','--no-block','chillcenter-update.service')
        wait_for(entered.exists, 'real offline backup window was not reached')
        pid = int((data/'backups/backup-child-pid').read_text())
        credentials = dict(line.split(':',1) for line in pathlib.Path('/proc',str(pid),'status').read_text().splitlines() if ':' in line)
        service_uid = int(run('/usr/bin/id','-u','chillcenter',capture_output=True).stdout)
        service_gid = int(run('/usr/bin/id','-g','chillcenter',capture_output=True).stdout)
        require(all(int(value)==service_uid for value in credentials['Uid'].split()), 'backup retained a privileged UID')
        require(all(int(value)==service_gid for value in credentials['Gid'].split()), 'backup retained a privileged GID')
        require(not credentials['Groups'].strip(), 'backup retained supplementary groups')
        require(all(int(credentials[key].strip(),16)==0 for key in ('CapPrm','CapEff','CapAmb')), 'backup retained capabilities after dropping UID')
        run('/usr/bin/systemctl','kill','--kill-whom=main','--signal='+signal,'chillcenter-update.service')
        wait_for(lambda: active() and not pathlib.Path('/run/chillcenter-update/maintenance').exists(), 'service/maintenance not restored after '+signal)
        wait_for(lambda: not pathlib.Path('/proc',str(pid)).exists(), 'backup child survived unit failure')
        require(os.readlink(root/'current') == 'releases/0.7.0-1', 'old slot changed during failed backup')
    # The actual ExecStopPost must also replace an already-running candidate.
    dropin = pathlib.Path('/etc/systemd/system/chillcenter-update.service.d')
    dropin.mkdir(parents=True, exist_ok=True)
    override = dropin/'90-v07-fault-test.conf'
    override.write_text('[Service]\nExecStart=\nExecStart=/usr/bin/sleep 120\n')
    try:
        run('/usr/bin/systemctl','daemon-reload')
        run('/usr/bin/systemctl','reset-failed','chillcenter-update.service',check=False)
        run('/usr/bin/systemctl','start','--no-block','chillcenter-update.service')
        wait_for(lambda: int(run('/usr/bin/systemctl','show','chillcenter-update.service','-p','MainPID','--value',capture_output=True).stdout) > 0, 'fault window main PID missing')
        for unresponsive in (False, True):
            if unresponsive:
                run('/usr/bin/systemctl','reset-failed','chillcenter-update.service',check=False)
                run('/usr/bin/systemctl','start','--no-block','chillcenter-update.service')
                wait_for(lambda: int(run('/usr/bin/systemctl','show','chillcenter-update.service','-p','MainPID','--value',capture_output=True).stdout)>0, 'second fault window missing')
            candidate(unresponsive)
            pathlib.Path('/run/chillcenter-update/maintenance').write_text('post-activation crash')
            run('/usr/bin/systemctl','kill','--kill-whom=main','--signal=KILL','chillcenter-update.service')
            wait_for(lambda: active() and os.readlink(root/'current') == 'releases/0.7.0-1' and not pathlib.Path('/run/chillcenter-update/maintenance').exists(), 'post-activation ExecStopPost failed', seconds=55)
            wait_for(lambda: run('/usr/bin/systemctl','show','chillcenter-update.service','-p','ActiveState','--value',capture_output=True).stdout.strip() in ('failed','inactive'), 'updater rescue has not exited')
            print('PASS active-candidate ExecStopPost; ignores TERM:', unresponsive)
    finally:
        override.unlink(missing_ok=True)
        run('/usr/bin/systemctl','daemon-reload')
    old.unlink(); real.rename(old)
    # End only test-owned interrupted intents; no real user state exists here.
    state_path = data/'updates/state.json'
    state = json.loads(state_path.read_text())
    for operation in state['operations']:
        if operation['status'] in ('running','pending','deferred_busy'):
            operation['status']='failed'; operation['result_code']='fixture_completed'
    state_path.write_text(json.dumps(state))
    print('PASS actual root/systemd: signed offline cache, TERM/KILL during backup, descendant cleanup, post-activation ExecStopPost rollback')
    return manifest, state_path

def seed_signed_cache(manifest, payload):
    encoded = json.dumps(manifest, indent=2).encode()
    digest = hashlib.sha256(encoded).hexdigest()
    component = manifest['component']
    cache = pathlib.Path('/var/lib/chillcenter-update/cache',component)
    slot = cache/digest; slot.mkdir(parents=True,exist_ok=True,mode=0o700)
    (slot/'manifest.json').write_bytes(encoded); (slot/'payload').write_bytes(payload)
    run('/usr/bin/minisign','-S','-s','/etc/chillcenter/release.key','-m',str(slot/'manifest.json'),'-x',str(slot/'manifest.json.minisig'))
    (cache/'available.json').write_text(json.dumps({'schema_version':1,'operation_id':str(uuid.uuid4()),'component':component,
        'target':manifest['assets'][0]['target'],'release_tag':manifest['release_tag'],'cache_key':digest}))
    return digest, slot

def state_machine_and_expiry_boundaries(server, manifest, state_path):
    helper='/usr/lib/chillcenter/chill-update'
    state=json.loads(state_path.read_text())
    def save_operation(automatic, attempts, status='running'):
        current=json.loads(state_path.read_text())
        operation={'id':str(uuid.uuid4()),'component':'server','kind':'install','status':status,
            'created_unix':int(time.time())-60,'updated_unix':int(time.time()),'attempts':attempts,
            'result_code':None,'manifest_sha256':hashlib.sha256(json.dumps(manifest,indent=2).encode()).hexdigest(),'automatic':automatic}
        current['operations']=[operation]; state_path.write_text(json.dumps(current))
        return operation['id']
    # Current server policy is Manual. An interrupted automatic claim must not
    # become a manual install merely because the process restarted.
    for automatic, attempts, expected in [(True,1,'automatic_policy_changed'),(False,3,'retry_exhausted')]:
        operation_id=save_operation(automatic,attempts)
        run(helper,'tick')
        operation=next(op for op in json.loads(state_path.read_text())['operations'] if op['id']==operation_id)
        require(operation['status']=='failed' and operation['result_code']==expected,'crash/policy/attempt reconciliation failed')
    # A valid queued intent with a corrupt payload gets a terminal outcome,
    # rather than surviving Pending with attempts=0 on every tick.
    digest, slot=seed_signed_cache(manifest,server.read_bytes())
    (slot/'payload').write_bytes(b'corrupt-cache')
    save_operation(False,0,'pending')
    result=run(helper,'tick',capture_output=True)
    require(all(op['status']=='failed' for op in json.loads(state_path.read_text())['operations']),'corrupt preflight retained a nonterminal intent')
    (slot/'payload').write_bytes(server.read_bytes())
    busy_idle_web_boundary(state_path)
    # Root-only replay fixture: exact bytes were accepted while valid yesterday.
    expired=json.loads(json.dumps(manifest))
    now=datetime.datetime.now(datetime.timezone.utc)
    issued=now-datetime.timedelta(days=2); expires=now-datetime.timedelta(days=1)
    accepted=int((issued+datetime.timedelta(hours=1)).timestamp())
    expired.update(release_sequence=3,issued_at=issued.isoformat(),expires_at=expires.isoformat())
    digest,slot=seed_signed_cache(expired,server.read_bytes())
    trust=pathlib.Path('/var/lib/chillcenter-update/trust/sequences.json')
    pending={'sequence':3,'version':'0.7.1','manifest_sha256':digest,'accepted_unix':accepted}
    trust.write_text(json.dumps({'schema_version':2,'components':{'server':{'highest_sequence':3,'pending':pending}}}));trust.chmod(0o600)
    run(helper,'manifest','server')
    no_consent=run(helper,'apply','server',check=False,capture_output=True)
    require(no_consent.returncode!=0 and '--retry-accepted' in no_consent.stderr,'accepted expired release bypassed local root recovery')
    # Discovery is blocked before any cache eviction, preserving exact receipt.
    run(helper,'check','server',check=False,capture_output=True)
    require((slot/'payload').exists() and json.loads((slot.parent/'available.json').read_text())['cache_key']==digest,'check destroyed pending recovery bytes')
    bad=json.loads(trust.read_text());bad['components']['server']['pending']['accepted_unix']=int(expires.timestamp())+1
    trust.write_text(json.dumps(bad))
    require(run(helper,'manifest','server',check=False,capture_output=True).returncode!=0,'acceptance outside original validity window was admitted')
    bad['components']['server']['pending']=pending;trust.write_text(json.dumps(bad))
    # The local recovery proceeds beyond signature/expiry validation, performs
    # real backup/activation, and deliberately fails health: test executable is
    # v0.7.0 but signed test version is v0.7.1. It must restore the old service.
    retry=run(helper,'apply','server','--retry-accepted',check=False,capture_output=True)
    require(retry.returncode!=0 and 'health' in retry.stderr,'explicit accepted-time retry did not reach health/rollback: '+retry.stderr)
    require(os.readlink('/usr/lib/chillcenter/server/current')=='releases/0.7.0-1','failed recovered candidate was not rolled back')
    run('/usr/bin/systemctl','is-active','--quiet','chillcenter.service')
    require(json.loads(trust.read_text())['components']['server']['highest_sequence']==3,'recovery lowered monotonic floor')
    print('PASS production state machine: persisted automatic policy cancellation, retry cap, corrupt preflight terminalization, exact accepted-time expiry replay, local consent, real backup/health rollback')

def busy_idle_web_boundary(state_path):
    """Hold a real HTTP mutation open, then retry the exact manual web intent."""
    helper='/usr/lib/chillcenter/chill-update'
    root=pathlib.Path('/usr/lib/chillcenter/web')
    previous=root/'releases/0.6.0-1'
    previous.mkdir(parents=True,exist_ok=True)
    (previous/'index.html').write_text('synthetic prior web')
    (previous/'build.json').write_text(json.dumps({'version':'0.6.0','build':'0'*12}))
    if not (root/'current').exists(): (root/'current').symlink_to('releases/0.6.0-1')
    now=datetime.datetime.now(datetime.timezone.utc)
    context=ssl.create_default_context(cafile='/etc/chillcenter/health-ca.pem')
    context.minimum_version=ssl.TLSVersion.TLSv1_2
    def status():
        with urllib.request.urlopen('https://localhost:18443/api/2026-10-14/admin/updates',context=context,timeout=3) as response:
            return json.load(response)
    def open_mutation():
        connection=context.wrap_socket(socket.create_connection(('127.0.0.1',18443)),server_hostname='localhost')
        connection.sendall(b'POST /api/2026-10-14/profiles HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 10000\r\n\r\n{')
        wait_for(lambda: status()['busy'], 'partial HTTP mutation did not reserve actual server admission')
        return connection
    def manifest_for(version,sequence):
        archive=io.BytesIO()
        with zipfile.ZipFile(archive,'w') as package:
            package.writestr('index.html','synthetic signed web '+version)
            package.writestr('build.json',json.dumps({'version':version,'build':str(sequence)*12}))
        payload=archive.getvalue()
        manifest={'schema_version':1,'component':'web','version':version,'release_sequence':sequence,'release_tag':'web-v'+version,
            'source_commit':'b'*40,'key_id':'synthetic-v07','issued_at':(now-datetime.timedelta(minutes=1)).isoformat(),'expires_at':(now+datetime.timedelta(days=1)).isoformat(),
            'compatibility':{'os_arch':['web'],'api':{'min':'2026.10.14','max':'2026.10.14'},'required_server':{'min':'0.7.0','max':'0.7.99'}},
            'assets':[{'target':'web','url':'https://github.com/ungeekrange-cmyk/chillcenter/releases/download/web-v'+version+'/web.zip',
                'byte_length':len(payload),'sha256':hashlib.sha256(payload).hexdigest(),'format':'zip'}]}
        digest,_=seed_signed_cache(manifest,payload)
        return digest
    digest=manifest_for('0.7.0',1)
    held=open_mutation()
    try:
        operation_id=run(helper,'intent','web','install',capture_output=True).stdout.strip()
        result=run(helper,'apply','web',capture_output=True,check=False)
        require(result.returncode!=0, 'busy HTTP mutation did not defer update')
        operation=next(op for op in json.loads(state_path.read_text())['operations'] if op['id']==operation_id)
        require(operation['status']=='deferred_busy' and not operation['automatic'] and operation['manifest_sha256']==digest and operation['attempts']==1,'manual Busy lost identity/origin/attempt count')
        require(os.readlink(root/'current')=='releases/0.6.0-1','busy attempt changed web slot')
        trust=json.loads(pathlib.Path('/var/lib/chillcenter-update/trust/sequences.json').read_text()) if pathlib.Path('/var/lib/chillcenter-update/trust/sequences.json').exists() else {'components':{}}
        require('web' not in trust['components'], 'busy attempt consumed a root sequence')
    finally:
        held.close()
    wait_for(lambda: not status()['busy'], 'closed mutation left server busy')
    state=json.loads(state_path.read_text())
    for op in state['operations']:
        if op['id']==operation_id: op['updated_unix']-=901
    state_path.write_text(json.dumps(state))  # Test clock advance, not a production override.
    run(helper,'tick')
    operation=next(op for op in json.loads(state_path.read_text())['operations'] if op['id']==operation_id)
    require(operation['status']=='succeeded' and not operation['automatic'] and operation['attempts']==2,'idle tick failed to finish the SAME manual intent')
    require(os.readlink(root/'current')=='releases/0.7.0-1' and status()['installed']['web']['version']=='0.7.0','successful web activation not reported by real server')
    # Automatic origin is retained even if the explicit policy changes while
    # a Busy retry is pending; it must never silently turn into manual consent.
    digest=manifest_for('0.7.1',2)
    state=json.loads(state_path.read_text())
    state['policies']['web']['mode']='automatic'
    state['components']['web']['last_scheduled_local_date']=time.strftime('%Y-%m-%d')
    automatic_id=str(uuid.uuid4())
    state['operations'].append({'id':automatic_id,'component':'web','kind':'install','status':'pending','created_unix':int(time.time()),'updated_unix':int(time.time()),'attempts':0,'result_code':None,'automatic':True})
    state_path.write_text(json.dumps(state))
    held=open_mutation()
    try: run(helper,'tick')
    finally: held.close()
    state=json.loads(state_path.read_text())
    operation=next(op for op in state['operations'] if op['id']==automatic_id)
    require(operation['status']=='deferred_busy' and operation['automatic'] and operation['manifest_sha256']==digest,'automatic Busy lost bound origin')
    run(helper,'policy','web','manual','2','0',str(state['policies']['web']['revision']))
    run(helper,'tick')
    operation=next(op for op in json.loads(state_path.read_text())['operations'] if op['id']==automatic_id)
    require(operation['status']=='failed' and operation['result_code']=='automatic_policy_changed','policy change revived automatic install')
    print('PASS real HTTP Busy -> durable manual Deferred -> same-intent idle success; automatic origin/policy cancellation; signed web install/health')

def service_boundary(uid):
    run("/usr/bin/systemctl", "start", "chillcenter.service")
    time.sleep(1)
    run("/usr/bin/systemctl", "is-active", "--quiet", "chillcenter.service")
    # Actual service UID can traverse/read slots, but cannot write their root-owned parents.
    read = run("/usr/sbin/runuser", "-u", "chillcenter", "--", "/usr/bin/test", "-r", "/usr/lib/chillcenter/server/current/payload", check=False)
    require(read.returncode == 0, "service UID cannot read server slot")
    write = run("/usr/sbin/runuser", "-u", "chillcenter", "--", "/usr/bin/touch", "/usr/lib/chillcenter/server/forbidden", check=False)
    require(write.returncode != 0, "service UID wrote root-owned slot")
    require(mode(pathlib.Path("/usr/lib/chillcenter/server/releases/0.7.0-1")) in ("0o750", "0o755"), "slot directory mode is not service-traversable")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--updater", required=True, type=pathlib.Path)
    parser.add_argument("--server", required=True, type=pathlib.Path)
    parser.add_argument("--repo", default=pathlib.Path(__file__).resolve().parents[1], type=pathlib.Path)
    parser.add_argument('--resume-fixture', action='store_true')
    parser.add_argument('--reset-fixture-ledger', action='store_true', help='Archive only this guarded disposable VM test ledger before replay')
    args = parser.parse_args()
    require_guard(args.resume_fixture)
    for binary in (args.updater, args.server):
        require(binary.is_file() and os.access(binary, os.X_OK), "missing executable: %s" % binary)
    uid = int(run('/usr/bin/id','-u','chillcenter',capture_output=True).stdout) if args.resume_fixture else install_boundary_fixture(args.updater, args.server, args.repo)
    refresh_fixture(args.repo, args.updater, args.server)
    if args.reset_fixture_ledger:
        ledger=pathlib.Path('/var/lib/chillcenter-update/trust/sequences.json')
        if ledger.exists(): ledger.rename(ledger.with_name('test-history-'+str(uuid.uuid4())+'.json'))
        # Reset only the synthetic web slot after a prior successful fixture run.
        current=pathlib.Path('/usr/lib/chillcenter/web/current')
        if current.is_symlink(): current.unlink()
    service_boundary(uid)
    require(pathlib.Path('/etc/chillcenter').stat().st_uid == 0, 'systemd transferred configuration ownership to service')
    certificate = pathlib.Path('/var/lib/chillcenter/data/tls/certificate.pem')
    shutil.copy2(certificate, '/etc/chillcenter/health-ca.pem'); os.chown('/etc/chillcenter/health-ca.pem',0,0)
    candidate = activation_and_failure_boundaries()
    manifest,state_path=offline_cache_and_kill_boundaries(args.server, candidate)
    state_machine_and_expiry_boundaries(args.server,manifest,state_path)
    # Exact live-root checks: Linux root-only ledger plus production recovery under
    # the real service sandbox.  These are intentionally fail-closed if the
    # cross-built binaries/Minisign/systemd image do not meet prerequisites.
    env = dict(os.environ, CHILL_UPDATE_DISPOSABLE_VM="1")
    run("/usr/bin/systemctl", "show", "chillcenter.service", "-p", "ProtectSystem", "-p", "ReadWritePaths", "-p", "ProtectProc", "-p", "ProcSubset")
    run("/usr/lib/chillcenter/chill-update", "--config", "/etc/chillcenter/updates.json", "recover", "server", env=env)
    print("PASS: real service UID root-slot boundary and root recovery preflight")
    print("LIMITS: disposable Ubuntu ARM64 only; no Pi/NAS, physical Mac confirmation, power-cut/disk-full, or GitHub transport fault simulation.")

if __name__ == "__main__":
    main()
