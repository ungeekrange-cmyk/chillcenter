const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
function harness(invoke){
 const nodes=[],ids=new Map(),timers=new Map();let counter=0;
 class Element{constructor(tag,text){this.tag=tag;this.textContent=text||'';this.children=[];this.value='';this.hidden=false;this.disabled=false;nodes.push(this);}append(...items){this.children.push(...items);}setAttribute(){}addEventListener(){}querySelector(){return grid;}}
 const grid=new Element('div');const byId=id=>{if(!ids.has(id))ids.set(id,new Element('div'));return ids.get(id);};
 const state={stage:'connected',server_id:'s',profile_id:'p'},window={},messages=[];
 vm.runInNewContext(fs.readFileSync(__dirname+'/tmdb.js','utf8'),{window,document:{},state,$:byId,message:value=>messages.push(value),node:(tag,text)=>new Element(tag,text),invoke,confirm:()=>true,loadPage:()=>{},setTimeout:fn=>{const id=++counter;timers.set(id,fn);return id;},clearTimeout:id=>timers.delete(id)});
 return {nodes,byId,state,window,timers,messages,async tick(){const [id,fn]=timers.entries().next().value;timers.delete(id);await fn();}};
}
test('enrichment immediately reports progress above the administration fold and then real counters',async()=>{
 let resolve;const waiting=new Promise(r=>resolve=r),h=harness(()=>waiting);
 const action=h.byId('tmdb-enrich').onclick();assert.match(h.messages[0],/demandé/);assert.equal(h.byId('tmdb-enrich').disabled,true);
 resolve({job_id:'j',status:'queued',total:4,processed:0});await action;assert.match(h.messages.at(-1),/Traités : 0 \/ 4/);
});
test('TMDB password is cleared before invocation, never restored after failure',async()=>{
 let resolve,captured;const pending=new Promise(r=>resolve=r),h=harness(c=>{captured=c;return pending;});
 const form=h.nodes.find(n=>n.tag==='form'),password=h.nodes.find(n=>n.type==='password');password.value='synthetic-only';
 const submit=form.onsubmit({preventDefault(){}});assert.equal(password.value,'');assert.equal(captured.credential,'synthetic-only');resolve(null);await submit;assert.equal(password.value,'');
});
test('expired job stops polling and does not claim server needs update',async()=>{
 const h=harness(async c=>{if(c.action==='tmdb_job_start')return {job_id:'j',status:'queued',total:1};throw {status:404,code:'TMDB_JOB_NOT_FOUND'};});
 await h.byId('tmdb-enrich').onclick();assert.equal(h.timers.size,1);await h.tick();
 assert.equal(h.timers.size,0);assert.ok(h.nodes.some(n=>n.textContent.includes('Ce travail a expiré')));
 assert.equal(h.nodes.find(n=>n.textContent==='Annuler l’enrichissement').hidden,true);
});
test('bounded-job errors explain saturation and do not automatically start another job',async()=>{
 let calls=0;const h=harness(async()=>{calls++;throw {status:429,code:'TMDB_JOB_BUSY'};});await h.byId('tmdb-enrich').onclick();
 assert.equal(calls,1);assert.equal(h.timers.size,0);assert.ok(h.nodes.some(n=>n.textContent.startsWith('Deux enrichissements')));
});
