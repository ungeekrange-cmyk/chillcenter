"use strict";
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const {fixture}=require('./library-harness.js');
const movie=(id,extra={})=>({media:{id,title:id,category:'film',available:true,season:null,episode:null,...extra},series_key:null,season:null,progress:null,matches:[]});
const payload=(continue_watching=[],recently_added=[],recommendations=[])=>({continue_watching,recently_added,recommendations});
function home(read){const f=fixture(read);f.sandbox.state.stage='connected';f.sandbox.shellSection='home';f.sandbox.document.addEventListener=()=>{};vm.runInContext(fs.readFileSync(__dirname+'/../home.js','utf8'),f.sandbox);return f;}
const cards=rail=>rail.querySelectorAll('.recommendation-card');
test('Home has three ordered horizontal rows, hides empty rows and caps deduped entities at20',async()=>{
 const show={...movie('s',{category:'serie',series:'Show'}),series_key:'series:s'};
 const f=home(()=>payload([movie('resume')],[show,show,...Array.from({length:30},(_,i)=>movie('m'+i))],[movie('related')]));
 await f.sandbox.window.refreshHome();const rails=f.get('home-rails').children;
 assert.deepEqual(rails.map(r=>r.children[0].textContent),['Reprendre','Ajouts récents','Pour vous']);assert.equal(cards(rails[1]).length,20);
 assert.equal(rails[1].querySelectorAll('.recommendation-rail').length,1);assert.equal(f.calls.filter(c=>c.action==='catalog').length,0);
 const empty=home(()=>payload([],[],[]));await empty.sandbox.window.refreshHome();assert.equal(empty.get('home-rails').children.length,0);assert.equal(empty.get('home-empty').hidden,false);
});
test('Continue displays measured film progress and season0, opening a series with its selected season',async()=>{
 const show={...movie('show',{category:'serie',series:'Show'}),series_key:'tv:show',season:0,progress:{position_ms:2500,duration_ms:10000}};
 const film={...movie('film'),progress:{position_ms:4290,duration_ms:10000}};
 const f=home(()=>payload([show,film]));const opened=[];f.sandbox.window.catalogOpenSeries=(...args)=>opened.push(args);
 await f.sandbox.window.refreshHome();const result=cards(f.get('home-rails'));
 assert.equal(result[0].querySelectorAll('.home-badge')[0].textContent,'Saison 0 · 25 %');assert.equal(result[1].querySelectorAll('.home-badge')[0].textContent,'42 %');
 await result[0].onclick();assert.deepEqual(opened,[['tv:show',0]]);
 f.sandbox.state.profile_id='B';f.sandbox.window.clearHomeContext();await result[0].onclick();assert.equal(opened.length,1,'retained old cards cannot navigate for another profile');
});
test('profile changes clear old rows and offline errors never restore another profile snapshot',async()=>{
 let offline=false;const f=home(()=>{if(offline)throw Error('offline');return payload([movie('A')]);});await f.sandbox.window.refreshHome();
 f.sandbox.state.profile_id='B';f.sandbox.window.clearHomeContext();offline=true;await f.sandbox.window.refreshHome();assert.equal(f.get('home-rails').children.length,0);
 assert.match(f.get('home-status').textContent,/Accueil indisponible/);
});
test('A→B→A starts a new read and discards both older pending responses',async()=>{
 const waits=[];const f=home(()=>new Promise(resolve=>waits.push(resolve)));
 const a1=f.sandbox.window.refreshHome();f.sandbox.state.profile_id='B';f.sandbox.window.clearHomeContext();const b=f.sandbox.window.refreshHome();
 f.sandbox.state.profile_id='p';f.sandbox.window.clearHomeContext();const a2=f.sandbox.window.refreshHome();assert.equal(waits.length,3);
 waits[0](payload([movie('stale-A')]));waits[1](payload([movie('stale-B')]));await Promise.all([a1,b]);assert.equal(f.get('home-rails').children.length,0);
 waits[2](payload([movie('fresh-A')]));await a2;assert.equal(cards(f.get('home-rails'))[0].dataset.homeMediaId,'fresh-A');
});
test('same-context failures preserve rows; unsupported server clears them; hidden/player views issue no read',async()=>{
 let mode='ok';const f=home(()=>{if(mode==='offline')throw Error('offline');if(mode==='old')throw {status:404};return payload([movie('resume')]);});
 await f.sandbox.window.refreshHome();mode='offline';await f.sandbox.window.refreshHome();assert.equal(cards(f.get('home-rails')).length,1);
 mode='old';await f.sandbox.window.refreshHome();assert.equal(cards(f.get('home-rails')).length,0);assert.match(f.get('home-status').textContent,/mise à jour du serveur/);
 const before=f.calls.length;f.sandbox.document.hidden=true;await f.sandbox.window.refreshHome();f.sandbox.document.hidden=false;f.sandbox.document.body.classList.add('player-open');await f.sandbox.window.refreshHome();assert.equal(f.calls.length,before);
});
test('concurrent Home reads coalesce and a mutation during that read schedules a fresh snapshot',async()=>{
 let resolve;let count=0;const f=home(()=>++count===1?new Promise(r=>resolve=r):payload([movie('fresh')]));
 const first=f.sandbox.window.refreshHome();const second=f.sandbox.window.refreshHome(true);assert.equal(count,1);resolve(payload([movie('old')]));await Promise.all([first,second]);await new Promise(r=>setImmediate(r));assert.equal(count,2);assert.equal(cards(f.get('home-rails'))[0].dataset.homeMediaId,'fresh');
});
test('recent series opens with default season selection while unknown Continue opens null explicitly',async()=>{
 const series={...movie('show',{category:'serie',series:'Show'}),series_key:'show',season:null};const f=home(()=>payload([series],[series]));const calls=[];f.sandbox.window.catalogOpenSeries=(...args)=>calls.push(args);await f.sandbox.window.refreshHome();const result=cards(f.get('home-rails'));await result[0].onclick();await result[1].onclick();assert.deepEqual(calls,[['show',null],['show']]);
});

test('Continue renders an existing film resume without inventing a duration or percentage',async()=>{
 const f=home(()=>payload([movie('legacy-resume')]));
 await f.sandbox.window.refreshHome();const result=cards(f.get('home-rails'));
 assert.equal(result.length,1);assert.equal(result[0].dataset.homeMediaId,'legacy-resume');
 assert.equal(result[0].querySelectorAll('.home-badge').length,0);
});

test('season progress reports completed episodes out of the whole season, not one episode percentage',async()=>{
 const show={...movie('show',{category:'serie',series:'Show'}),series_key:'show',season:null,episode_counts:{watched:4,total:150},progress:{position_ms:17000,duration_ms:45000}};
 const f=home(()=>({...payload([show]),watch_revision:7}));await f.sandbox.window.refreshHome();
 assert.equal(cards(f.get('home-rails'))[0].querySelectorAll('.home-badge')[0].textContent,'Saison inconnue · 4 / 150 épisodes vus · 3 %');
 assert.equal(f.get('home-rails').querySelectorAll('.home-progress')[0].getAttribute('aria-valuenow'),'3');
});

test('watched action retains the card while saving, coalesces clicks and removes only after success',async()=>{
 let resolve,seen=false;const f=home(command=>{
  if(command.action==='set_watched')return new Promise(r=>resolve=()=>{seen=true;r({saved:true});});
  return payload(seen?[]:[movie('film')]);
 });await f.sandbox.window.refreshHome();const check=f.get('home-rails').querySelectorAll('.home-watched')[0];
 const saving=check.onclick();assert.equal(cards(f.get('home-rails')).length,1);assert.equal(check.disabled,true);
 await check.onclick();await f.sandbox.window.refreshHome(true);assert.equal(f.calls.filter(c=>c.action==='set_watched').length,1);assert.equal(cards(f.get('home-rails')).length,1);
 resolve();await saving;await new Promise(r=>setImmediate(r));assert.equal(cards(f.get('home-rails')).length,0);
});

test('failed or late profile watched writes never remove a current card or alter the new profile',async()=>{
 let mode='fail',resolve;const f=home(command=>{
  if(command.action==='set_watched') {if(mode==='fail')throw Error('not saved');return new Promise(r=>resolve=r);}
  return payload([movie(f.sandbox.state.profile_id)]);
 });await f.sandbox.window.refreshHome();let check=f.get('home-rails').querySelectorAll('.home-watched')[0];
 await check.onclick();assert.equal(cards(f.get('home-rails')).length,1);assert.equal(check.disabled,false);assert.match(f.get('home-status').textContent,/not saved/);
 mode='pending';const saving=check.onclick();f.sandbox.state.profile_id='B';f.sandbox.window.clearHomeContext();await f.sandbox.window.refreshHome();resolve({saved:true});await saving;
 assert.equal(cards(f.get('home-rails'))[0].dataset.homeMediaId,'B');
});

test('season checkbox sends the explicit atomic season scope and captured catalogue revision',async()=>{
 const show={...movie('show',{category:'serie'}),series_key:'show',season:1,episode_counts:{watched:1,total:2}};
 let seen=false;const f=home(command=>{if(command.action==='group_watched'){seen=true;return {saved:true};}return {...payload(seen?[]:[show]),watch_revision:17};});
 await f.sandbox.window.refreshHome();const check=f.get('home-rails').querySelectorAll('.home-watched')[0];assert.match(check.getAttribute('aria-label'),/saison/);await check.onclick();
 const command=f.calls.find(c=>c.action==='group_watched');assert.deepEqual(JSON.parse(JSON.stringify(command)),{action:'group_watched',media_id:'show',watched:true,scope:'season',expected_revision:17});
});
test('unchanged Home refresh keeps card identity and container progress never escapes its profile',async()=>{
 const item={...movie('film'),progress:{position_ms:3025917,duration_ms:9382421}};
 const f=home(()=>payload([item]));await f.sandbox.window.refreshHome();const card=cards(f.get('home-rails'))[0];
 await f.sandbox.window.refreshHome(true);assert.equal(cards(f.get('home-rails'))[0],card);
 assert.equal(f.sandbox.window.homeProgressFor('film').duration_ms,9382421);
 f.sandbox.state.profile_id='B';assert.equal(f.sandbox.window.homeProgressFor('film'),null);
});
test('refilling the twentieth slot keeps remaining card identities instead of flashing their artwork',async()=>{
 const seen=new Set(),items=Array.from({length:21},(_,i)=>movie('m'+i));
 const f=home(command=>{if(command.action==='set_watched'){seen.add(command.media_id);return {saved:true};}return payload(items.filter(i=>!seen.has(i.media.id)).slice(0,20));});
 await f.sandbox.window.refreshHome();const retained=cards(f.get('home-rails'))[1];
 await f.get('home-rails').querySelectorAll('.home-watched')[0].onclick();await new Promise(r=>setImmediate(r));
 const result=cards(f.get('home-rails'));assert.equal(result.length,20);assert.equal(result[0],retained);assert.equal(result.at(-1).dataset.homeMediaId,'m20');
});
test('asynchronous poster loading preserves the real Home progress overlay',async()=>{
 const item={...movie('film',{poster:'artwork'}),progress:{position_ms:3025917,duration_ms:9382421}};
 const f=home(c=>c.action==='home'?payload([item]):{src:'blob:fixture'});await f.sandbox.window.refreshHome();
 const poster=f.get('home-rails').querySelectorAll('.recommendation-poster')[0],bar=poster.querySelectorAll('.home-progress')[0];
 f.emitArtwork([{target:poster,isIntersecting:true}]);await new Promise(r=>setImmediate(r));
 assert.equal(poster.querySelectorAll('img').length,1);assert.equal(poster.querySelectorAll('.home-progress')[0],bar);assert.equal(bar.getAttribute('aria-valuenow'),'32');
});
