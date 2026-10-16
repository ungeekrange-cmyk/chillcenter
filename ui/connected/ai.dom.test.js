// Exercise the shipped controller, including asynchronous invoke boundaries.
const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
function harness(invoke){
 const nodes=[],timers=new Map();let next=0;
 class Element{
  constructor(tag,text){this.tag=tag;this.textContent=text||'';this.children=[];this.disabled=false;this.hidden=false;this.value='';this.listeners={};nodes.push(this);}
  append(...items){this.children.push(...items);}replaceChildren(...items){this.children=items;}
  setAttribute(){}addEventListener(type,fn){this.listeners[type]=fn;}
  showModal(){this.open=true;}close(){this.open=false;}
  querySelector(){return grid;}querySelectorAll(){return this.children.filter(x=>['input','select','button'].includes(x.tag));}
 }
 const grid=new Element('div'),admin=new Element('section'),body=new Element('body');
 const state={server_id:'s',profile_id:'p'},window={__TAURI__:{core:{invoke}},confirm:()=>true};
 const ids=new Map([['admin-panel',admin]]);const byId=id=>{if(!ids.has(id))ids.set(id,new Element('div'));return ids.get(id);};
 const sandbox={window,document:{body},state,node:(tag,text)=>new Element(tag,text),$:byId,message:()=>{},setTimeout:fn=>{const id=++next;timers.set(id,fn);return id;},clearTimeout:id=>timers.delete(id),run:async()=>{},loadPage:()=>{}};
 vm.runInNewContext(fs.readFileSync(__dirname+'/ai.js','utf8'),sandbox);
 const find=text=>nodes.find(n=>n.textContent===text),form=nodes.find(n=>n.tag==='form');
 return {window,state,nodes,timers,find,form,byId,status:nodes.find(n=>n.tag==='p'&&n!==admin),async tick(){const [id,fn]=timers.entries().next().value||[];assert.ok(fn,'pending timer');timers.delete(id);await fn();}};
}
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};
const submit=h=>h.form.onsubmit({preventDefault(){}});
const working=id=>({id,stage:'working',mode:'organize'});
const preview=id=>({id,stage:'preview',mode:'organize',items:[],plan:{message:'Preview',changes:[{id:'m',rating:5}],matched_ids:[]}});
test('close before Start response cancels returned job and never resurrects poll',async()=>{
 const start=deferred(),calls=[],h=harness(async(_,{command:c})=>{calls.push(c);return c.action==='start'?start.promise:{};});
 const pending=submit(h);h.find('Fermer').onclick();start.resolve(working('j'));await pending;
 assert.equal(calls.filter(c=>c.action==='cancel'&&c.id==='j').length,1);assert.equal(h.timers.size,0);assert.equal(h.find('Proposer').disabled,false);
});
test('late rejection after profile switch cannot overwrite new-context status',async()=>{
 const start=deferred(),h=harness(async()=>start.promise),pending=submit(h);h.state.profile_id='other';h.window.aiContextChanged();start.reject({message:'STALE ERROR'});await pending;
 assert.equal(h.status.textContent,'');assert.equal(h.timers.size,0);
});
test('status CLIENT_BUSY is retried only for reads and eventually resumes polling',async()=>{
 let statusCalls=0;const h=harness(async(_,{command:c})=>{if(c.action==='start')return working('j');if(c.action==='status'){if(++statusCalls===1)throw {code:'CLIENT_BUSY'};return working('j');}});
 await submit(h);const polling=h.tick();for(let i=0;i<8&&!h.timers.size;i++)await Promise.resolve();assert.equal(h.timers.size,1);await h.tick();await polling;
 assert.equal(statusCalls,2);assert.equal(h.timers.size,1);
});
test('confirmed application cannot be dismissed or sent twice; stale response stays hidden',async()=>{
 const apply=deferred();let applyCalls=0;const h=harness(async(_,{command:c})=>{if(c.action==='start')return preview('j');if(c.action==='apply'){applyCalls++;return apply.promise;}});
 await submit(h);const button=h.find('Confirmer cette proposition'),pending=button.onclick();h.find('Fermer').onclick();await button.onclick();
 assert.equal(h.find('Fermer').disabled,true);assert.equal(applyCalls,1);
 h.state.profile_id='other';h.window.aiContextChanged();apply.resolve(working('j'));await pending;assert.equal(h.timers.size,0);assert.equal(h.status.textContent,'');
});
test('advice combines watched and categories before context cap and never applies',async()=>{
 const calls=[],h=harness(async(_,{command:c})=>{calls.push(c);return {id:'j',stage:'preview',mode:'search',items:[],plan:{message:'Advice',changes:[],matched_ids:[]}};});
 h.byId('advice-request').value='Un film drôle';h.byId('advice-watched').value='false';
 await h.byId('advice-form').onsubmit({preventDefault(){}});
 const start=calls.find(c=>c.action==='start');
 assert.equal(start.mode,'search');assert.equal(start.watched,false);assert.equal(start.available,true);
 assert.equal(start.recommendations,true);
 assert.deepEqual(Array.from(start.categories),['film','serie']);assert.equal(start.category,null);
 assert.equal(h.find('Confirmer cette proposition'),undefined);
 h.byId('advice-category').value='film';h.byId('advice-category').onchange();
 assert.equal(h.byId('advice-results').children.length,0);
 assert.equal(calls.filter(c=>c.action==='start').length,1);
});
test('changing profile clears both advice and organization results',async()=>{
 const h=harness(async()=>({}));h.byId('advice-results').append({textContent:'Previous profile'});
 h.state.profile_id='next';h.window.aiContextChanged();
 assert.equal(h.byId('advice-results').children.length,0);assert.equal(h.byId('advice-status').textContent,'');
});
