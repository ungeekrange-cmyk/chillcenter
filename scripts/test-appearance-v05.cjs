// Real connected UI, isolated synthetic bridge; no server, profile or video mutations.
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+'/playwright':'playwright');
const assert=require('node:assert/strict'),path=require('node:path'),fs=require('node:fs/promises');
function luminance(raw){const hex=raw.trim().replace('#','');const channels=raw.trim().startsWith('#')?(hex.length===3?[...hex].map(c=>c+c):hex.match(/../g)).map(c=>parseInt(c,16)):raw.match(/[\d.]+/g).slice(0,3).map(Number);const c=channels.map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);return c[0]*.2126+c[1]*.7152+c[2]*.0722}
function contrast(a,b){const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)}
(async()=>{const browser=await chromium.launch({executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run']});try{
 const out=process.env.CHILL_V05_EVIDENCE;assert(out);await fs.mkdir(out,{recursive:true});
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.setDefaultTimeout(8000);page.on('pageerror',e=>errors.push(e.message));await page.route('https://**/*',r=>r.abort());await page.route('http://**/*',r=>r.abort());
 await page.addInitScript(()=>{window.fixtureAmbiance='christmas';window.fixtureCalls=[];window.__TAURI__={core:{invoke:async(name,args)=>{const c=args.command;window.fixtureCalls.push(c.action);if(c.action==='boot')return {stage:'server',theme:'dark',servers:[]};if(c.action==='resolve_appearance')return {ambiance:window.fixtureAmbiance,appearance:{mode:'manual',manual:window.fixtureAmbiance||'winter'}};throw Error('Unexpected fixture command '+c.action)}}};});
 await page.goto('file://'+path.resolve(__dirname,'../ui/connected/index.html'));await page.evaluate(()=>window.chillReady);
 for(const event of ['christmas','halloween'])for(const tone of ['dark','light']){
  await page.evaluate(async([event,tone])=>{window.fixtureAmbiance=event;state.theme=tone;document.documentElement.dataset.theme=tone;await refreshAppearance()},[event,tone]);
  const bg=await page.locator('.sidebar').evaluate(el=>getComputedStyle(el).backgroundColor);assert.equal(bg,({christmas:{dark:'rgb(112, 16, 25)',light:'rgb(186, 48, 40)'},halloween:{dark:'rgb(53, 32, 20)',light:'rgb(245, 212, 167)'}})[event][tone]);
  const sidebarPair=await page.locator('.sidebar').evaluate(el=>{const s=getComputedStyle(el);return [s.getPropertyValue('--accent'),s.getPropertyValue('--button')]});assert(contrast(...sidebarPair)>=4.5,'Sidebar accent/button tokens must pair accessibly');
  const avatarPair=await page.locator('#profile-avatar').evaluate(el=>{const s=getComputedStyle(el);return [s.backgroundColor,s.color]});assert(contrast(...avatarPair)>=4.5,'Actual workspace avatar keeps its own readable root palette');
  const scene=await page.locator('.workspace').evaluate(el=>({image:getComputedStyle(el,'::before').backgroundImage,position:getComputedStyle(el,'::before').backgroundPosition,left:getComputedStyle(el,'::before').left,pointer:getComputedStyle(el,'::before').pointerEvents}));assert(scene.image.includes(event+'.png'));assert.match(scene.position,/100% 50%/);assert.equal(scene.left,'235px');assert.equal(scene.pointer,'none');
  assert.equal(await page.locator('.ambiance-'+(event==='christmas'?'snow':'bat')).count(),event==='christmas'?14:5);
  await page.screenshot({path:path.join(out,event+'-'+tone+'.png'),fullPage:true});
 }
 await page.emulateMedia({reducedMotion:'reduce'});assert.equal(await page.locator('.ambiance-weather').isVisible(),false);await page.emulateMedia({reducedMotion:'no-preference'});
 await page.evaluate(()=>{Object.defineProperty(document,'hidden',{configurable:true,get:()=>true});document.dispatchEvent(new Event('visibilitychange'))});assert.equal(await page.locator('.ambiance-bat').first().evaluate(el=>getComputedStyle(el).animationPlayState),'paused');
 await page.evaluate(()=>{Object.defineProperty(document,'hidden',{configurable:true,get:()=>false});document.dispatchEvent(new Event('visibilitychange'));document.body.classList.add('player-open')});assert.equal(await page.locator('.ambiance-bat').first().evaluate(el=>getComputedStyle(el).animationPlayState),'paused');await page.evaluate(()=>document.body.classList.remove('player-open'));
 await page.evaluate(async()=>{window.fixtureAmbiance=null;await refreshAppearance()});assert.equal(await page.locator('.workspace').evaluate(el=>getComputedStyle(el,'::before').content),'none');assert.equal(await page.locator('.ambiance-weather').isVisible(),false);
 for(const width of [760,1000,1440]){await page.setViewportSize({width,height:1000});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));}
 assert.deepEqual(errors,[]);console.log('PASS connected appearance: four palettes, packaged right-anchored backgrounds, bounded snow/bats, reduced-motion/hidden/player pause, off, native min-width layout; no server or video touched.');
}finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
