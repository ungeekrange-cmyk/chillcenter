// Isolated browser regression: real UI/CSS, synthetic bridge, no Pi or video.
const fs=require('node:fs'),assert=require('node:assert/strict'),path=require('node:path');
const {chromium,webkit}=require(process.env.CHILL_PREVIEW_NODE_MODULES?path.join(process.env.CHILL_PREVIEW_NODE_MODULES,'playwright'):'playwright');
(async()=>{
 let tested=0;
 for(const [name,engine] of [['chromium',chromium],['webkit',webkit]]){
  const executablePath=name==='chromium'&&process.env.CHILL_CHROME_PATH||engine.executablePath();
  if(!fs.existsSync(executablePath)){console.log('NOT_AVAILABLE '+name);continue;}
  const browser=await engine.launch({headless:true,executablePath});tested++;
  try{
   const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[];
   page.on('pageerror',e=>errors.push(e.message));
   await page.route('http://**',route=>route.abort());await page.route('https://**',route=>route.abort());
   const installFixture=({web=false}={})=>{
    if(web)window.ChillBridge={};
    window.downloadCommands=[];
    const videos=Array.from({length:160},(_,i)=>({id:'video-'+i,title:'Video '+i,category:'film',year:2011,path:'Films/Attack the Block - - (2011) - - Vosta.mkv',overview:'Résumé synthétique. '.repeat(1200),genres:[],directors:[],cast:[],available:true,watched:false,poster:null}));
    const state={stage:'connected',server_id:'synthetic',profile_id:'profile',profiles:[{id:'profile',name:'Test',revision:1}],library:{revision:1,video_root:null},theme:'dark',offline_downloads:true,playback_cache:{forward_mib:128,back_mib:64},player_presentation:{fullscreen_on_start:true}};
    window.__TAURI__={core:{invoke:async(name,{command:c})=>{
     if(name==='downloads_command'){window.downloadCommands.push(c.action);return {items:[],supported:!web};}
     if(c.action==='boot'||c.action==='refresh')return structuredClone(state);
     if(c.action==='catalog')return {items:videos.slice(c.offset,c.offset+48),has_more:c.offset+48<videos.length};
     if(c.action==='media_detail')return {media:videos.find(m=>m.id===c.media_id),revision:1};
     if(c.action==='related_media')return {items:[]};
     if(c.action==='resolve_appearance')return {ambiance:null,appearance:{mode:'off',theme:'dark'}};
     throw Error('Unexpected synthetic command '+c.action);
    }}};
   };
   await page.addInitScript(installFixture);
   await page.goto('file://'+path.resolve(__dirname,'../ui/connected/index.html'));await page.evaluate(async()=>{await window.chillReady;shellShow("catalog");await loadPage();});
   await page.evaluate(()=>scrollTo(0,500));
   await page.evaluate(()=>window.catalogOpenMedia('video-1'));
   await page.locator('#media-dialog').waitFor({state:'visible'});
   for(const width of [1440,390]){
    await page.setViewportSize({width,height:800});
    assert.ok(await page.locator('#detail-download').evaluate(button=>{
     const r=button.getBoundingClientRect(),d=document.querySelector('#media-dialog').getBoundingClientRect();
     return r.bottom<=d.bottom&&r.top>=d.top;
    }), name+': Download must be above the fold with a long synopsis at '+width);
    for(const selector of ['#detail-editor','#detail-path','#detail-artwork','#detail-tmdb','#detail-stars']){
     assert.equal(await page.locator(selector).isVisible(),false);
    }
   }
   await page.setViewportSize({width:1440,height:900});
   const background=await page.evaluate(()=>scrollY),dialog=page.locator('#media-dialog');
   const rect=await dialog.boundingBox();await page.mouse.move(rect.x+rect.width*.7,rect.y+rect.height*.6);
   await page.mouse.wheel(0,500);await page.waitForFunction(()=>document.querySelector('#media-dialog').scrollTop>0);
   assert.equal(await page.evaluate(()=>scrollY),background,name+': dialog scroll must not move the catalogue');
   await dialog.evaluate(d=>{d.scrollTop=d.scrollHeight;});await page.mouse.wheel(0,900);
   await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
   assert.equal(await page.evaluate(()=>scrollY),background,name+': scroll must not chain beyond the end of the dialog');
   await page.mouse.move(10,10);await page.mouse.wheel(0,900);
   await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
   assert.equal(await page.evaluate(()=>scrollY),background,name+': the backdrop must not scroll the catalogue');
   await page.locator('#detail-edit').click();
   assert.equal(await page.locator('#detail-edit').getAttribute('aria-expanded'),'true');
   assert.equal(await page.locator('#detail-artwork').isVisible(),true);
   assert.equal(await page.locator('#detail-tmdb').isVisible(),true);
   assert.ok(await page.locator('#detail-editor').evaluate(el=>document.querySelector('#related-media').compareDocumentPosition(el)&Node.DOCUMENT_POSITION_FOLLOWING));
   assert.equal(await page.locator('#detail-path').textContent(),'Films/Attack the Block - - (2011) - - Vosta.mkv');
   await page.locator('#detail-edit-close').click();
   assert.equal(await page.locator('#detail-editor').isVisible(),false);
   await page.locator('#media-dialog .dialog-close').click();
   await page.mouse.move(800,500);await page.mouse.wheel(0,600);await page.waitForFunction(y=>scrollY>y,background);
   assert.deepEqual(errors,[]);
   const plain=await browser.newPage(),assets=path.resolve(__dirname,'../ui/connected');
   await plain.addInitScript(installFixture,{web:true});
   await plain.route('http://chillcenter.test/**',async route=>{
    const pathname=new URL(route.request().url()).pathname;
    const file=path.resolve(assets,pathname==='/'?'index.html':'.'+pathname);
    if(!file.startsWith(assets+path.sep)||!fs.existsSync(file))return route.fulfill({status:404});
    await route.fulfill({path:file});
   });
   await plain.goto('http://chillcenter.test/');
   await plain.evaluate(async()=>{await window.chillReady;await window.catalogOpenMedia('video-1');});
   assert.equal(await plain.evaluate(()=>isSecureContext),false);
   await plain.locator('#detail-download').click();
   const feedback=plain.locator('#detail-download-notice [role=status]');
   assert.equal(await feedback.isVisible(),true);
   assert.equal(await feedback.locator('a').getAttribute('href'),'https://chillcenter.test:8443/');
   assert.deepEqual(await plain.evaluate(()=>[...new Set(downloadCommands)]),['list']);
   console.log('PASS '+name+': visible action, collapsed editor, modal scrolling, clear HTTPS entry from HTTP');
  }finally{await browser.close();}
 }
 assert.ok(tested,'No bundled browser available; no installation was attempted.');
})().catch(e=>{console.error(e);process.exitCode=1;});
