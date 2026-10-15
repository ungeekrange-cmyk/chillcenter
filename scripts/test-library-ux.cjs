// Synthetic bridge + disposable browser. No Pi, NAS, personal profile or video.
const {chromium}=require('playwright'),assert=require('node:assert/strict'),path=require('node:path');
(async()=>{const browser=await chromium.launch({headless:true});try{
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(()=>{
  const items=Array.from({length:160},(_,i)=>({id:'video-'+i,path:'Films/Film '+i+'.mkv',title:'Film '+i,category:'film',year:2025,genres:[],cast:[],directors:[],overview:i===1?'':'Résumé '+i,poster:i!==1?'poster':null,available:true,watched:false,rating:null,tmdb_id:null}));window.fixtureItems=items;
  const view={stage:'connected',server_id:'synthetic',profile_id:'profile',profiles:[{id:'profile',name:'Test',revision:1}],library:{revision:1},theme:'dark'};
  for(const [index,item] of items.slice(150,152).entries())Object.assign(item,{category:'serie',series:'Doctor Who',season:1,episode:index+1,tmdb_id:57243});
  const playlist={id:'playlist',name:'Ma liste',revision:1,media_ids:[]};let mirror=structuredClone(items),mirrorPlaylist=structuredClone(playlist),generation=0,seasonPoster=false,mirrorSeason=false;
  function publish(){mirror=structuredClone(items);mirrorPlaylist=structuredClone(playlist);mirrorSeason=seasonPoster;}
  function src(fresh){return {src:'data:image/svg+xml;base64,'+btoa('<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300"><rect width="200" height="300" fill="'+(fresh?'#aa3412':'#483c72')+'"/></svg>')};}
  window.calls=[];window.__TAURI__={core:{invoke:async(name,args)=>{
   if(name==='artwork_command'){generation++;if(args.target.kind==='series')items.filter(m=>m.category==='serie').forEach(m=>{m.poster='new';m.poster_custom=true;});else if(args.target.kind==='season')seasonPoster=true;else items.find(m=>m.id===args.target.media_id).poster='new';return {ok:true};}
   if(name==='ai_command'){window.fixtureAiCalls=(window.fixtureAiCalls||[]).concat(args.command.action);if(args.command.action==='settings')return {provider:'codex',codex_path:'codex',claude_path:'claude'};throw Error('No real AI invocation is allowed in this fixture');}
   const c=args.command;window.calls.push(c);
   if(c.action==='boot'||c.action==='refresh')return structuredClone(view);
   if(c.action==='sync_catalog'){const started=generation;if(window.delaySync)return new Promise(r=>window.releaseSync=()=>{if(started!==generation)return r({changed:false,discarded:true});publish();r({changed:true});});publish();return {changed:true};}
   if(c.action==='catalog'){let found=mirror.filter(m=>!c.q||m.title.toLowerCase().includes(c.q.toLowerCase()));return {items:structuredClone(found.slice(c.offset,c.offset+48)),has_more:c.offset+48<found.length};}
   if(c.action==='media_detail')return {media:structuredClone(mirror.find(m=>m.id===c.media_id)),revision:1};
   if(c.action==='series')return {name:c.name,episodes:structuredClone(mirror.filter(m=>m.category==='serie')),tmdb_id:mirror.find(m=>m.category==='serie').tmdb_id,metadata:{name:'Doctor Who'},season_metadata:mirrorSeason?{'1':{poster:true,overview:'Résumé saison'}}:{},revision:1,counters:{unwatched:2}};
   if(c.action==='season_artwork')return src(true);
   if(c.action==='playlists')return {items:[structuredClone(mirrorPlaylist)]};if(c.action==='playlist')return {playlist:structuredClone(mirrorPlaylist)};
   if(c.action==='related_media'||c.action==='sagas')return {items:[]};
   if(c.action==='add_playlist'){generation++;playlist.media_ids.push(...c.media_ids);playlist.revision++;return {ok:true};}
   if(c.action==='artwork')return src(mirror.find(m=>m.id===c.media_id).poster==='new');
   if(c.action==='group_rating'){generation++;items.find(m=>m.id===c.media_id).rating=c.rating;return {ok:true};}
   if(c.action==='tmdb_candidates'){const kind=items.find(m=>m.id===c.media_id).category==='serie'?'tv':'movie';if(window.delayTmdb)return new Promise(resolve=>window.releaseTmdb=()=>resolve({candidates:[{id:10,title:'Réponse tardive',kind}]}));return {candidates:[{id:10,title:'Choix TMDB',kind,year:2025}]};}
   if(c.action==='tmdb_select'){generation++;Object.assign(items.find(m=>m.id===c.media_id),{tmdb_id:10,title:'Choix TMDB',overview:'Résumé TMDB',poster:'new'});return {ok:true};}
   if(c.action==='resolve_appearance')return {ambiance:null,appearance:{mode:'off',theme:'dark'}};
   if(c.action==='tmdb_status')return {configured:false};if(c.action==='scan_status')return {running:false,count:null};if(c.action==='unavailable')return {items:[],has_more:false};
   throw Error('Unexpected fixture command '+c.action);
  }}};
 });
 await page.goto('file://'+path.resolve(__dirname,'../ui/connected/index.html'));await page.evaluate(()=>window.chillReady);await page.waitForFunction(()=>catalogItems.length===160&&!syncPromise);
 await page.evaluate(async()=>{$('organize-ai').disabled=true;await run({action:'refresh'});});assert.equal(await page.locator('#organize-ai').isEnabled(),true);
 await page.locator('#admin-toggle').click();await page.locator('#organize-ai').click();await page.locator('#ai-dialog').waitFor({state:'visible'});assert.deepEqual(await page.evaluate(()=>window.fixtureAiCalls),['settings']);await page.locator('#ai-dialog').getByRole('button',{name:'Fermer',exact:true}).click();await page.locator('#admin-close').click();
 await page.locator('.media-card').first().hover();assert.equal(await page.locator('.media-card select').count(),0);assert.equal(await page.locator('.media-card .poster-actions .play-video').first().isVisible(),true);
 await page.evaluate(()=>openMedia('video-0'));await page.locator('#detail-poster img').waitFor();
 await page.evaluate(()=>openMedia('video-1'));assert.equal(await page.locator('#detail-title').textContent(),'Film 1');assert.equal(await page.locator('#detail-poster img').count(),0);assert.equal(await page.locator('#detail-overview').textContent(),'Informations à compléter');
 assert.equal(await page.locator('#detail-stars button').count(),5);assert.equal(await page.locator('#detail-rating-scope').count(),0);
 await page.locator('#detail-stars button').nth(3).click();await page.waitForFunction(()=>document.querySelectorAll('#detail-stars [aria-pressed=true]').length===4);
 await page.locator('#detail-artwork').click();await page.locator('#detail-poster img').waitFor();assert.equal(await page.locator('#media-dialog').isVisible(),true);
 await page.locator('#detail-tmdb button').filter({hasText:'Rechercher TMDB'}).click();await page.locator('#detail-tmdb .tmdb-candidates button').click();await page.waitForFunction(()=>$('detail-title').textContent==='Choix TMDB');assert.equal(await page.locator('#media-dialog').isVisible(),true);
 await page.mouse.click(3,3);assert.equal(await page.locator('#media-dialog').isVisible(),false);
 await page.evaluate(()=>scrollTo(0,1200));await page.waitForFunction(()=>scrollY>1000);const before=await page.evaluate(()=>scrollY);await page.evaluate(()=>openMedia('video-40'));await page.mouse.click(3,3);assert.equal(await page.evaluate(()=>scrollY),before);
 await page.evaluate(()=>{window.delaySync=true;void syncCatalog();});await page.waitForFunction(()=>typeof window.releaseSync==='function');assert.equal(await page.locator('#search').isEnabled(),true);assert.equal(await page.locator('.media-card').count()>0,true);await page.locator('#search').fill('Film 9');await page.waitForFunction(()=>catalogItems.length===11);await page.evaluate(()=>{window.delaySync=false;window.releaseSync();});await page.waitForFunction(()=>!syncPromise);
 await page.evaluate(async()=>{window.fixtureItems.forEach((m,index)=>m.tmdb_id=index<2?null:10);await syncAfterMutation();window.delayTmdb=true;});await page.locator('#admin-toggle').click();await page.locator('#matching-admin button').filter({hasText:'Lancer la reconnaissance forcée'}).click();await page.waitForFunction(()=>typeof window.releaseTmdb==='function');assert.equal(await page.locator('#search').isEnabled(),true);assert.match(await page.locator('.matching-row span').first().getAttribute('title'),/Films\//);await page.locator('#matching-admin button').filter({hasText:'Arrêter les recherches'}).click();await page.evaluate(()=>{window.delayTmdb=false;window.releaseTmdb();});assert.match(await page.locator('#matching-admin').textContent(),/Recherche arrêtée/);
 await page.locator('#matching-admin button').filter({hasText:'Lancer la reconnaissance forcée'}).click();await page.locator('.matching-row .tmdb-candidates button').first().waitFor();await page.waitForFunction(()=>!syncPromise);
 await page.evaluate(()=>{window.delaySync=true;void syncCatalog();});await page.waitForFunction(()=>!!syncPromise);
 await page.evaluate(()=>{document.querySelector('.matching-row .tmdb-candidates button').click();void openMedia('video-0');});await page.waitForFunction(()=>window.calls.filter(c=>c.action==='tmdb_select').length===2);
 await page.evaluate(()=>{window.delaySync=false;window.releaseSync();});await page.waitForFunction(()=>$('detail-title').textContent==='Choix TMDB'&&!syncPromise);await page.mouse.click(3,3);
 await page.evaluate(()=>window.catalogPickCollection('video-9','film'));await page.locator('#playlist-targets button').filter({hasText:'Ma liste'}).click();await page.waitForFunction(()=>!$('playlist-target-dialog').open);await page.evaluate(()=>openCollections('playlist'));await page.waitForFunction(()=>$('collections-list').textContent.includes('1 vidéos'));
 await page.evaluate(()=>openSeries('tmdb:tv:10'));await page.locator('#series-common .poster img').waitFor();await page.locator('#series-common button').filter({hasText:'Importer une image'}).click();await page.waitForFunction(()=>document.querySelector('#series-common .poster img')&&window.fixtureItems.filter(m=>m.category==='serie').every(m=>m.poster_custom));
 await page.locator('.series-season summary').click();await page.locator('.season-provider button').filter({hasText:'Importer une image'}).click();
 try{await page.locator('.series-season[open] > .series-poster').scrollIntoViewIfNeeded();await page.waitForFunction(()=>document.querySelector('.series-season[open] > .series-poster img'));}catch(error){console.error('SEASON_FIXTURE_STATE',await page.evaluate(()=>({open:document.querySelector('.series-season')?.open,metadata:seriesView?.season_metadata,tmdb:seriesView?.tmdb_id,calls:window.calls.slice(-8),message:$('message').textContent})));throw error;}assert.equal(await page.locator('#collection-title').textContent(),'Doctor Who');
 assert.deepEqual(errors,[]);console.log('PASS synthetic library UX: poster controls, five stars, detail reset, in-place artwork/matching, backdrop, scroll preservation, usable search during delayed sync.');
 }finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
