const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
function harness(seconds,{available=true,error=null}={}){
 let now=0,nextTimer=0;const timers=new Map(),calls=[],elements=new Map(),listeners={};
 function element(){return {hidden:false,disabled:false,textContent:'',value:'',dataset:{},classList:{add(){},remove(){}},append(){},replaceChildren(){},querySelector(){return null;},getBoundingClientRect(){return {x:0,y:0,width:320,height:180};}};}
 const get=id=>{if(!elements.has(id))elements.set(id,element());return elements.get(id);};
 const document={body:element(),hidden:false,activeElement:null,querySelector:get,addEventListener:(type,fn)=>listeners[type]=fn};
 const sandbox={document,window:{addEventListener(){},__TAURI__:{core:{invoke:async(_,{command})=>{calls.push(command);return {stage:'closed'};}}}},$:get,node:element,message(){},Date:{now:()=>now},setTimeout:(fn,delay)=>{const id=++nextTimer;timers.set(id,{fn,at:now+delay});return id;},clearTimeout:id=>timers.delete(id),setInterval:()=>0,clearInterval(){},requestAnimationFrame:fn=>fn(),ResizeObserver:class{observe(){}},loadPage:async()=>{}};
 vm.createContext(sandbox);vm.runInContext(fs.readFileSync(__dirname+'/player.js','utf8'),sandbox);
 const state={stage:'player',media_id:'a',native:{ended:true,error},autoplay_seconds:seconds,next:{available,new_season:true,media:{id:'b',title:'Episode suivant',available,poster:null}}};
 vm.runInContext('playerState='+JSON.stringify(state)+'; renderNext();',sandbox);
 return {calls,get,sandbox,timers,listeners,async advance(ms){const end=now+ms;for(let i=0;i<1000;i++){const entry=[...timers].filter(([,t])=>t.at<=end).sort((a,b)=>a[1].at-b[1].at)[0];if(!entry)break;const [id,t]=entry;timers.delete(id);now=t.at;await t.fn();}now=end;await Promise.resolve();}};
}
for(const seconds of [0,5,10,20,30])test('shipped next controller launches exactly after '+seconds+' seconds',async()=>{
 const h=harness(seconds);assert.equal(h.get('player-next-season').textContent,'Nouvelle saison');
 if(seconds) {await h.advance(seconds*1000-1);assert.equal(h.calls.length,0);await h.advance(1);}else await h.advance(0);
 assert.equal(h.calls.length,1);assert.equal(h.calls[0].action,'next');assert.equal(h.calls[0].expected_id,'b');
});
test('off, unavailable and decoding error never auto-launch',async()=>{
 for(const h of [harness(null),harness(0,{available:false}),harness(0,{error:'decoder error'})]){await h.advance(60000);assert.equal(h.calls.length,0);}
});
test('cancel, backgrounding and profile-close cancel timers without changing preference',async()=>{
 for(const action of ['cancelNext();','document.hidden=true;','hidePlayer();']){const h=harness(10);vm.runInContext(action,h.sandbox);
  if(action.startsWith('document'))h.listeners.visibilitychange();
  await h.advance(30000);assert.equal(h.calls.length,0);
  if(!action.startsWith('hide'))assert.equal(vm.runInContext('playerState.autoplay_seconds',h.sandbox),10);
 }
});
test('launch-now skips delay and sends only expected next identity',async()=>{
 const h=harness(30);await vm.runInContext('launchNext();',h.sandbox);assert.equal(h.calls.length,1);assert.equal(h.calls[0].expected_id,'b');await h.advance(30000);assert.equal(h.calls.length,1);
});
