// Isolated browser regression: real UI/CSS, synthetic bridge, no Pi or video.
const {chromium,webkit}=require('playwright'),fs=require('node:fs'),assert=require('node:assert/strict'),path=require('node:path');
(async()=>{
 let tested=0;
 for(const [name,engine] of [['chromium',chromium],['webkit',webkit]]){
  if(!fs.existsSync(engine.executablePath())){console.log('NOT_AVAILABLE '+name);continue;}
  const browser=await engine.launch({headless:true});tested++;
  try{
   const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[];
   page.on('pageerror',e=>errors.push(e.message));
   await page.route('http://**',route=>route.abort());await page.route('https://**',route=>route.abort());
   await page.addInitScript(()=>{
    const videos=Array.from({length:160},(_,i)=>({id:'video-'+i,title:'Video '+i,category:'film',year:2011,path:'Films/Attack the Block - - (2011) - - Vosta.mkv',overview:'Résumé synthétique. '.repeat(1200),genres:[],directors:[],cast:[],available:true,watched:false,poster:null}));
    const state={stage:'connected',server_id:'synthetic',profile_id:'profile',profiles:[{id:'profile',name:'Test',revision:1}],library:{revision:1,video_root:null},theme:'dark',playback_cache:{forward_mib:128,back_mib:64},player_presentation:{fullscreen_on_start:true}};
    window.__TAURI__={core:{invoke:async(name,{command:c})=>{
     if(c.action==='boot'||c.action==='refresh')return structuredClone(state);
     if(c.action==='catalog')return {items:videos.slice(c.offset,c.offset+48),has_more:c.offset+48<videos.length};
     if(c.action==='media_detail')return {media:videos.find(m=>m.id===c.media_id),revision:1};
     if(c.action==='related_media')return {items:[]};
     if(c.action==='resolve_appearance')return {ambiance:null,appearance:{mode:'off',theme:'dark'}};
     throw Error('Unexpected synthetic command '+c.action);
    }}};
   });
   await page.goto('file://'+path.resolve(__dirname,'../ui/connected/index.html'));await page.evaluate(()=>window.chillReady);
   await page.evaluate(()=>scrollTo(0,500));
   await page.evaluate(()=>window.catalogOpenMedia('video-1'));
   await page.locator('#media-dialog').waitFor({state:'visible'});
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
   assert.equal(await page.locator('#detail-path').textContent(),'Films/Attack the Block - - (2011) - - Vosta.mkv');
   await page.locator('#media-dialog .dialog-close').click();
   await page.mouse.move(800,500);await page.mouse.wheel(0,600);await page.waitForFunction(y=>scrollY>y,background);
   assert.deepEqual(errors,[]);console.log('PASS '+name+': relative filename/path, internal modal scroll, locked backdrop, restored catalogue scrolling');
  }finally{await browser.close();}
 }
 assert.ok(tested,'No bundled browser available; no installation was attempted.');
})().catch(e=>{console.error(e);process.exitCode=1;});
