"use strict";
// Static contract checks for root-only paths. They intentionally never invoke
// systemctl, network, Pi/NAS, or the installer as root.
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path");
const root=path.resolve(__dirname,"..");
const helper=fs.readFileSync(path.join(root,"crates/chill-updates/src/main.rs"),"utf8");
const install=fs.readFileSync(path.join(root,"scripts/install-server.sh"),"utf8");
const updateUnit=fs.readFileSync(path.join(root,"deploy/updates/chillcenter-update.service"),"utf8");
const serviceUnit=fs.readFileSync(path.join(root,"deploy/updates/chillcenter.service"),"utf8");
assert.match(helper,/\.download-\{\}/);assert.match(helper,/fs::rename\(&stage, &directory\)/);assert.match(helper,/abandoned_cache_temporary_kind/);assert.match(helper,/File::open\(&stage\)\?\.sync_all/);
assert.match(helper,/HelperLease/);assert.match(helper,/LOCK_EX \| libc::LOCK_NB/);assert.match(helper,/Some\("recover" \| "rescue"\)/);assert.match(helper,/OperationStatus::DeferredBusy/);assert.match(helper,/Some\("available"\)/);assert.match(helper,/due_attempts_for/);assert.match(helper,/authorize_activation\(root, &authorization\)/);assert.doesNotMatch(helper,/write_new_sync\(&authorization/);
assert.match(install,/^#!\/bin\/bash -p$/m);assert.match(install,/export PATH=\/usr\/sbin:\/usr\/bin:\/sbin:\/bin/);assert.match(install,/unset BASH_ENV ENV CDPATH GLOBIGNORE PYTHONPATH PYTHONHOME PYTHONSTARTUP PYTHONUSERBASE PYTHONINSPECT PYTHONWARNINGS/);assert.match(install,/"\$PYTHON" -I -/);assert.match(install,/"\$MINISIGN" -Vm/);
assert.match(updateUnit,/TimeoutStartSec=10min/);assert.match(updateUnit,/TimeoutStopSec=150s/);assert.match(updateUnit,/ExecStopPost=-\/usr\/lib\/chillcenter\/chill-update --config \/etc\/chillcenter\/updates\.json rescue/);
assert.match(serviceUnit,/^ReadWritePaths=\/var\/lib\/chillcenter$/m);
assert.doesNotMatch(serviceUnit,/^ReadWritePaths=.*\/usr\/lib\/chillcenter/m);
for(const component of ["server","web","themes"])assert.ok(serviceUnit.includes(`ExecStartPre=+/usr/lib/chillcenter/chill-update --config /etc/chillcenter/updates.json recover ${component}`));
console.log("PASS update helper hardening static contract: atomic cache, serialized helper, constrained bootstrap, recovery paths.");
