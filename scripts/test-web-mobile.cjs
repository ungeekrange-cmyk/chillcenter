// Real built web UI with an intercepted same-origin fixture API. No live Pi,
// NAS or video decoder is used; fullscreen/orientation policy is mocked only
// in the explicitly labelled presentation test at the end.
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES ? process.env.CHILL_PREVIEW_NODE_MODULES+'/playwright' : 'playwright');
const assert=require('node:assert/strict'),fs=require('node:fs/promises'),path=require('node:path');
const Query=require('../web/src/catalog-query.js');
const origin='https://chillcenter.test',profile='11111111-1111-4111-8111-111111111111';
const entries=Array.from({length:180},(_,i)=>({media:{
 id:`22222222-2222-4222-8222-${String(i).padStart(12,'0')}`,title:`Movie ${String(i).padStart(3,'0')}`,
 category:i===179?'documentaire':i===178?'spectacle':'film',year:2020+i%7,genres:[i%2?'Drama':'Comedy'],
 directors:[],cast:[],path:`fixture/movie-${i}.mp4`,series:null,season:null,episode:null,series_metadata:null,
 overview:'Synthetic mobile layout synopsis.',watched:false,rating:null,available:true,poster:null,
},revision:1}));
const media=entries[0].media.id;
for(const [index,episode] of [[176,1],[177,2]])Object.assign(entries[index].media,{category:'serie',title:'Episode '+episode,series:'Fixture series',tmdb_id:123,season:1,episode,series_metadata:{name:'Fixture series',overview:'Series synopsis',genres:['Drama'],cast:[],directors:[],year:2026}});
const saga={id:'fixture-saga',name:'Fixture saga',revision:1,media_ids:entries.slice(0,3).map(e=>e.media.id),poster:null};
const playlist={id:'77777777-7777-4777-8777-777777777777',name:'Fixture playlist',revision:1,media_ids:entries.slice(0,3).map(e=>e.media.id),poster:null};

async function routeFixture(page,root){
 await page.addInitScript(()=>{
  if(!localStorage.getItem('chill.settings')){
   localStorage.setItem('chillcenter-interface-language','fr');
   localStorage.setItem('chill.settings',JSON.stringify({language:'fr',theme:'dark',appearance:{mode:'off',manual:'autumn'},last_profile:{fixture:'11111111-1111-4111-8111-111111111111'}}));
  }
 });
 await page.route('**/*',async route=>{
  const req=route.request(),url=new URL(req.url());if(url.origin!==origin)return route.abort();
  const json=(body,status=200)=>route.fulfill({status,contentType:'application/json',body:JSON.stringify(body)});
  if(url.pathname.startsWith('/api/2026-10-14')){
   const p=url.pathname.slice('/api/2026-10-14'.length);
   if(p==='/server')return json({server_id:'fixture',capabilities:['catalog_pages','playback_leases','media_range'],tls:{}});
   if(p==='/profiles')return json([{id:profile,name:'Camille',revision:1}]);
   if(p==='/admin/library')return json({video_root:'/fixture/library',additional_video_roots:[],revision:1});
   if(p.endsWith('/preferences'))return json({audio_language:'en',subtitle_language:'en',autoplay_seconds:20});
   if(p==='/admin/scan')return json({running:false,count:null,last_error:null});
   if(p==='/admin/tmdb')return json({configured:true,auto_enrich:false});
   if(p.endsWith('/unavailable'))return json({items:[],has_more:false,epoch:1});
   if(p.endsWith('/catalog/revision'))return json({revision:'r1'});
   if(p.endsWith('/catalog/changes'))return json(req.postDataJSON().known_revision==='r1'?{revision:'r1',unchanged:true}:{revision:'r1',unchanged:false,reset:true,snapshot_id:'55555555-5555-4555-8555-555555555555',total:entries.length,items:entries.slice(0,100),deleted_ids:[],sagas:[saga],playlists:[playlist],series_revisions:{'tmdb:tv:123':1},season_metadata:{},has_more:true,next_offset:100});
   if(p.includes('/catalog/changes/'))return json({revision:'r1',items:entries.slice(Number(url.searchParams.get('offset'))),has_more:false,next_offset:null});
   if(p.endsWith('/catalog')){const query=Object.fromEntries(url.searchParams);return json(Query.browse(Query.prepare(entries),{offset:Number(query.offset||0),limit:48,grouped:true,category:query.category||null,q:query.q||null,genre:query.genre||null,year:query.year==null?null:Number(query.year),watched:query.watched==null?null:query.watched==='true',available:query.available==null?null:query.available==='true',sort:query.sort||null}));}
   if(p.includes('/series/'))return json(Query.series(Query.prepare(entries),decodeURIComponent(p.split('/series/')[1]),1,url.searchParams.get('selector')||'title'));
   if(p.endsWith('/related'))return json({items:[]});
   if(p.endsWith('/sagas'))return json({items:[saga]});
   if(p.endsWith('/playlists'))return json({items:[playlist]});
   if(p.endsWith('/sagas/'+saga.id))return json({saga});
   if(p.endsWith('/playlists/'+playlist.id))return json({playlist});
   if(p.endsWith('/artwork'))return json({code:'NOT_FOUND'},404);
   if(p.includes('/media/')){const entry=entries.find(e=>e.media.id===p.split('/').at(-1));return json(entry||{code:'NOT_FOUND'},entry?200:404);}
   return json({code:'ROUTE_NOT_FOUND'},404);
  }
  const file=path.resolve(root,url.pathname==='/'?'index.html':decodeURIComponent(url.pathname.slice(1)));
  if(!file.startsWith(root+path.sep))return route.abort();
  try{return route.fulfill({body:await fs.readFile(file),contentType:({'.html':'text/html','.js':'text/javascript','.css':'text/css','.json':'application/json','.svg':'image/svg+xml','.png':'image/png','.wasm':'application/wasm','.webmanifest':'application/manifest+json'})[path.extname(file)]||'application/octet-stream'});}catch{return route.fulfill({status:404,body:''});}
 });
}
async function noOverflow(page,label){
 const size=await page.evaluate(()=>({body:document.documentElement.scrollWidth,viewport:innerWidth}));
 assert(size.body<=size.viewport+1,`${label}: horizontal overflow ${JSON.stringify(size)}`);
}

(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHILL_CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run']});
 const errors=[];
 try{
  const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,deviceScaleFactor:1});
  const page=await context.newPage();page.setDefaultTimeout(10000);page.on('pageerror',e=>errors.push(e.message));
  await routeFixture(page,path.resolve(__dirname,'../web/dist'));
  await page.goto(origin);await page.evaluate(()=>window.chillReady);
  await page.waitForSelector('.media-card');await page.waitForFunction(()=>!document.getElementById('mobile-nav').hidden);
  assert((await page.locator('meta[name=viewport]').getAttribute('content')).includes('viewport-fit=cover'));
  assert.equal(await page.locator('#mobile-nav button').count(),5);
  assert.equal(await page.locator('.sidebar').isVisible(),false);
  assert.equal(await page.locator('#profile-name').isVisible(),true);
  assert.equal(await page.locator('.global-search').isVisible(),false);
  assert.equal(await page.locator('#filter-form').isVisible(),false);
  assert((await page.locator('#media').boundingBox()).y<300,'catalogue is visible above the fold without search');
  for(const width of [320,390,430,736,844]){
   await page.setViewportSize({width,height:width>=736?390:844});
   await page.waitForFunction(()=>getComputedStyle(document.getElementById('mobile-nav')).position==='fixed');
   await page.evaluate(()=>window.restoreCatalogLayout?.());
   try{await page.waitForFunction(w=>{const grid=document.querySelector('#media .virtual-window');return grid&&getComputedStyle(grid).gridTemplateColumns.split(' ').length===(w<=360?2:3);},width);}
   catch(error){error.message+=JSON.stringify(await page.evaluate(()=>{const grid=document.querySelector('#media .virtual-window');return {width:innerWidth,height:innerHeight,mediaWidth:document.getElementById('media').getBoundingClientRect().width,grid:grid?.style.cssText,computed:grid&&getComputedStyle(grid).gridTemplateColumns};}));throw error;}
   await noOverflow(page,`catalogue ${width}`);
   // Query in the same JS turn: a Locator's earlier element handle may be an
   // intentionally retired virtual window after the resize RAF replaces it.
   const columns=await page.evaluate(()=>{const el=document.querySelector('#media .virtual-window');return {count:getComputedStyle(el).gridTemplateColumns.split(' ').length,inline:el.style.gridTemplateColumns,display:getComputedStyle(el).display,hidden:document.getElementById('catalog-panel').hidden,stage:state.stage};});
   assert.equal(columns.count,width<=360?2:3,`poster density at ${width}: ${JSON.stringify(columns)}`);
   const nav=await page.locator('#mobile-nav').boundingBox();
   if(width>=736){assert.equal(nav.x,0);assert(nav.width<=76&&nav.y===0&&nav.height>=389,'landscape navigation is a left rail');}
   else assert(Math.abs(nav.y+nav.height-844)<=1);
  }
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.locator('#media .media-card.catalog-open .poster-actions:visible').count(),0,'catalogue poster actions stay off the mobile posters');
  await page.waitForFunction(()=>typeof window.catalogOpenMedia==='function');
  await page.evaluate(()=>scrollTo(0,0));await page.waitForTimeout(80);
  // Choose an already visible card: scrolling an offscreen virtual row would
  // deliberately replace that node before the click is delivered.
  const firstVisibleId=await page.evaluate(()=>[...document.querySelectorAll('#media .media-card.catalog-open')].find(el=>{const r=el.getBoundingClientRect();return !el.querySelector('.series-counts')&&r.top>80&&r.bottom<760;})?.dataset.mediaId);
  assert(firstVisibleId,'a catalogue card is visible for its ordinary detail click');
  await page.locator(`#media [data-media-id="${firstVisibleId}"] h2`).click();
  await page.waitForSelector('#media-dialog[open]');await page.locator('#media-dialog .dialog-close').click();
  const profileBox=await page.locator('#profile-menu').boundingBox();assert(profileBox&&profileBox.x+profileBox.width>=370,'profile stays in the upper right');
  await page.locator('#mobile-search-toggle').click();assert.equal(await page.locator('.global-search').isVisible(),true);
  // Select a category before the shared input debounce fires.  The category
  // request and the delayed search refresh must both retain this query.
  await page.locator('#search').fill('Movie 000');
  await page.locator('[data-mobile-category=film]').click();
  await page.waitForTimeout(320);
  for(const [selector,category,expected] of [['[data-mobile-category=film]','film',1],['[data-mobile-category=serie]','serie',0],['[data-mobile-category=documentaire]','documentaire',0],['#mobile-brand','',1]]){
   await page.locator(selector).click();
   await page.waitForFunction(([category,count])=>document.getElementById('category').value===category&&document.getElementById('search').value==='Movie 000'&&document.querySelectorAll('#media .media-card').length===count,[category,expected]);
  }
  await page.locator('#mobile-search-toggle').click();await page.locator('#search').fill('');await page.waitForFunction(()=>document.getElementById('search').value===''&&document.querySelectorAll('#media .media-card').length>1);
  await page.locator('[data-mobile-category=film]').click();await page.waitForFunction(()=>document.getElementById('category').value==='film');
  await page.locator('#mobile-search-toggle').click();assert.equal(await page.locator('.global-search').isVisible(),true);
  await page.locator('#facet-genres').click();await noOverflow(page,'expanded genres');
  await page.locator('.facet-chip').filter({hasText:'Drama'}).click();
  await page.waitForFunction(()=>window.catalogFacets.values().genres.includes('Drama'));
  await page.locator('#facet-years').click();await noOverflow(page,'expanded years');
  const years=await page.locator('.facet-year').allTextContents();assert.equal(Number(years[0]),2026);
  await page.locator('.facet-year').filter({hasText:'2020'}).click();await page.locator('.facet-year').filter({hasText:'2024'}).click();
  assert.equal(await page.locator('.facet-year[aria-pressed=true]').count(),2);
  assert.deepEqual(await page.evaluate(()=>window.catalogFacets.values().years),[2020,2024]);
  await page.screenshot({path:'/private/tmp/chillcenter-mobile-web-built-dark.png'});
  // Return to the unfiltered full collection through the approved Plus sheet.
  await page.locator('[data-mobile-more]').click();await page.locator('[data-mobile-all]').click();
  await page.evaluate(()=>{window.catalogFacets?.clear();document.getElementById('genre').value='';document.getElementById('year').value='';return loadPage();});
  await page.evaluate(()=>scrollTo(0,1800));await page.waitForTimeout(120);
  const before=await page.evaluate(()=>scrollY);
  // The virtual window deliberately retains overscan rows above the viewport.
  // Click a genuinely visible title, not the first offscreen overscan card
  // (Playwright would scroll to that card before opening the dialog).
  const visibleId=await page.evaluate(()=>[...document.querySelectorAll('#media .media-card')].find(el=>{const r=el.querySelector('h2').getBoundingClientRect();return r.top>200&&r.bottom<650;})?.dataset.mediaId);
  assert(visibleId,'a title is visible at the retained catalogue offset');
  await page.locator(`#media [data-media-id="${visibleId}"] h2`).click();await page.waitForSelector('#media-dialog[open]');
  await noOverflow(page,'detail');
  await page.locator('#media-dialog .dialog-close').click();
  await page.waitForFunction(y=>Math.abs(scrollY-y)<5,before);
  await page.evaluate(()=>scrollTo(0,document.documentElement.scrollHeight));await page.waitForTimeout(150);
  const last=page.locator('#media .media-card').last();
  const lastBox=await last.boundingBox(),navBox=await page.locator('#mobile-nav').boundingBox();
  assert(lastBox&&lastBox.y+lastBox.height<=navBox.y+1,'last virtual row remains above navigation');
  await page.locator('[data-mobile-category=serie]').click();
  assert.equal(await page.locator('.global-search').isVisible(),false);
  await page.evaluate(()=>{window.catalogFacets.clear();return loadPage();});
  await page.locator('#media .media-card h2').first().click();await page.waitForSelector('.series-episodes .media-card');
  await page.waitForSelector('.mobile-season-details');
  assert.equal(await page.locator('.mobile-season-details').first().getAttribute('open'),null);
  assert.equal(await page.locator('.mobile-season-details .season-provider').first().isVisible(),false);
  assert.equal(await page.locator('.mobile-series-details').getAttribute('open'),null);
  assert.equal(await page.evaluate(()=>{const season=document.querySelector('.series-season');return [...season.children].indexOf(season.querySelector('.series-episodes'))<[...season.children].indexOf(season.querySelector('.mobile-season-details'));}),true);
  await page.locator('.mobile-season-details > summary').first().click();
  assert.equal(await page.locator('.mobile-season-details .season-provider').first().isVisible(),true);
  await page.setViewportSize({width:1280,height:950});await page.waitForFunction(()=>!document.querySelector('.mobile-season-details'));
  assert.equal(await page.locator('.season-provider').first().isVisible(),true);
  assert.equal(await page.locator('#mobile-search-toggle').isVisible(),false);
  await page.setViewportSize({width:390,height:844});await page.waitForSelector('.mobile-season-details');
  await page.locator('#mobile-search-toggle').click();assert.equal(await page.locator('.global-search').isVisible(),true);
  await page.locator('[data-mobile-more]').click();await page.locator('[data-mobile-sagas]').click();
  assert.equal(await page.locator('.global-search').isVisible(),false);
  try{await page.waitForFunction(()=>!document.getElementById('collections-panel').hidden);}catch(error){console.error('sagas diagnostics',await page.evaluate(()=>({section:shellSection,message:document.getElementById('message').textContent,disabled:document.getElementById('show-sagas').disabled,handler:String(document.getElementById('show-sagas').onclick),sheet:document.getElementById('mobile-more-sheet').open})),errors);throw error;}await noOverflow(page,'sagas');
  await page.locator('[data-collection-id="fixture-saga"]').click();await page.waitForSelector('#collection-members .media-card');
  await noOverflow(page,'saga members');
  const collectionActions=await page.locator('#collection-members .actions').first().evaluate(el=>({opacity:getComputedStyle(el).opacity,buttons:[...el.querySelectorAll('button')].map(b=>({width:b.getBoundingClientRect().width,height:b.getBoundingClientRect().height}))}));
  assert.equal(collectionActions.opacity,'1');assert(collectionActions.buttons.every(b=>b.width>=44&&b.height>=44),'collection controls remain touch-sized without hover');
  await page.locator('[data-mobile-more]').click();await page.locator('[data-mobile-playlists]').click();
  await page.waitForFunction(()=>!document.getElementById('collections-panel').hidden);await noOverflow(page,'playlists');
  await page.locator('[data-mobile-more]').click();await page.locator('[data-mobile-config]').click();
  await page.waitForFunction(()=>!document.getElementById('admin-panel').hidden);await noOverflow(page,'configuration');
  assert.equal(await page.locator('#ai-settings').isVisible(),false);
  await page.locator('#interface-language').selectOption('en');await page.waitForFunction(()=>document.documentElement.lang==='en');
  assert.match(await page.locator('[data-mobile-category=serie]').textContent(),/TV shows|Series/);
  assert.match(await page.locator('[data-mobile-more]').textContent(),/More/);
  for(const theme of ['light','dark']){
   await page.evaluate(value=>document.documentElement.dataset.theme=value,theme);await noOverflow(page,`${theme} settings`);
  }
  await page.setViewportSize({width:1280,height:950});await page.waitForTimeout(100);
  assert.equal(await page.locator('.sidebar').isVisible(),true);assert.equal(await page.locator('#mobile-nav').isVisible(),false);assert.equal(await page.locator('#mobile-brand').isVisible(),false);
  await page.setViewportSize({width:390,height:844});
  await page.evaluate(()=>{
   const panel=document.getElementById('player-panel');panel.hidden=false;document.body.classList.add('player-open');
   document.documentElement.dataset.ambiance='halloween';
   const button=document.createElement('button');button.id='test-mobile-fullscreen';button.textContent='Fullscreen presentation test';
   button.style.cssText='position:fixed;left:60px;top:180px;z-index:2000;max-width:250px';
   button.onclick=()=>ChillMobilePlayback.requestFromGesture();panel.append(button);
  });
  await page.locator('#test-mobile-fullscreen').click();
  await page.waitForFunction(()=>document.fullscreenElement===document.getElementById('player-panel'));
  assert.equal(await page.locator('#mobile-nav').isVisible(),false);
  assert.equal(await page.locator('.app-topbar').isVisible(),false);
  assert.equal(await page.evaluate(()=>getComputedStyle(document.querySelector('.workspace'),'::before').content),'none');
  await page.setViewportSize({width:844,height:390});
  await page.waitForFunction(()=>document.getElementById('mobile-nav').hidden);
  // Inject OS-inset values, not a fake claim of physical cutout testing.
  for(const [left,right] of [[44,0],[0,44]]){
   await page.evaluate(([left,right])=>{document.documentElement.style.setProperty('--mobile-safe-left',`${left}px`);document.documentElement.style.setProperty('--mobile-safe-right',`${right}px`);document.documentElement.style.setProperty('--mobile-safe-bottom','21px');},[left,right]);
   const safe=await page.evaluate(()=>{const r=document.getElementById('native-video').getBoundingClientRect(),h=getComputedStyle(document.querySelector('.player-heading')),o=getComputedStyle(document.getElementById('player-overlay'));return {x:r.x,right:r.right,bottom:r.bottom,headingLeft:parseFloat(h.paddingLeft),headingRight:parseFloat(h.paddingRight),overlayLeft:parseFloat(o.paddingLeft),overlayRight:parseFloat(o.paddingRight)};});
   assert(safe.x>=left&&safe.right<=844-right&&safe.bottom<=369,`video avoids synthetic camera/home insets ${JSON.stringify(safe)}`);
   assert(safe.headingLeft>=left&&safe.headingRight>=right&&safe.overlayLeft>=left&&safe.overlayRight>=right,'player controls avoid left and right camera cutouts');
  }
  await page.evaluate(()=>{for(const edge of ['left','right','bottom'])document.documentElement.style.removeProperty(`--mobile-safe-${edge}`);});
  await page.setViewportSize({width:390,height:844});
  await page.evaluate(async()=>{if(document.fullscreenElement)await document.exitFullscreen();document.getElementById('test-mobile-fullscreen').remove();document.getElementById('player-panel').hidden=true;document.body.classList.remove('player-open');});
  await page.waitForFunction(()=>!document.getElementById('mobile-nav').hidden);
  for(const event of ['halloween','christmas']){
   await page.evaluate(value=>document.documentElement.dataset.ambiance=value,event);
   assert.equal(await page.evaluate(()=>getComputedStyle(document.querySelector('.workspace'),'::before').left),'0px');
  }
  // Mock only the browser policy to prove synchronous gesture ordering,
  // real-state reporting and fallback cleanup without fetching any video.
  const presentation=await page.evaluate(async()=>{
   const panel=document.getElementById('player-panel'),trace=[];
   panel.hidden=false;document.body.classList.add('player-open');
   const hint=document.getElementById('mobile-orientation-hint');
   let actual=false;const presented={hidden:false,requestFullscreen(){trace.push('request');actual=true;return Promise.resolve();}};
   const ctl=ChillMobilePlaybackCore.createController({window:{matchMedia:()=>({matches:true}),screen:{orientation:{lock:async mode=>trace.push(mode),unlock:()=>trace.push('unlock')}}},document:{get fullscreenElement(){return actual?presented:null;},exitFullscreen:async()=>{actual=false;trace.push('exit');}},panel:()=>presented,hint:()=>hint});
   const promise=ctl.requestFromGesture();trace.push('after-call');await promise;
   const immediate=trace.slice(0,2);ctl.stop();
   const denied=ChillMobilePlaybackCore.createController({window:{matchMedia:()=>({matches:true}),screen:{}},document:{},panel:()=>({requestFullscreen:()=>Promise.reject(new Error('denied'))}),hint:()=>hint});
   await denied.requestFromGesture();const fallback=!hint.hidden;
   await new Promise(resolve=>setTimeout(resolve,ChillMobilePlaybackCore.HINT_MS+50));
   denied.refreshMedia();denied.refreshViewport();denied.refreshLanguage();
   const expired=hint.hidden;denied.stop();
   panel.hidden=true;document.body.classList.remove('player-open');return {immediate,fallback,expired,cleaned:hint.hidden};
  });
  assert.deepEqual(presentation.immediate,['request','after-call']);assert(presentation.fallback&&presentation.expired&&presentation.cleaned);
  assert.deepEqual(errors,[]);
  console.log('PASS built mobile web: 320/390/430/736/844 layouts, 1280 desktop restore, fixed navigation/Plus, profile/touch targets, virtual rows and preserved detail scroll, live genre/year filters, nonempty collections/configuration, EN/FR, themes, AI excluded; real Chromium fullscreen entry/exit, orientation denial mocked, no physical-device or live video claim.');
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
