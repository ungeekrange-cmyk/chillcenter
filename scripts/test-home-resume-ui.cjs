// Built shared UI + actual web bridge, intercepted same-origin API only.
// Real WAAPI animations/layout; no household API, NAS or playback is contacted.
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+'/playwright':'playwright');
const assert=require('node:assert/strict'),fs=require('node:fs/promises'),path=require('node:path');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHILL_CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run']});
 const root=path.resolve(__dirname,'../web/dist'),origin='https://chillcenter.test',profile='11111111-1111-4111-8111-111111111111';
 const image=await fs.readFile(path.resolve(__dirname,'../icons/branding/vector/default.png'));
 try {
  for(const [width,reducedMotion] of [[1280,'no-preference'],[390,'no-preference'],[1280,'reduce']]) {
   const context=await browser.newContext({viewport:{width,height:950},reducedMotion});const page=await context.newPage();page.setDefaultTimeout(10000);
   const errors=[],writes=[];page.on('pageerror',e=>errors.push(e.message));
   const ids=[2,3,4].map(n=>`${n}${'2'.repeat(7)}-2222-4222-8222-222222222222`);
   const entries=ids.map((id,i)=>({media:{id,title:['Reprise mesurée','Autre film commencé','Série en cours'][i],category:i===2?'serie':'film',series:i===2?'Série en cours':null,season:null,episode:null,year:2026,genres:[],cast:[],directors:[],path:'fixture.mp4',overview:'Synthetic fixture',poster:'/api/2026-10-14/artwork',watched:false,rating:null,available:true,tmdb_id:i===2?12:null},revision:1}));
   let releaseWrite,writeReady,failNext=false,epoch=1;
   await page.addInitScript(()=>{
    localStorage.setItem('chillcenter-interface-language','fr');localStorage.setItem('chill.settings',JSON.stringify({language:'fr',theme:'dark',appearance:{mode:'off'},last_profile:{fixture:'11111111-1111-4111-8111-111111111111'}}));
    window.homeAnimationEvidence=[];const original=Element.prototype.animate;
    Element.prototype.animate=function(frames,options){const result=original.call(this,frames,options);if(this.classList.contains('home-resume-card'))window.homeAnimationEvidence.push({frames,options});return result;};
   });
   await page.route('**/*',async route=>{
    const req=route.request(),url=new URL(req.url());if(url.origin!==origin)return route.abort();
    const json=(body,status=200)=>route.fulfill({status,contentType:'application/json',body:JSON.stringify(body)});
    if(url.pathname.startsWith('/api/2026-10-14')) {
     const p=url.pathname.slice('/api/2026-10-14'.length);
     if(p==='/server')return json({server_id:'fixture',version:'0.7.6',capabilities:['catalog_pages','playback_leases','media_range'],tls:{}});
     if(p==='/profiles')return json([{id:profile,name:'Test',revision:1}]);
     if(p==='/admin/library')return json({video_root:'/fixture',additional_video_roots:[],revision:1});
     if(p.endsWith('/preferences'))return json({audio_language:'en',subtitle_language:'en',autoplay_seconds:20});
     if(p==='/admin/scan')return json({running:false});
     if(p==='/admin/tmdb')return json({configured:false});
     if(p.endsWith('/catalog/revision'))return json({revision:'r'+epoch});
     if(p.endsWith('/catalog/changes'))return json(req.postDataJSON().known_revision==='r'+epoch?{revision:'r'+epoch,unchanged:true}:{revision:'r'+epoch,unchanged:false,reset:true,snapshot_id:'55555555-5555-4555-8555-555555555555',total:entries.length,items:entries,deleted_ids:[],sagas:[],playlists:[],series_revisions:{},season_metadata:{},has_more:false,next_offset:null});
     if(p.endsWith('/playback-progress'))return json({revision:'p1',unchanged:false,items:[]});
     if(p.endsWith('/home'))return json({watch_revision:epoch,continue_watching:entries.filter(e=>!e.media.watched).map((e,i)=>({media:e.media,series_key:e.media.category==='serie'?'tmdb:tv:12':null,season:e.media.category==='serie'?1:null,progress:{position_ms:300000,duration_ms:900000},resume_position_ms:300000,episode_counts:e.media.category==='serie'?{watched:4,total:150}:null,matches:[]})),recently_added:[],recommendations:[]});
     if(p.endsWith('/watched')&&req.method()==='PUT') {
      const body=req.postDataJSON();writes.push(body);
      if(failNext){failNext=false;return json({code:'FIXTURE_FAILURE'},503);}
      await new Promise(resolve=>{releaseWrite=resolve;writeReady?.();writeReady=null;});
      entries.find(e=>e.media.id===p.split('/').at(-2)).media.watched=body.watched;epoch++;
      return json({saved:true});
     }
     if(p.endsWith('/artwork'))return route.fulfill({contentType:'image/png',body:image});
     if(p.endsWith('/unavailable'))return json({items:[],has_more:false,epoch:1});
     return json({code:'ROUTE_NOT_FOUND'},404);
    }
    const file=path.resolve(root,url.pathname==='/'?'index.html':decodeURIComponent(url.pathname.slice(1)));if(!file.startsWith(root+path.sep))return route.abort();
    try {const ext=path.extname(file);return route.fulfill({body:await fs.readFile(file),contentType:({'.html':'text/html','.js':'text/javascript','.css':'text/css','.json':'application/json','.svg':'image/svg+xml','.png':'image/png','.webmanifest':'application/manifest+json'})[ext]||'application/octet-stream'});}catch{return route.fulfill({status:404,body:''});}
   });
   await page.goto(origin);await page.evaluate(()=>window.chillReady);await page.waitForSelector('.home-resume-card');
   const cards=page.locator('.home-resume-card');assert.equal(await cards.count(),3);
   await page.waitForFunction(()=>document.querySelectorAll('.recommendation-poster img').length>=2);
   assert.equal(await page.locator('.home-progress').count(),3,'artwork loading preserves every progress overlay');
   assert.match(await cards.nth(0).textContent(),/33 %/);assert.match(await cards.nth(2).textContent(),/4 \/ 150 épisodes vus · 3 %/);
   assert.equal(await page.locator('#show-in-progress').count(),0);
   if(process.env.CHILL_HOME_QA_DIR)await page.screenshot({path:path.join(process.env.CHILL_HOME_QA_DIR,`home-initial-${width}-${reducedMotion}.png`),fullPage:true});
   const first=await cards.nth(0).boundingBox(),second=page.locator(`.home-resume-card:has([data-home-media-id="${ids[1]}"])`);
   const before=await second.boundingBox();await cards.nth(0).locator('.home-watched').click();
   await page.waitForFunction(()=>document.querySelector('.home-watched').disabled);assert.equal(await cards.count(),3,'pending write keeps its card');
   assert.equal(await cards.nth(0).locator('.home-watched').getAttribute('aria-pressed'),'false');
   assert.equal(writes.length,1);releaseWrite();
   await page.waitForFunction(()=>document.querySelectorAll('.home-resume-card').length===2);
   await page.waitForFunction(()=>[...document.querySelectorAll('.home-resume-card')].every(c=>c.getAnimations().every(a=>a.playState!=='running')));
   const after=await second.boundingBox();
   if(!after || !before || !first) {
    console.error('Home layout diagnostic',width,await page.evaluate(()=>({section:shellSection,homeHidden:document.getElementById('home-panel').hidden,cards:[...document.querySelectorAll('.home-resume-card')].map(c=>({text:c.textContent,rect:c.getBoundingClientRect().toJSON(),display:getComputedStyle(c).display})),errors:document.getElementById('home-status').textContent})));
    if(process.env.CHILL_HOME_QA_DIR)await page.screenshot({path:path.join(process.env.CHILL_HOME_QA_DIR,'home-layout-failure.png'),fullPage:true});
   }
   if(before&&after&&first&&Math.abs(after.x-first.x)>=2)console.error('Home reflow coordinates',JSON.stringify({width,first,before,after,scroll:await page.locator('.recommendation-rail').evaluate(e=>e.scrollLeft)}));
   assert(before&&after&&first);assert(before.x-after.x>120);assert(Math.abs(after.x-first.x)<2);
   const evidence=await page.evaluate(()=>window.homeAnimationEvidence);
   if(reducedMotion==='reduce')assert.equal(evidence.length,0);
   else {assert(evidence.some(e=>e.frames[1]?.opacity===0));assert(evidence.some(e=>e.frames[0]?.transform?.startsWith('translateX(')));}
   failNext=true;await second.locator('.home-watched').click();await page.waitForFunction(()=>document.getElementById('home-status').textContent.length>0);assert.equal(await cards.count(),2);assert.equal(await second.locator('.home-watched').isEnabled(),true);
   const seasonRevision=epoch;const show=page.locator(`.home-resume-card:has([data-home-media-id="${ids[2]}"])`);await show.locator('.home-watched').click();
   await page.waitForFunction(()=>document.querySelector('.home-resume-card:last-child .home-watched').disabled);assert.deepEqual(writes.at(-1),{watched:true,scope:'season',expected_revision:seasonRevision});releaseWrite();
   await page.waitForFunction(()=>document.querySelectorAll('.home-resume-card').length===1);
   if(width===390) {
    await page.waitForFunction(async()=>await window.refreshHome(true));
    for(const e of entries)e.media.watched=false;
    for(let i=0;i<6;i++)entries.push({media:{...entries[1].media,id:`77777777-7777-4777-8777-${String(10+i).padStart(12,'0')}`,title:'Reprise '+i,watched:false},revision:1});
    epoch++;await page.evaluate(()=>window.refreshHome(true));
    await page.waitForFunction(()=>document.querySelectorAll('.home-resume-card').length===9);
    for(const pick of [4,'last']) {
     const count=await cards.count(),index=pick==='last'?count-1:pick;
     const expected=await cards.nth(index<count-1?index+1:index-1).getAttribute('data-home-media-id');
     const control=cards.nth(index).locator('.home-watched');await control.scrollIntoViewIfNeeded();await control.focus();
     assert(await page.locator('.recommendation-rail').evaluate(e=>e.scrollLeft)>0);
     const arrived=new Promise(resolve=>writeReady=resolve);await control.click();await arrived;releaseWrite();
     await page.waitForFunction(n=>document.querySelectorAll('.home-resume-card').length===n,count-1);
     await page.waitForFunction(()=>[...document.querySelectorAll('.home-resume-card')].every(c=>c.getAnimations().every(a=>a.playState!=='running')));
     await page.waitForFunction(async()=>await window.refreshHome(true));
     const focus=await page.evaluate(()=>{const control=document.activeElement,rail=document.querySelector('.recommendation-rail'),c=control.getBoundingClientRect(),r=rail.getBoundingClientRect();return {id:control.closest('.home-resume-card')?.dataset.homeMediaId,visible:c.left>=r.left&&c.right<=r.right};});
     assert.equal(focus.id,expected,'keyboard focus follows the removed card position');assert.equal(focus.visible,true,'focus remains visible in the scrolled rail');
    }
    console.log('PASS scrolled middle/last removal: adjacent visible keyboard focus preserved');
   }
   assert.deepEqual(errors,[]);
   if(process.env.CHILL_HOME_QA_DIR)await page.screenshot({path:path.join(process.env.CHILL_HOME_QA_DIR,`home-${width}-${reducedMotion}.png`),fullPage:true});
   console.log(`PASS Home ${width}px/${reducedMotion}: percentages, episode counts, pending/error safety, fade then real leftward reflow, atomic season command`);
   await context.close();
  }
 } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
