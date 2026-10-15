const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
function harness(invoke){
 const nodes=[],node=(tag,text)=>{const e={tag,textContent:text||'',children:[],append(...c){this.children.push(...c);}};nodes.push(e);return e;};
 const window={chillReady:Promise.resolve()},state={};
 vm.runInNewContext(fs.readFileSync(__dirname+'/player-settings.js','utf8'),{node,window,state,invoke,$:()=>({querySelector:()=>({append(){}})})});
 return {window,state,check:nodes.find(n=>n.id==='player-fullscreen-default'),form:nodes.find(n=>n.tag==='form'),status:nodes.find(n=>n.tag==='p'&&!n.textContent)};
}
test('fullscreen starts enabled for legacy settings, and only the Mac-local command is submitted',async()=>{
 const calls=[],f=harness(async c=>{calls.push(c);return {player_presentation:{fullscreen_on_start:c.fullscreen_on_start}};});await Promise.resolve();assert.equal(f.check.checked,true);
 f.check.checked=false;await f.form.onsubmit({preventDefault(){}});assert.deepEqual(JSON.parse(JSON.stringify(calls)),[{action:'save_player_settings',fullscreen_on_start:false}]);assert.equal(f.state.player_presentation.fullscreen_on_start,false);
});
test('a failed local save retains the previous preference and duplicate submits are ignored',async()=>{
 let reject,calls=0;const f=harness(()=>{calls++;return new Promise((_,r)=>reject=r);});await Promise.resolve();f.check.checked=false;
 const saving=f.form.onsubmit({preventDefault(){}});await f.form.onsubmit({preventDefault(){}});assert.equal(calls,1);
 reject({message:'Disque indisponible'});await saving;assert.equal(f.check.checked,true);assert.match(f.status.textContent,/Disque/);assert.equal(f.state.player_presentation,undefined);
});
