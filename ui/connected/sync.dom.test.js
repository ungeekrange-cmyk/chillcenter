const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
function fixture(){const pending=[],calls=[],icon={hidden:false,classList:{add(){},remove(){}},setAttribute(){}};let loads=0;const sandbox={state:{stage:'connected',server_id:'s',profile_id:'A'},$:()=>icon,node:()=>icon,invoke:()=>new Promise(resolve=>{calls.push(sandbox.state.profile_id);pending.push(resolve);}),loadPage:async()=>{loads++;return true;}};vm.createContext(sandbox);const source=fs.readFileSync(__dirname+'/app.js','utf8');vm.runInContext(source.slice(source.indexOf('let syncPromise='),source.indexOf('\nwindow.addEventListener("focus"')),sandbox);return {sandbox,calls,pending,icon,run:code=>vm.runInContext(code,sandbox),loads:()=>loads};}
test('a different profile queues its own check after the old context finishes',async()=>{const f=fixture();const a=f.run('syncCatalog()');f.sandbox.state.profile_id='B';const b=f.run('syncCatalog()');f.pending.shift()({changed:true});await a;await Promise.resolve();assert.deepEqual(f.calls,['A','B']);f.pending.shift()({changed:true});await b;assert.equal(f.loads(),1);});
test('mutation refresh waits for the old check then explicitly obtains a new revision',async()=>{const f=fixture();const old=f.run('syncCatalog()'),after=f.run('syncAfterMutation()');assert.deepEqual(f.calls,['A']);f.pending.shift()({changed:false,discarded:true});await old;await Promise.resolve();assert.deepEqual(f.calls,['A','A']);f.pending.shift()({changed:true});await after;assert.equal(f.loads(),1);});
test('web acknowledges only a successful complete refresh and retries after failure',async()=>{
 const f=fixture(),commands=[];let success=false;
 f.sandbox.loadPage=async()=>success;
 f.sandbox.invoke=async command=>{commands.push(command.action);return command.action==='sync_catalog'?{changed:true,ack_ticket:7}:{discarded:false};};
 await f.run('syncCatalog()');assert.deepEqual(commands,['sync_catalog']);assert.match(f.icon.title,/conservé/);
 success=true;await f.run('syncCatalog()');assert.deepEqual(commands,['sync_catalog','sync_catalog','ack_catalog_sync']);assert.equal(f.icon.title,'Catalogue à jour');
});
test('manual connection and certificate approval start a detached warm-cache check',async()=>{
 const source=fs.readFileSync(__dirname+'/app.js','utf8');
 for(const mode of ['connect','approve']){
  let loaded=0,checked=0;const control={};
  const sandbox={state:{theme:'dark',token:'token'},render(){},run:async()=>({profile_id:'A'}),loadPage:async()=>loaded++,syncCatalog:()=>{checked++;return new Promise(()=>{});},$:()=>control};
  vm.createContext(sandbox);
  if(mode==='connect'){vm.runInContext(source.slice(source.indexOf('async function connect('),source.indexOf('\nasync function selectProfile')),sandbox);await vm.runInContext('connect("chillcenter.local")',sandbox);}
  else{vm.runInContext(source.split('\n').find(line=>line.startsWith('$("approve-trust").onclick=')),sandbox);await control.onclick();}
  assert.equal(loaded,1);assert.equal(checked,1,'Synchronization starts without awaiting network completion');
 }
});
