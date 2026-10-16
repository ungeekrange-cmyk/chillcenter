"use strict";

// Local only: fake SSH makes a target-validation regression unable to contact a host.
const assert=require("node:assert/strict");
const childProcess=require("node:child_process");
const fs=require("node:fs");
const os=require("node:os");
const path=require("node:path");

const root=path.resolve(__dirname,"..");
const gate=path.join(root,"docs/releases/G-INTEGRATION-PI-001.md");
const fixture=path.join(root,"scripts/install-pi-first-install.sh");
const fakeDir=fs.mkdtempSync(path.join(os.tmpdir(),"chillcenter-fake-ssh-"));
const sshMarker=path.join(fakeDir,"ssh-called");
fs.writeFileSync(path.join(fakeDir,"ssh"),`#!/usr/bin/env bash
touch "\${CHILLCENTER_FAKE_SSH_MARKER}"
exit 99
`,{mode:0o755});

function run(script,args,environment={}){
 const env={...process.env,PATH:fakeDir+path.delimiter+process.env.PATH,CHILLCENTER_FAKE_SSH_MARKER:sshMarker,...environment};
 for(const [name,value] of Object.entries(env))if(value===undefined)delete env[name];
 const result=childProcess.spawnSync("bash",[path.join(root,script),...args],{cwd:root,encoding:"utf8",env});
 assert.notEqual(result.status,0,`${script} must refuse incomplete or unsafe target input`);
 assert.equal(fs.existsSync(sshMarker),false,`${script} must reject before SSH`);
 return result.stderr+result.stdout;
}

const common={
 "scripts/install-pi-first-install.sh":[
  "--archive",fixture,"--sha256","0".repeat(64),"--integration-gate",gate,"--go"
 ],
 "scripts/apply-working-directory.sh":[
  "--dropin",fixture,"--sha256","0".repeat(64),"--vendor-unit-sha256","0".repeat(64),
  "--integration-gate",gate,"--ssh-control","/private/tmp/no-control","--go"
 ],
 "scripts/finish-pi-r7-account.sh":[
  "--archive",fixture,"--sha256","0".repeat(64),"--integration-gate",gate,
  "--ssh-control","/private/tmp/no-control","--go"
 ]
};

for(const [script,args] of Object.entries(common)){
 fs.rmSync(sshMarker,{force:true});
 const missing=run(script,args,{CHILLCENTER_TARGET_HOST:undefined});
 assert.match(missing,/CHILLCENTER_TARGET_HOST: set the explicit SSH target user@host/);
 fs.rmSync(sshMarker,{force:true});
 const invalid=run(script,args,{
  CHILLCENTER_TARGET_HOST:"-operator@192.168.50.9",
  CHILLCENTER_LISTEN_ADDRESS:"192.168.50.9:8443",
  CHILLCENTER_TLS_NAME:"192.168.50.9",
  CHILLCENTER_NAS_SOURCE:"//192.0.2.92/Video"
 });
 assert.match(invalid,/Invalid CHILLCENTER_TARGET_HOST/);
}

const firstInstall=fs.readFileSync(path.join(root,"scripts/install-pi-first-install.sh"),"utf8");
const finish=fs.readFileSync(path.join(root,"scripts/finish-pi-r7-account.sh"),"utf8");
const forbiddenHouseholdPrefix=["192","168","1"].join(".")+".";
for(const source of [firstInstall,finish]){
 assert(!source.includes(forbiddenHouseholdPrefix));
 assert(source.includes('nas_source=$3'));
}
assert(finish.includes('sudo -n bash -s -- "$root" "$nas_source"'));
assert(finish.includes('root=$1\nnas_source=$2'));
assert(!firstInstall.includes("admin@"+forbiddenHouseholdPrefix));
assert(!fs.existsSync(sshMarker));
fs.rmSync(fakeDir,{recursive:true,force:true});
console.log("operator address configuration: PASS (fake SSH unused)");
