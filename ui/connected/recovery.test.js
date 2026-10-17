const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const {isRevisionConflict,captureRecoveryFields,restoreRecoveryFields}=require('./recovery.js');
test('revision recovery never mislabels busy jobs, scan locks or playback transfer conflicts',()=>{
 for(const code of ['PROFILE_CONFLICT','CONFIG_CONFLICT'])assert.equal(isRevisionConflict({code,status:409}),true);
 for(const code of ['CLIENT_BUSY','SCAN_ACTIVE','TMDB_JOB_BUSY','LEASE_ACTIVE','IMPORT_PLAN_STALE'])assert.equal(isRevisionConflict({code,status:409}),false);
 assert.equal(isRevisionConflict({code:'PROFILE_CONFLICT',status:503}),false);
});
test('draft capture excludes secrets/files and restores replaced controls by stable identity',()=>{
 const old={id:'title',value:'my draft',checked:false},secret={id:'secret',type:'password',value:'never-store-this'},file={type:'file',value:'no-path'};
 const fields=captureRecoveryFields({querySelectorAll:()=>[old,secret,file]});assert.equal(fields.length,1);
 const replacement={id:'title',value:'new server value',checked:false};restoreRecoveryFields(fields,{querySelectorAll:()=>[replacement]});assert.equal(replacement.value,'my draft');
});
function fixture(reload){
 const messages=[],root={children:[],prepend(n){this.children.unshift(n);}},state={stage:'connected',server_id:'s',profile_id:'p'};
 const node=(tag,text)=>({tag,textContent:text,children:[],append(...n){this.children.push(...n);},setAttribute(){},remove(){root.children=root.children.filter(n=>n!==this);}});
 const window={},context={window,state,node,uiText:x=>x,message:x=>messages.push(x),document:{addEventListener(){}}};
 vm.runInNewContext(fs.readFileSync(__dirname+'/recovery.js','utf8'),context);
 const target={root,label:'Actualiser cette fiche',current:()=>state.profile_id==='p',reload};
 window.offerConflictRecovery({status:409,code:'PROFILE_CONFLICT'},target);
 return {window,state,root,messages,button:root.children[0].children[1],status:root.children[0].children[0]};
}
test('only the explicit recovery button performs one read; double-clicks never replay writes',async()=>{
 let calls=0,resolve;const wait=new Promise(r=>resolve=r),f=fixture(async()=>{calls++;await wait;return true;});assert.equal(calls,0);
 const first=f.button.onclick();await f.button.onclick();assert.equal(calls,1);assert.equal(f.button.disabled,true);resolve();await first;
 assert.equal(f.root.children.length,0);assert.equal(f.messages.length,1);assert.equal(f.button.disabled,false);
});
test('failed recovery retains its draft-facing action and is explicitly retryable',async()=>{
 const f=fixture(async()=>{throw Error('offline');});await f.button.onclick();assert.match(f.status.textContent,/saisie est conservée/);assert.equal(f.button.disabled,false);assert.equal(f.root.children.length,1);
});
test('profile change during recovery cannot show success or a stale recovery panel',async()=>{
 let resolve;const wait=new Promise(r=>resolve=r),f=fixture(async()=>{await wait;return false;});const pending=f.button.onclick();f.state.profile_id='other';f.window.recoveryContextChanged();resolve();await pending;
 assert.equal(f.root.children.length,0);assert.deepEqual(f.messages,[]);
});
test('all recovery labels and error paths have English translations',()=>{
 const english=require('./locales.js');for(const file of ['recovery.js']){
  const text=fs.readFileSync(__dirname+'/'+file,'utf8');
  for(const [,literal]of text.matchAll(/uiText\("([^"]+)"\)/g))assert.equal(typeof english[literal],'string',literal);
 }
 assert.equal(english['Actualisation différée. Réessayez dans un instant.'],'Refresh deferred. Try again shortly.');
});
