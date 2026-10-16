const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
function harness(seconds,{available=true,error=null,terminal=false,context=null,resolved=true}={}){
 let now=0,nextTimer=0;const timers=new Map(),calls=[],elements=new Map(),listeners={};
 function element(){const classes=new Set(),events={};return {hidden:false,disabled:false,textContent:'',value:'',dataset:{},events,attributes:{},classList:{add(...items){items.forEach(x=>classes.add(x));},remove(...items){items.forEach(x=>classes.delete(x));},contains:x=>classes.has(x),toggle(x,enabled){if(enabled)classes.add(x);else classes.delete(x);}},setAttribute(key,value){this.attributes[key]=value;},getAttribute(key){return this.attributes[key]??null;},addEventListener(type,fn){events[type]=fn;},append(){},replaceChildren(){},querySelector(){return null;},getBoundingClientRect(){return {x:0,y:0,width:320,height:180};}};}
 const get=id=>{if(!elements.has(id))elements.set(id,element());return elements.get(id);};
 const document={body:element(),hidden:false,activeElement:null,querySelector:get,querySelectorAll:()=>[],createElementNS:element,addEventListener:(type,fn)=>listeners[type]=fn};
 const sandbox={document,window:{addEventListener(){},__TAURI__:{core:{invoke:async(_,{command})=>{calls.push(command);return {stage:'closed'};}}}},$:get,node:element,message(){},Date:{now:()=>now},setTimeout:(fn,delay)=>{const id=++nextTimer;timers.set(id,{fn,at:now+delay});return id;},clearTimeout:id=>timers.delete(id),setInterval:()=>0,clearInterval(){},requestAnimationFrame:fn=>fn(),ResizeObserver:class{observe(){}},loadPage:async()=>{}};
 vm.createContext(sandbox);vm.runInContext(fs.readFileSync(__dirname+'/player.js','utf8'),sandbox);
 sandbox.state={server_id:'server',profile_id:'profile'};
 const state={stage:'player',media_id:'a',context,native:{ended:true,error},autoplay_seconds:seconds,next:resolved?{available,new_season:true,media:terminal?null:{id:'b',title:'Episode suivant',available,poster:null}}:null};
 vm.runInContext('playerState='+JSON.stringify(state)+'; renderNext();',sandbox);
 return {calls,get,sandbox,timers,listeners,async advance(ms){const end=now+ms;for(let i=0;i<1000;i++){const entry=[...timers].filter(([,t])=>t.at<=end).sort((a,b)=>a[1].at-b[1].at)[0];if(!entry)break;const [id,t]=entry;timers.delete(id);now=t.at;await t.fn();}now=end;await Promise.resolve();}};
}
for(const seconds of [0,5,10,15,20,30])test('shipped next controller launches exactly after '+seconds+' seconds',async()=>{
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
test('the next thumbnail is keyboard-accessible and starts the same validated next video',async()=>{
 const h=harness(30);const thumb=h.get('player-next-thumb');assert.equal(thumb.attributes.role,'button');assert.equal(thumb.attributes['aria-disabled'],'false');thumb.onclick();assert.equal(h.calls.length,1);assert.equal(h.calls[0].action,'next');
 const unavailable=harness(30,{available:false}),blocked=unavailable.get('player-next-thumb');assert.equal(blocked.attributes['aria-disabled'],'true');blocked.onclick();assert.equal(unavailable.calls.length,0);
});
function playing(h,overrides={}){
 const native={loaded:true,paused:false,buffering:false,ended:false,error:null,...overrides};
 vm.runInContext('playerState={stage:"player",native:'+JSON.stringify(native)+'};showPlayerControls();',h.sandbox);
}
test('controls hide after 2.5 seconds of playing inactivity and pointer brings them back',async()=>{
 const h=harness(null);playing(h);await h.advance(2499);assert.equal(h.get('player-panel').classList.contains('controls-hidden'),false);
 await h.advance(1);assert.equal(h.get('player-panel').classList.contains('controls-hidden'),true);
 h.get('player-panel').events.pointermove();assert.equal(h.get('player-panel').classList.contains('controls-hidden'),false);
 await h.advance(2500);assert.equal(h.get('player-panel').classList.contains('controls-hidden'),true);
});
test('pause, buffering, loading, ended, focus and menu hover pin native-overlay controls',async()=>{
 for(const native of [{paused:true},{buffering:true},{loaded:false},{ended:true},{error:'failed'}]){
  const h=harness(null);playing(h,native);await h.advance(5000);assert.equal(h.get('player-panel').classList.contains('controls-hidden'),false);
 }
 for(const source of ['focus','hover']){
  const h=harness(null);playing(h);
  if(source==='focus')h.sandbox.document.activeElement={closest:()=>h.get('player-overlay')};
  else h.get('player-overlay').events.pointerenter();
  await h.advance(5000);assert.equal(h.get('player-panel').classList.contains('controls-hidden'),false);
  if(source==='focus'){h.sandbox.document.activeElement=null;h.get('player-panel').events.focusout();}
  else h.get('player-overlay').events.pointerleave();
  await h.advance(2500);assert.equal(h.get('player-panel').classList.contains('controls-hidden'),true);
 }
});
test('native fullscreen state drives viewport class and button accessibility',()=>{
 const h=harness(null);playing(h,{fullscreen:true});vm.runInContext('syncPlayerFullscreen();',h.sandbox);
 assert.equal(h.sandbox.document.body.classList.contains('player-fullscreen'),true);assert.equal(h.get('player-fullscreen').attributes['aria-pressed'],'true');
 playing(h,{fullscreen:false});vm.runInContext('syncPlayerFullscreen();',h.sandbox);assert.equal(h.sandbox.document.body.classList.contains('player-fullscreen'),false);
});
test('keyboard reveals controls but leaves focused native form controls and buttons to their own keys',async()=>{
 const h=harness(null);playing(h);await h.advance(2500);
 h.listeners.keydown({key:' ',code:'Space',target:{tagName:'BUTTON'},preventDefault(){throw Error('button key stolen');}});
 assert.equal(h.get('player-panel').classList.contains('controls-hidden'),false);assert.equal(h.calls.length,0);
});

for(const kind of ['series','saga','playlist','standalone'])test('confirmed terminal '+kind+' closes once, syncs watched state and returns to its origin',async()=>{
 const context=kind==='standalone'?{kind}:{kind,id:kind+'-fixture'},h=harness(20,{terminal:true,context}),returned=[];
 let synced=0;h.sandbox.syncAfterMutation=async()=>{synced++;};
 h.sandbox.window.catalogReturnFromPlayback=async(playback,current)=>{assert.equal(current(),true);returned.push(JSON.parse(JSON.stringify(playback)));};
 vm.runInContext('renderNext();renderNext();',h.sandbox);
 await h.advance(0);await new Promise(resolve=>setImmediate(resolve));
 assert.equal(synced,1);assert.deepEqual(h.calls.map(c=>c.action),['stop']);
 assert.equal(h.get('player-panel').hidden,true);
 assert.deepEqual(returned,[{media_id:'a',context}]);
});
test('EOF with another episode refreshes personal state once without closing or delaying the proposal',async()=>{
 const h=harness(20),returned=[];let synced=0;
 h.sandbox.syncAfterMutation=async()=>{synced++;};h.sandbox.window.catalogReturnFromPlayback=p=>returned.push(p);
 await h.advance(0);vm.runInContext('renderNext();renderNext();',h.sandbox);await h.advance(0);
 assert.equal(synced,1);assert.equal(h.get('player-next').hidden,false);assert.equal(h.calls.length,0);assert.equal(returned.length,0);
 await h.advance(20000);assert.deepEqual(h.calls.map(c=>c.action),['next']);
});
test('an unavailable next item, a decoder error or an unresolved EOF never closes the player',async()=>{
 for(const options of [{available:false},{terminal:true,error:'decoder error'},{terminal:true,resolved:false}]){
  const h=harness(0,options);await h.advance(60000);assert.equal(h.calls.length,0);
 }
});
test('cancel returns to the original series and does not launch another episode',async()=>{
 const h=harness(20,{context:{kind:'series',id:'series-key'}}),returned=[];
 let synced=0;h.sandbox.syncAfterMutation=async()=>{synced++;};
 h.sandbox.window.catalogReturnFromPlayback=p=>returned.push(JSON.parse(JSON.stringify(p)));
 await h.advance(0);
 await h.get('player-next-cancel').onclick();await h.advance(30000);
 assert.equal(synced,1,'cancellation reuses the EOF sync instead of issuing a second delta request');
 assert.deepEqual(h.calls.map(c=>c.action),['stop']);assert.deepEqual(returned,[{media_id:'a',context:{kind:'series',id:'series-key'}}]);
});
test('failed stop leaves the current page intact',async()=>{
 const h=harness(20,{terminal:true});let returned=0;
 h.sandbox.window.__TAURI__.core.invoke=async()=>{throw Error('stop failed');};
 h.sandbox.window.catalogReturnFromPlayback=()=>returned++;
 await h.advance(0);await new Promise(resolve=>setImmediate(resolve));assert.equal(returned,0);
});
for(const change of ['state.profile_id="other";','++playerNavigationEpoch;'])test('delayed completion cannot replace a newer profile or navigation: '+change,async()=>{
 const h=harness(20,{terminal:true}),returned=[];let resolveSync;
 h.sandbox.state={server_id:'server',profile_id:'profile'};
 h.sandbox.syncAfterMutation=()=>new Promise(resolve=>{resolveSync=resolve;});
 h.sandbox.window.catalogReturnFromPlayback=p=>returned.push(p);
 await h.advance(0);assert.deepEqual(h.calls.map(c=>c.action),['stop']);
 vm.runInContext(change,h.sandbox);resolveSync();await new Promise(resolve=>setImmediate(resolve));assert.equal(returned.length,0);
});
test('a completion scheduled before a newer launch cannot stop it',async()=>{
 const h=harness(20,{terminal:true});vm.runInContext('++playerNavigationEpoch;',h.sandbox);
 await h.advance(0);assert.equal(h.calls.length,0);
});
test('unknown series order is not a terminal EOF and never closes or auto-launches',async()=>{
 const h=harness(0,{terminal:true,context:{kind:'series',id:'flat'}});
 vm.runInContext('playerState.next.order_unknown=true;',h.sandbox);
 // The deferred EOF task must recheck the resolved state, not a guessed absence.
 await h.advance(30000);assert.equal(h.calls.length,0);assert.equal(h.get('player-next').hidden,true);
});
test('expiry reports launching, never cancellation, while the next command is pending',async()=>{
 const h=harness(5);let finish;
 h.sandbox.window.__TAURI__.core.invoke=async(_,{command})=>{h.calls.push(command);return new Promise(resolve=>{finish=resolve;});};
 await h.advance(5000);
 assert.match(h.get('player-next-count').textContent,/Lancement de/);
 assert.doesNotMatch(h.get('player-next-count').textContent,/annul/i);
 assert.equal(h.get('player-next-cancel').disabled,true);
 await vm.runInContext('launchNext();',h.sandbox);assert.equal(h.calls.length,1);
 finish({stage:'closed'});await new Promise(resolve=>setImmediate(resolve));
});
test('a launch queued behind status cannot survive cancellation',async()=>{
 const h=harness(5);vm.runInContext('playerBusy=true;',h.sandbox);
 await h.advance(5000);assert.equal(h.calls.length,0);
 vm.runInContext('cancelNext();playerBusy=false;',h.sandbox);
 await h.advance(1000);assert.equal(h.calls.length,0);
});
test('an old queued launch cannot launch a replacement proposal',async()=>{
 const h=harness(5);vm.runInContext('playerBusy=true;',h.sandbox);await h.advance(5000);
 vm.runInContext('playerState.next.media.id="c";playerState.autoplay_seconds=null;renderNext();playerBusy=false;',h.sandbox);
 await h.advance(1000);assert.equal(h.calls.length,0);
});
test('backgrounding also cancels a launch waiting for the status lock',async()=>{
 const h=harness(5);vm.runInContext('playerBusy=true;',h.sandbox);await h.advance(5000);
 h.sandbox.document.hidden=true;h.listeners.visibilitychange();
 vm.runInContext('playerBusy=false;',h.sandbox);await h.advance(1000);assert.equal(h.calls.length,0);
});
test('transition artwork is prepared eight seconds before EOF but never covers live video',()=>{
 const h=harness(null),observed=[],retired=[];
 h.sandbox.window.chillArtwork=(target,id)=>observed.push({target,id});
 h.sandbox.window.retireArtwork=target=>retired.push(target);
 vm.runInContext('playerState={stage:"player",media_id:"current",position:91,duration:100,native:{loaded:true,ended:false}};renderPlayerBackdrop();',h.sandbox);
 assert.equal(observed.length,0);
 vm.runInContext('playerState.position=92;renderPlayerBackdrop();renderPlayerBackdrop();',h.sandbox);
 assert.deepEqual(observed.map(x=>x.id),['current']);
 assert.equal(h.get('player-stage').classList.contains('transition-visible'),false);
 vm.runInContext('playerState.native.ended=true;renderPlayerBackdrop();',h.sandbox);
 assert.equal(h.get('player-stage').classList.contains('transition-visible'),true);
 vm.runInContext('hidePlayer();',h.sandbox);
 assert.equal(h.get('player-stage').classList.contains('transition-visible'),false);
 assert.ok(retired.includes(h.get('player-backdrop')));
});
test('transition artwork is reset for another media or profile, with a dark EOF fallback without an image',()=>{
 const h=harness(null),observed=[];
 h.sandbox.window.chillArtwork=(target,id)=>observed.push(id);
 vm.runInContext('playerState={stage:"player",media_id:"a",position:99,duration:100,native:{loaded:true,ended:false}};renderPlayerBackdrop();playerState.media_id="b";playerState.position=1;renderPlayerBackdrop();',h.sandbox);
 assert.deepEqual(observed,['a']);
 vm.runInContext('playerState.native.ended=true;renderPlayerBackdrop();',h.sandbox);
 assert.deepEqual(observed,['a','b']);
 assert.equal(h.get('player-stage').classList.contains('transition-visible'),true);
 vm.runInContext('state.profile_id="other";renderPlayerBackdrop();',h.sandbox);
 assert.deepEqual(observed,['a','b','b']);
});
