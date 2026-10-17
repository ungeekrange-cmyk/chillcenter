// Exercises the real shared UI against a synthetic bridge. No Pi, NAS or decoder.
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+'/playwright':'playwright');
const assert=require('node:assert/strict'),path=require('node:path');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHILL_CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run']});
 try{
  const page=await browser.newPage({viewport:{width:1180,height:900}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(6000);
  await page.route('https://**/*',r=>r.abort());await page.route('http://**/*',r=>r.abort());
  await page.addInitScript(()=>{
   localStorage.setItem('chillcenter-interface-language','fr');
   const media={id:'11111111-1111-4111-8111-111111111111',title:'Fixture',category:'film',year:2026,genres:[],directors:[],cast:[],overview:'Server description',series:null,season:null,episode:null,available:true,watched:false,rating:null,poster:null,path:'fixture.mp4'};
   const profiles=[{id:'22222222-2222-4222-8222-222222222222',name:'Test',revision:1}];
   window.fixture={revision:1,media,calls:[],syncPending:null,library:{revision:1,video_root:'/fixture/library',additional_video_roots:[]}};
   window.__TAURI__={core:{invoke:async(name,args)=>{
    const c=args.command;fixture.calls.push({...c});
    if(c.action==='boot'||c.action==='refresh')return {stage:'connected',language:'fr',theme:'dark',server_id:'fixture',profile_id:profiles[0].id,profiles,library:{...fixture.library},preferences:{audio_language:'en',subtitle_language:'en',autoplay_seconds:20}};
    if(c.action==='catalog')return {items:[media],has_more:false,series_counters:{}};
    if(c.action==='sync_catalog'){if(fixture.holdSync)return new Promise(resolve=>fixture.syncPending=resolve);return {changed:true};}
    if(c.action==='resolve_appearance')return {ambiance:null,appearance:{mode:'off',manual:'autumn'}};
    if(c.action==='media_detail')return {media:{...media},revision:fixture.revision};
    if(c.action==='related_media')return {items:[]};
    if(c.action==='scan_status')return {running:false,count:null,last_error:null};
    if(c.action==='unavailable')return {items:[],has_more:false,epoch:1};
    if(c.action==='tmdb_status')return {configured:false};
    if(c.action==='save_library'){if(c.expected_revision!==fixture.library.revision)throw {status:409,code:'CONFIG_CONFLICT',message:'Modification concurrente'};return {...state,library:{...fixture.library}};}
    if(c.action==='edit_media'){
     if(c.expected_revision!==fixture.revision)throw {status:409,code:'PROFILE_CONFLICT',message:'Modification concurrente : rechargez les données avant de réessayer.'};
     Object.assign(media,c.patch);fixture.revision++;return {media,revision:fixture.revision};
    }
    throw Error('Unexpected fixture action '+c.action);
   }}};
  });
  await page.goto('file://'+path.resolve(__dirname,'../ui/connected/index.html'));await page.evaluate(()=>window.chillReady);
  const search=page.locator('.global-search .search-icon');assert.equal(await search.count(),1,'Search must use a sized SVG, not a font-dependent glyph');
  for(const theme of ['light','dark'])for(const width of [760,1180,1600]){
   await page.setViewportSize({width,height:900});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
   const box=await search.boundingBox(),input=await page.locator('#search').boundingBox();assert(box.width>=22&&box.height>=22);assert(Math.abs(box.y+box.height/2-input.y-input.height/2)<1);
  }
  await page.setViewportSize({width:1180,height:900});
  await page.evaluate(()=>{fixture.holdSync=true;void syncCatalog();});
  const spinner=page.locator('#sync-indicator'),glyph=spinner.locator('svg'),frame=await spinner.boundingBox();
  for(const time of [0,125,250,500,750]){
   await glyph.evaluate((el,time)=>{const a=el.getAnimations()[0];a.pause();a.currentTime=time;},time);
   const rect=await glyph.boundingBox(),now=await spinner.boundingBox();
   assert(Math.abs(rect.x+rect.width/2-frame.x-frame.width/2)<0.6);assert(Math.abs(rect.y+rect.height/2-frame.y-frame.height/2)<0.6);assert.deepEqual(now,frame);
  }
  assert.equal(await page.locator('#search').isEnabled(),true);
  await page.emulateMedia({reducedMotion:'reduce'});assert.equal(await glyph.evaluate(el=>getComputedStyle(el).animationName),'none');await page.emulateMedia({reducedMotion:'no-preference'});
  await page.evaluate(()=>{fixture.holdSync=false;fixture.syncPending({changed:false});});
  await page.locator('.media-card h2').click();await page.locator('#detail-edit').click();await page.locator('#edit-title').fill('My unsaved title');
  await page.evaluate(()=>{fixture.revision=2;fixture.media.overview='New server description';});
  await page.locator('#metadata-form button').click();
  const recovery=page.locator('#media-dialog .conflict-recovery');await recovery.waitFor();
  assert.equal(await page.locator('#edit-title').inputValue(),'My unsaved title');
  assert.equal(await page.locator('#search').isEnabled(),true,'Recovery must not globally disable navigation');
  await recovery.getByRole('button',{name:'Actualiser cette fiche'}).click();
  await page.waitForFunction(()=>!document.querySelector('.conflict-recovery'));
  assert.equal(await page.locator('#edit-title').inputValue(),'My unsaved title','Reload must preserve the edit draft');
  assert.equal(await page.locator('#metadata-form').isVisible(),true);
  assert.equal(await page.locator('#detail-overview').textContent(),'New server description');
  assert.equal(await page.evaluate(()=>fixture.calls.filter(c=>c.action==='edit_media').length),1,'No implicit mutation replay');
  await page.locator('#metadata-form button').click();
  await page.waitForFunction(()=>fixture.calls.filter(c=>c.action==='edit_media').length===2);
  assert.equal(await page.evaluate(()=>fixture.calls.filter(c=>c.action==='edit_media').at(-1).expected_revision),2);
  await page.waitForFunction(()=>document.getElementById('metadata-form').hidden);
  await page.locator('#detail-edit').click();await page.locator('#edit-title').fill('Another draft');await page.evaluate(()=>fixture.revision++);
  await page.locator('#metadata-form button').click();await page.locator('#media-dialog .conflict-recovery').waitFor();
  await page.evaluate(()=>fixture.holdSync=true);await page.locator('#media-dialog .conflict-recovery button').first().click();
  await page.locator('#media-dialog .dialog-close').click();await page.waitForFunction(()=>!document.querySelector('.conflict-recovery'));assert.equal(await page.locator('.conflict-recovery').count(),0,'Closing removes stale feedback before the held request completes');
  await page.evaluate(()=>{fixture.holdSync=false;fixture.syncPending({changed:false});});
  await page.locator('#admin-toggle').click();await page.locator('#video-root').fill('/fixture/draft');
  await page.locator('#library-add-location').click();await page.locator('#library-add-location').click();
  await page.locator('#additional-library-locations input').nth(0).fill('/fixture/extra-one');await page.locator('#additional-library-locations input').nth(1).fill('/fixture/extra-two');
  await page.evaluate(()=>fixture.library={revision:2,video_root:'/fixture/server-changed',additional_video_roots:['/fixture/server-extra']});
  await page.locator('#library-form button[type="submit"],#library-form button:not([type])').click();
  await page.locator('#admin-panel .conflict-recovery').getByRole('button',{name:'Actualiser les réglages de bibliothèque'}).click();
  await page.waitForFunction(()=>!document.querySelector('.conflict-recovery'));
  assert.equal(await page.locator('#video-root').inputValue(),'/fixture/draft');assert.deepEqual(await page.locator('#additional-library-locations input').evaluateAll(nodes=>nodes.map(n=>n.value)),['/fixture/extra-one','/fixture/extra-two']);
  assert.equal(await page.evaluate(()=>state.library.revision),2);assert.equal(await page.evaluate(()=>fixture.calls.filter(c=>c.action==='save_library').length),1);
  assert.deepEqual(errors,[]);
  console.log('PASS: real shared UI search sizing, stable SVG rotation/reduced motion, nonblocking conflict recovery, media/library drafts, close during held recovery, and explicit retry on latest revision.');
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
