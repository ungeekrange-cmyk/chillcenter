// Real connected UI, isolated synthetic bridge; no server, profile or video mutations.
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+'/playwright':'playwright');
const assert=require('node:assert/strict'),path=require('node:path'),fs=require('node:fs/promises');
function luminance(raw){const hex=raw.trim().replace('#','');const channels=raw.trim().startsWith('#')?(hex.length===3?[...hex].map(c=>c+c):hex.match(/../g)).map(c=>parseInt(c,16)):raw.match(/[\d.]+/g).slice(0,3).map(Number);const c=channels.map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);return c[0]*.2126+c[1]*.7152+c[2]*.0722}
function contrast(a,b){const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)}
(async()=>{const browser=await chromium.launch({executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run']});try{
 const out=process.env.CHILL_V05_EVIDENCE;assert(out);await fs.mkdir(out,{recursive:true});
 const themeRoot=path.resolve(__dirname,'../themes/default'),manifest=JSON.parse(await fs.readFile(path.join(themeRoot,'manifest.json'),'utf8'));
 const fixturePack={manifest,assets:Object.fromEntries(await Promise.all(Object.keys(manifest.assets).map(async name=>[name,'data:image/png;base64,'+(await fs.readFile(path.join(themeRoot,name))).toString('base64')])))};
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.setDefaultTimeout(8000);page.on('pageerror',e=>errors.push(e.message));await page.route('https://**/*',r=>r.abort());await page.route('http://**/*',r=>r.abort());
 const assertEventAsset=async(event,tone,scene)=>{
  const receipt=await page.evaluate(async()=>{const value=document.documentElement.style.getPropertyValue('--event-art'),url=/^url\("([^"]+)"\)$/.exec(value)?.[1];if(!url||!url.startsWith('blob:'))return null;const bytes=await(await fetch(url)).arrayBuffer(),hash=await crypto.subtle.digest('SHA-256',bytes);return {url,sha256:[...new Uint8Array(hash)].map(b=>b.toString(16).padStart(2,'0')).join('')};});
  assert(receipt,'Native theme must use a locally minted Blob URL');
  assert(scene.image.includes(receipt.url),'Computed background must consume its image URL');
  assert.equal(receipt.sha256,manifest.assets[manifest.themes.find(theme=>theme.id===event)[tone].background].sha256,'Rendered background bytes must match this event, not the previous event');
 };
 await page.addInitScript(pack=>{window.fixtureAmbiance='christmas';window.fixtureCalls=[];window.__TAURI__={core:{invoke:async(name,args)=>{const c=args.command;window.fixtureCalls.push(c.action);if(name==='theme_command'&&c.action==='load')return pack;if(c.action==='boot')return {stage:'connected',server_id:'appearance-fixture',profile_id:'appearance-profile',profiles:[{id:'appearance-profile',name:'Test',revision:1}],library:{revision:1},theme:'dark',servers:[]};if(c.action==='catalog')return {items:[],has_more:false};if(c.action==='sync_catalog')return {changed:false};if(c.action==='resolve_appearance')return {ambiance:window.fixtureAmbiance,appearance:{mode:window.fixtureAmbiance?'manual':'off',manual:window.fixtureAmbiance||'winter'}};throw Error('Unexpected fixture command '+c.action)}}};},fixturePack);
 await page.goto('file://'+path.resolve(__dirname,'../ui/connected/index.html'));await page.evaluate(()=>window.chillReady);
 assert.equal(await page.locator('.global-search').isVisible(),true,'Regression uses the visible connected search header');
 for(const event of ['christmas','halloween'])for(const tone of ['dark','light']){
  await page.evaluate(async([event,tone])=>{window.fixtureAmbiance=event;state.theme=tone;document.documentElement.dataset.theme=tone;await refreshAppearance()},[event,tone]);
  const bg=await page.locator('.sidebar').evaluate(el=>getComputedStyle(el).backgroundColor);assert.equal(bg,({christmas:{dark:'rgb(112, 16, 25)',light:'rgb(186, 48, 40)'},halloween:{dark:'rgb(53, 32, 20)',light:'rgb(245, 212, 167)'}})[event][tone]);
  const sidebarPair=await page.locator('.sidebar').evaluate(el=>{const s=getComputedStyle(el);return [s.getPropertyValue('--accent'),s.getPropertyValue('--button')]});assert(contrast(...sidebarPair)>=4.5,'Sidebar accent/button tokens must pair accessibly');
  const avatarPair=await page.locator('#profile-avatar').evaluate(el=>{const s=getComputedStyle(el);return [s.backgroundColor,s.color]});assert(contrast(...avatarPair)>=4.5,'Actual workspace avatar keeps its own readable root palette');
  const scene=await page.locator('.workspace').evaluate(el=>({image:getComputedStyle(el,'::before').backgroundImage,position:getComputedStyle(el,'::before').backgroundPosition,left:getComputedStyle(el,'::before').left,pointer:getComputedStyle(el,'::before').pointerEvents}));await assertEventAsset(event,tone,scene);assert.match(scene.position,/100% 50%/);assert.equal(scene.left,'235px');assert.equal(scene.pointer,'none');
  assert.equal(await page.locator('.app-topbar').evaluate(el=>getComputedStyle(el).backgroundColor),'rgba(0, 0, 0, 0)','Event artwork must remain visible behind the topbar: '+event+'/'+tone);
  assert.equal(await page.locator('.workspace').evaluate(el=>getComputedStyle(el,'::before').top),'0px','Event artwork starts at the top of the right pane');
  assert.equal(await page.locator('.ambiance-'+(event==='christmas'?'snow':'bat')).count(),event==='christmas'?14:5);
  await page.screenshot({path:path.join(out,event+'-'+tone+'.png'),fullPage:true});
 }
 await page.emulateMedia({reducedMotion:'reduce'});assert.equal(await page.locator('.ambiance-weather').isVisible(),false);await page.emulateMedia({reducedMotion:'no-preference'});
 for(const event of ['easter','valentine'])for(const tone of ['dark','light']){
  await page.evaluate(async([event,tone])=>{window.fixtureAmbiance=event;state.theme=tone;document.documentElement.dataset.theme=tone;await refreshAppearance()},[event,tone]);
  const scene=await page.locator('.workspace').evaluate(el=>({image:getComputedStyle(el,'::before').backgroundImage,content:getComputedStyle(el,'::before').content}));await assertEventAsset(event,tone,scene);assert.equal(scene.content,'""');
  assert.equal(await page.locator('.ambiance-'+(event==='easter'?'rabbit':'petal')).count(),event==='easter'?2:10);
  if(event==='easter'){
   await page.evaluate(()=>{for(const a of document.getAnimations())if(a.animationName.startsWith('chill-rabbit')){a.pause();a.currentTime=a.animationName==='chill-rabbit-roam'?16000:700;}});
   const poses=await page.locator('.ambiance-rabbit').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect(),s=getComputedStyle(n);return{width:r.width,height:r.height,transform:s.transform};}));
   for(const pose of poses){assert(Math.abs(pose.width-64)<.05);assert(Math.abs(pose.height-51.2)<.05);}
   assert.equal(poses[0].transform,'none');assert.equal(poses[1].transform,'matrix(-1, 0, 0, 1, 0, 0)');
  }
  await page.screenshot({path:path.join(out,event+'-'+tone+'.png'),fullPage:true});
  await page.evaluate(()=>document.body.classList.add('player-open'));assert.equal(await page.locator('.workspace').evaluate(el=>getComputedStyle(el,'::before').content),'none','Player cutout must never retain event art');await page.evaluate(()=>document.body.classList.remove('player-open'));
 }
 await page.evaluate(()=>{Object.defineProperty(document,'hidden',{configurable:true,get:()=>true});document.dispatchEvent(new Event('visibilitychange'))});assert.equal(await page.locator('.ambiance-bat').first().evaluate(el=>getComputedStyle(el).animationPlayState),'paused');
 await page.evaluate(()=>{Object.defineProperty(document,'hidden',{configurable:true,get:()=>false});document.dispatchEvent(new Event('visibilitychange'));document.body.classList.add('player-open')});assert.equal(await page.locator('.ambiance-bat').first().evaluate(el=>getComputedStyle(el).animationPlayState),'paused');await page.evaluate(()=>document.body.classList.remove('player-open'));
 await page.evaluate(async()=>{window.fixtureAmbiance=null;await refreshAppearance()});assert.equal(await page.locator('.workspace').evaluate(el=>getComputedStyle(el,'::before').content),'none');assert.equal(await page.locator('.ambiance-weather').isVisible(),false);
 assert.notEqual(await page.locator('.app-topbar').evaluate(el=>getComputedStyle(el).backgroundColor),'rgba(0, 0, 0, 0)','Non-event header keeps its original solid background');
 for(const width of [760,1000,1440]){await page.setViewportSize({width,height:1000});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));}
 await page.evaluate(async()=>{window.fixtureAmbiance='easter';await refreshAppearance();const old=window.__TAURI__.core.invoke;window.__TAURI__.core.invoke=(name,args)=>name==='theme_command'?Promise.reject(Error('fixture unavailable')):old(name,args);await reloadThemePack();});
 assert.equal(await page.locator('.workspace').evaluate(el=>getComputedStyle(el,'::before').content),'none','Total pack failure must clear the previous backdrop');
 assert.equal(await page.evaluate(()=>document.documentElement.style.getPropertyValue('--event-rabbit')),'');
 assert.deepEqual(errors,[]);console.log('PASS connected appearance: four palettes, packaged right-anchored backgrounds, bounded snow/bats, reduced-motion/hidden/player pause, off, native min-width layout; no server or video touched.');
}finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
