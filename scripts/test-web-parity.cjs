// Built web client + its actual bridge/IndexedDB, using a synthetic same-origin
// API. No live server/NAS, native player, provider or deployment is contacted.
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+'/playwright':'playwright');
const assert=require('node:assert/strict'),fs=require('node:fs/promises'),path=require('node:path');
const Query=require('../web/src/catalog-query.js');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHILL_CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run']});
 try{
  const root=path.resolve(__dirname,'../web/dist'),origin='https://chillcenter.test',profile='11111111-1111-4111-8111-111111111111',movie='22222222-2222-4222-8222-222222222222';
  const make=(id,extra)=>({media:{id,title:'Fixture movie',category:'film',year:2026,genres:['Drama'],directors:['Fixture director'],cast:[],path:'fixture.mp4',series:null,season:null,episode:null,series_metadata:null,overview:'Fixture synopsis',watched:false,rating:null,available:true,poster:null,...extra},revision:1});
  const entries=[make(movie,{}),make('33333333-3333-4333-8333-333333333333',{category:'serie',title:'Pilot A',series:'Doctor Who',tmdb_id:2005,season:1,episode:1}),make('44444444-4444-4444-8444-444444444444',{category:'serie',title:'Pilot B',series:'Doctor Who',tmdb_id:2023,season:1,episode:1})];
  let epoch=1;const calls=[],errors=[],image=await fs.readFile(path.resolve(__dirname,'../icons/branding/vector/default.png'));
  const page=await browser.newPage({viewport:{width:1280,height:950}});page.setDefaultTimeout(8000);page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{if(!localStorage.getItem('chill.settings')){localStorage.setItem('chillcenter-interface-language','fr');localStorage.setItem('chill.settings',JSON.stringify({language:'fr',theme:'dark',appearance:{mode:'off',manual:'autumn'},last_profile:{fixture:'11111111-1111-4111-8111-111111111111'}}));}});
  await page.route('**/*',async route=>{
   const req=route.request(),url=new URL(req.url());if(url.origin!==origin)return route.abort();
   const json=(body,status=200)=>route.fulfill({status,contentType:'application/json',body:JSON.stringify(body)});
   if(url.pathname.startsWith('/api/2026-10-14')){
    const p=url.pathname.slice('/api/2026-10-14'.length),method=req.method();calls.push(method+' '+p+url.search);
    if(p==='/server')return json({server_id:'fixture',capabilities:['catalog_pages','playback_leases','media_range'],tls:{}});
    if(p==='/profiles')return json([{id:profile,name:'Test',revision:1}]);
    if(p==='/admin/library')return json({video_root:'/fixture/library',additional_video_roots:[],revision:1});
    if(p.endsWith('/preferences'))return json({audio_language:'en',subtitle_language:'en',autoplay_seconds:20});
    if(p==='/admin/scan')return json({running:false,count:null,last_error:null});
    if(p==='/admin/tmdb')return json({configured:true,credential_type:'v3_api_key',source:'stored',auto_enrich:false});
    if(p.endsWith('/unavailable'))return json({items:[],has_more:false,epoch:1});
    if(p.endsWith('/catalog/revision'))return json({revision:'r'+epoch});
    if(p.endsWith('/catalog/changes')){
     const known=req.postDataJSON().known_revision;if(known==='r'+epoch)return json({revision:known,unchanged:true});
     return json({revision:'r'+epoch,unchanged:false,reset:true,snapshot_id:'55555555-5555-4555-8555-555555555555',total:entries.length,items:entries,deleted_ids:[],sagas:[],playlists:[],series_revisions:{'tmdb:tv:2005':1,'tmdb:tv:2023':1},season_metadata:{},has_more:false,next_offset:null});
    }
    if(p.endsWith('/catalog'))return json(Query.browse(Query.prepare(entries),{offset:Number(url.searchParams.get('offset')||0),limit:48,grouped:url.searchParams.get('grouped')==='true',category:url.searchParams.get('category')||null}));
    if(p.includes('/series/')){const name=decodeURIComponent(p.split('/series/')[1]),series=Query.series(Query.prepare(entries),name,1,url.searchParams.get('selector')||'title');return json(series||{code:'NOT_FOUND'},series?200:404);}
    if(p.endsWith('/tmdb/candidates'))return json({candidates:[{id:123,title:'Matched fixture',year:2026,kind:'movie'}]});
    if(p.startsWith('/admin/profiles/')&&p.endsWith('/tmdb')&&method==='POST'){Object.assign(entries[0].media,{title:'Matched fixture',tmdb_id:123,overview:'Matched synopsis'});entries[0].revision++;epoch++;return json({media:entries[0].media,revision:entries[0].revision});}
    if(p.endsWith('/poster')&&method==='PUT'){entries[0].media.poster='/artwork';entries[0].revision++;epoch++;return json({media:entries[0].media,revision:entries[0].revision});}
    if(p.endsWith('/artwork'))return route.fulfill({contentType:'image/png',body:image});
    if(p.endsWith('/related'))return json({items:[]});
    if(p.includes('/media/'))return json(entries.find(e=>e.media.id===p.split('/').at(-1))||{},entries.some(e=>e.media.id===p.split('/').at(-1))?200:404);
    return json({code:'ROUTE_NOT_FOUND'},404);
   }
   const relative=decodeURIComponent(url.pathname==='/'?'index.html':url.pathname.slice(1)),file=path.resolve(root,relative);if(!file.startsWith(root+path.sep))return route.abort();
   try{const body=await fs.readFile(file),ext=path.extname(file);return route.fulfill({body,contentType:({'.html':'text/html','.js':'text/javascript','.css':'text/css','.json':'application/json','.svg':'image/svg+xml','.png':'image/png','.wasm':'application/wasm','.webmanifest':'application/manifest+json'})[ext]||'application/octet-stream'});}catch{return route.fulfill({status:404,body:''});}
  });
  await page.goto(origin);await page.evaluate(()=>window.chillReady);
  try{await page.waitForFunction(()=>document.querySelectorAll('.media-card').length===3);}catch(error){console.error({boot:await page.evaluate(()=>({stage:state.stage,profile:state.profile_id,category:document.getElementById('category').value,message:document.getElementById('message').textContent,cards:document.querySelectorAll('.media-card').length})),calls,errors});throw error;}
  assert.equal(await page.locator('.search-icon').count(),1);assert.equal(await page.locator('#organize-ai').isVisible(),false);assert.equal(await page.locator('#advice-toggle').isVisible(),false);
  await page.evaluate(()=>window.catalogOpenSeries('tmdb:tv:2005'));await page.waitForFunction(()=>document.querySelectorAll('.series-episodes .media-card').length===1);
  assert.match(await page.locator('.series-episodes').textContent(),/Pilot A/);assert.doesNotMatch(await page.locator('.series-episodes').textContent(),/Pilot B/);
  await page.evaluate(()=>shellShow('catalog'));await page.locator('.media-card h2').filter({hasText:'Fixture movie'}).click();
  await page.locator('#detail-tmdb').getByRole('button',{name:'Rechercher TMDB',exact:true}).click();await page.locator('.tmdb-candidates button').click();
  try{await page.waitForFunction(()=>document.getElementById('detail-title').textContent==='Matched fixture');}catch(error){console.error({detail:await page.locator('#detail-title').textContent(),message:await page.locator('#message').textContent(),association:await page.locator('#media-dialog .tmdb-association-status').textContent(),calls:calls.slice(-16),errors});throw error;}assert.equal(await page.locator('#detail-overview').textContent(),'Matched synopsis');
  const chooser=page.waitForEvent('filechooser');await page.locator('#detail-artwork').click();await(await chooser).setFiles({name:'fixture.png',mimeType:'image/png',buffer:image});
  await page.waitForFunction(()=>Boolean(document.querySelector('#detail-poster img')));
  await page.locator('#media-dialog .dialog-close').click();await page.locator('#admin-toggle').click();
  assert.equal(await page.locator('.configuration-category[data-category="ai"]').count(),0);assert.equal(await page.locator('#ai-settings').isVisible(),false);assert.equal(await page.locator('#archive-admin').isVisible(),true);
  // Select language through the actual persisted bridge, not by patching DOM text.
  await page.locator('#interface-language').selectOption('en');await page.waitForFunction(()=>document.documentElement.lang==='en');assert.equal(await page.locator('#interface-language').inputValue(),'en');
  assert(calls.some(c=>c.includes('/catalog/changes')));assert(calls.some(c=>c.startsWith('POST /admin/profiles/')&&c.endsWith('/tmdb')));assert(calls.some(c=>c.startsWith('PUT ')&&c.includes('/poster?expected_revision=')));
  assert.deepEqual(errors,[]);console.log('PASS built web UI: shared catalogue/icons, separate same-name series, TMDB association, native-browser artwork picker, settings/archive visibility, persisted EN/FR; AI excluded; synthetic same-origin API only.');
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
