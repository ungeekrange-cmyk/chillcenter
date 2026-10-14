// Isolated DOM/layout proof, synthetic bridge only. Not a native playback proof.
const {chromium}=require('playwright'),assert=require('node:assert/strict'),path=require('node:path');
(async()=>{
 const browser=await chromium.launch({headless:true});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('SYNTHETIC_PAGE_ERROR',e.message);});
  await page.addInitScript(()=>{
   const movies=Array.from({length:103},(_,i)=>({id:'video-'+i,title:(i%4===0?'Documentaire ':i%4===1?'Film ':i%4===2?'Série ':'Spectacle ')+i,category:['documentaire','film','serie','spectacle'][i%4],year:2026,available:true,watched:false,poster:'synthetic',genres:[],directors:[],cast:[],season:null,episode:null}));
   const saga={id:'saga',name:'Voyages imaginaires',revision:1,media_ids:['video-1','video-5'],poster:'synthetic'},playlist={id:'playlist',name:'Pour ce soir',revision:1,media_ids:Array.from({length:12},(_,i)=>'video-'+i)};window.syntheticPlaylist=playlist;
   const view={stage:'connected',server_id:'test-server',address:'https://synthetic.invalid',profile_id:'test-profile',profiles:[{id:'test-profile',name:'Profil de démonstration',revision:1}],library:{revision:1,video_root:null},theme:'dark',playback_cache:{forward_mib:128,back_mib:64},player_presentation:{fullscreen_on_start:true}};
   window.testCalls=[];
   window.__TAURI__={core:{invoke:async(name,{command:c})=>{
    window.testCalls.push(c);
    if(['boot','refresh'].includes(c.action))return structuredClone(view);
    if(c.action==='resolve_appearance')return {ambiance:null,appearance:{mode:'off',theme:'dark'}};
    if(c.action==='sagas')return {items:[saga]};if(c.action==='playlists')return {items:[playlist]};
    if(c.action==='saga')return {saga:structuredClone(saga)};if(c.action==='playlist')return {playlist:structuredClone(playlist)};
    if(c.action==='media_detail')return {media:movies.find(m=>m.id===c.media_id),revision:1};
    if(c.action==='catalog'){const items=movies.filter(m=>(!c.category||m.category===c.category)&&(!c.q||m.title.toLowerCase().includes(c.q.toLowerCase()))&&(c.watched==null||m.watched===c.watched));return {items:items.slice(c.offset,c.offset+48),has_more:c.offset+48<items.length};}
    if(c.action==='artwork'||c.action==='saga_artwork'||c.action==='playlist_artwork'){const svg='<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300"><rect width="200" height="300" fill="#8d493b"/><circle cx="100" cy="120" r="62" fill="#d8c995"/><path d="M20 240Q100 150 180 240V300H20" fill="#2b473b"/></svg>';return {src:'data:image/svg+xml;base64,'+btoa(svg)};}
    if(c.action==='add_playlist'||c.action==='add_saga'){const target=c.action==='add_saga'?saga:playlist;target.media_ids.push(...c.media_ids);target.revision++;return {ok:true};}
    if(c.action==='save_player_settings'){view.player_presentation.fullscreen_on_start=c.fullscreen_on_start;return {player_presentation:structuredClone(view.player_presentation)};}
    if(c.action==='tmdb_status')return {configured:false,auto_enrich:false};if(c.action==='scan_status')return {running:false,count:null};if(c.action==='unavailable')return {items:[],has_more:false,epoch:1};
    if(c.action==='stop')return {stage:'closed'};throw Error('Unmocked synthetic command '+c.action);
   }}};
  });
  await page.goto('file://'+path.resolve(__dirname,'../ui/connected/index.html'));await page.evaluate(()=>window.chillReady);await page.evaluate(()=>loadPage());
  assert.equal(await page.locator('#profile-name').textContent(),'Profil de démonstration');assert.equal(await page.locator('#profile-avatar').textContent(),'P');await page.locator('#profile-menu summary').click();assert.equal(await page.locator('#create-profile-menu').isVisible(),true);await page.locator('#profile-menu summary').click();
  await page.locator('#show-sagas').click();await page.locator('#collections-panel').waitFor({state:'visible'});
  assert.equal(await page.locator('dialog[open]').count(),0);assert.equal(await page.locator('#collection-create').isVisible(),false);
  await page.locator('#collections-list .poster-tile').click();await page.locator('#collection-panel').waitFor({state:'visible'});
  await page.waitForFunction(()=>document.querySelectorAll('#collection-members .collection-media-card').length===2);
  await page.waitForFunction(()=>document.querySelectorAll('#collection-members .poster img').length===2,{},{timeout:2500});
  assert.equal(await page.locator('#collection-panel').evaluate(e=>e.tagName),'SECTION');
  if(process.env.CHILL_UI_SCREENSHOTS)await page.screenshot({path:path.join(process.env.CHILL_UI_SCREENSHOTS,'saga-page.png')});
  await page.locator('#collection-back').click();await page.locator('#collection-create-toggle').click();assert.equal(await page.locator('#collection-create').isVisible(),true);await page.locator('#collection-create-cancel').click();
  await page.locator('#show-playlists').click();await page.locator('#collections-list .poster-tile').waitFor();assert.equal(await page.locator('#collections-list .collection-mosaic-cell').count(),9,'a default list cover composes at most nine posters');
  await page.evaluate(()=>{window.syntheticPlaylist.poster='synthetic';});await page.locator('#show-playlists').click();await page.waitForFunction(()=>document.querySelector('#collections-list .poster img'));assert.ok((await page.evaluate(()=>window.testCalls.some(c=>c.action==='playlist_artwork'))),'custom list artwork must replace the mosaic');await page.locator('#collections-list .poster-tile').click();await page.locator('#collection-add').click();await page.locator('#picker-panel').waitFor({state:'visible'});
  await page.locator('#picker-category').selectOption('documentaire');await page.waitForFunction(()=>pickerItems.length===25);
  assert.ok(await page.locator('#picker-items .poster-tile .poster').count()>0);assert.ok(await page.locator('#picker-items .poster-tile h2').first().textContent());
  const first=page.locator('#picker-items .poster-tile').first();await first.click();assert.equal(await first.getAttribute('aria-pressed'),'true');
  await page.locator('#picker-category').selectOption('serie');await page.waitForFunction(()=>pickerItems.length>0&&pickerItems.every(m=>m.category==='serie'));
  await page.waitForFunction(()=>document.querySelectorAll('#picker-items .poster img').length>=5,{},{timeout:2500});
  assert.match(await page.locator('#picker-selected').textContent(),/1 sélectionnée/);
  if(process.env.CHILL_UI_SCREENSHOTS)await page.screenshot({path:path.join(process.env.CHILL_UI_SCREENSHOTS,'picker-posters.png')});
  await page.locator('#picker-back').click();await page.locator('#search').fill('Documentaire');await page.waitForFunction(()=>catalogItems.length===26);
  assert.ok(await page.locator('#media .poster').count()>0);assert.equal(await page.locator('#category').inputValue(),'');
  await page.locator('#category').selectOption('documentaire');await page.waitForFunction(()=>catalogItems.every(m=>m.category==='documentaire'));
  await page.locator('#admin-toggle').click();
  try{await page.locator('#admin-panel').waitFor({state:'visible',timeout:5000});}catch(error){console.error('SYNTHETIC_ADMIN_STATE',await page.evaluate(()=>({stage:state.stage,scope:collectionScope(),baseline:catalogProfile,section:shellSection,busy,message:$('message').textContent,calls:window.testCalls.slice(-12).map(c=>c.action)})));throw error;}
  await page.locator('#player-fullscreen-default').uncheck();await page.locator('#player-presentation-form button').click();
  await page.waitForFunction(()=>state.player_presentation.fullscreen_on_start===false);
  assert.deepEqual(errors,[]);console.log('PASS synthetic-browser profile-avatar/menu and full-page collections/back/plus/mosaic/custom-poster-search/category-filters/selection/local-setting; no native-video claim');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
