"use strict";
// Focused connected-DOM proof only. No server, decoder or household state.
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path");
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?path.join(process.env.CHILL_PREVIEW_NODE_MODULES,"playwright"):"playwright");
const poster="data:image/png;base64,"+fs.readFileSync(path.join(__dirname,"../icons/branding/vector/default.png")).toString("base64");
const luminance=color=>{const rgb=color.match(/[\d.]+/g).slice(0,3).map(Number).map(n=>{n/=255;return n<=.04045?n/12.92:((n+.055)/1.055)**2.4;});return rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722;};
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",args:["--disable-background-networking"]});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[];
  page.on("pageerror",e=>errors.push(e.message));await page.route(/^https?:/,r=>r.abort());
  await page.addInitScript(poster=>{
   window.fixturePoster=poster;window.artworkCalls=[];window.fixtureNoImage=false;
   window.__TAURI__={core:{invoke:async(_,{command})=>{
    if(command.action==="boot")return {stage:"connected",server_id:"transition-server",profile_id:"transition-profile",profiles:[{id:"transition-profile",name:"Fixture",revision:1}],library:{revision:1},theme:"light",servers:[]};
    if(command.action==="catalog")return {items:[],has_more:false};
    if(command.action==="sync_catalog")return {changed:false};
    if(command.action==="resolve_appearance")return {ambiance:null,appearance:{mode:"off"}};
    if(command.action==="artwork"){
     window.artworkCalls.push(command.media_id);
     if(window.fixtureNoImage)throw {code:"ARTWORK_NOT_FOUND",message:"Fixture without poster"};
     return {src:window.fixturePoster};
    }
    throw Error("Unexpected fixture command: "+command.action);
   }}};
  },poster);
  await page.goto("file://"+path.resolve(__dirname,"../ui/connected/index.html"));await page.evaluate(()=>window.chillReady);
  for(const tone of ["light","dark"])for(const fullscreen of [false,true]){
   await page.evaluate(({tone,fullscreen})=>{
    hidePlayer();syncAfterMutation=async()=>{};
    document.documentElement.dataset.theme=tone;document.body.classList.add("player-open");document.body.classList.toggle("player-fullscreen",fullscreen);
    $("player-panel").hidden=false;window.artworkCalls=[];window.fixtureNoImage=false;
    playerState={stage:"player",media_id:"series-current",title:"Fixture série",position:91,duration:100,autoplay_seconds:null,native:{loaded:true,ended:false,error:null},next:{available:true,media:{id:"next",title:"Épisode suivant",available:true,poster:null}}};
    renderPlayerBackdrop();
   },{tone,fullscreen});
   assert.equal(await page.evaluate(()=>window.artworkCalls.length),0);
   await page.evaluate(()=>{playerState.position=92;renderPlayerBackdrop();renderPlayerBackdrop();});
   await page.waitForFunction(()=>$("player-backdrop").querySelector("img")?.complete);
   assert.deepEqual(await page.evaluate(()=>window.artworkCalls),["series-current"]);
   assert.equal(await page.locator("#player-backdrop").evaluate(e=>getComputedStyle(e).opacity),"0","Live video must never be painted over");
   await page.evaluate(()=>{playerState.native.ended=true;renderPlayerBackdrop();renderNext();});
   const css=await page.locator("#player-next-cancel").evaluate(e=>({color:getComputedStyle(e).color,card:getComputedStyle($("player-next")).backgroundColor}));
   // Composite the button's translucent white backing over the fixed dark card.
   const card=css.card.match(/\d+/g).slice(0,3).map(Number),background="rgb("+card.map(n=>Math.round(n+(255-n)*13/255)).join(",")+")";
   const a=luminance(css.color),b=luminance(background);assert.ok((Math.max(a,b)+.05)/(Math.min(a,b)+.05)>=4.5,`${tone}: cancel text contrast`);
   assert.equal(await page.locator("#player-backdrop").evaluate(e=>getComputedStyle(e).opacity),"1");
   assert.match(await page.locator("#player-backdrop img").evaluate(e=>getComputedStyle(e).filter),/blur\(28px\)/);
   const stage=await page.locator("#player-stage").boundingBox(),backdrop=await page.locator("#player-backdrop").boundingBox();assert.deepEqual(backdrop,stage);
   await page.setViewportSize({width:1100,height:740});
   assert.deepEqual(await page.locator("#player-backdrop").boundingBox(),await page.locator("#player-stage").boundingBox());
   await page.setViewportSize({width:1440,height:900});
   await page.evaluate(()=>{playerState.media_id="without-image";playerState.native.ended=false;playerState.position=92;window.fixtureNoImage=true;renderPlayerBackdrop();});
   await page.waitForFunction(()=>window.artworkCalls.includes("without-image"));
   await page.evaluate(()=>{playerState.native.ended=true;renderPlayerBackdrop();});
   assert.equal(await page.locator("#player-backdrop img").count(),0,"No previous series poster may leak into a missing image");
   assert.equal(await page.locator("#player-backdrop").evaluate(e=>getComputedStyle(e).opacity),"1");
   assert.match(await page.locator("#player-backdrop").evaluate(e=>getComputedStyle(e).backgroundImage),/gradient/);
  }
  await page.evaluate(()=>hidePlayer());assert.equal(await page.locator("#player-backdrop img").count(),0);
  assert.deepEqual(errors,[]);
  console.log("PASS transition: prewarm once at 8s, invisible during playback, blurred EOF backdrop and missing-image fallback; contrast >=4.5, light/dark, window/fullscreen and resize. No playback/server.");
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
