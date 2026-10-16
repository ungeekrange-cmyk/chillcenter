"use strict";
// Real connected DOM, synthetic Rust bridge: no video decoding or household server.
const assert=require("node:assert/strict"),path=require("node:path"),fs=require("node:fs");
const {chromium}=require(path.join(process.env.CHILL_PREVIEW_NODE_MODULES,"playwright"));
const evidence=process.env.CHILL_SERIES_EVIDENCE;
if(!evidence||!path.isAbsolute(evidence))throw Error("Set absolute CHILL_SERIES_EVIDENCE");
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",args:["--disable-background-networking"]});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on("pageerror",e=>errors.push(e.message));
  await page.route(/^https?:/,route=>route.abort());
  await page.addInitScript(()=>{
   window.fixtureAllSeen=false;window.fixtureCalls=[];
   const episode=(season,number)=>({id:`episode-${season}-${number}`,title:`Épisode ${number}`,series:"Série de démonstration",season,episode:number,category:"serie",available:true,poster:false,genres:[],directors:[],cast:[],watched:window.fixtureAllSeen||season===1,rating:null});
   window.__TAURI__={core:{invoke:async(channel,{command})=>{
    window.fixtureCalls.push(command);
    if(channel==="player_command")return {stage:"closed"};
    if(command.action==="boot")return {stage:"connected",server_id:"series-server",profile_id:"series-profile",profiles:[{id:"series-profile",name:"Camille",revision:1}],library:{revision:1},theme:"dark",servers:[],player_presentation:{fullscreen_on_start:false}};
    if(command.action==="catalog")return {items:[],has_more:false};
    if(command.action==="sync_catalog")return {changed:false};
    if(command.action==="resolve_appearance")return {ambiance:null,appearance:{mode:"off"}};
    if(command.action==="series")return {name:command.name,revision:1,metadata:{name:"Série de démonstration",year:2025,genres:["Comédie"],overview:"Un résumé de la série à côté de son affiche. Les saisons et les épisodes se parcourent ensuite dans une grille, sans empiler les fiches."},episodes:[episode(3,1),episode(2,4),episode(1,1),episode(2,2),episode(2,1),episode(2,3)],counters:{unwatched:window.fixtureAllSeen?0:5}};
    throw Error("Unexpected command: "+command.action);
   }}};
  });
  await page.goto("file://"+path.resolve(__dirname,"../ui/connected/index.html"));await page.evaluate(()=>window.chillReady);
  await page.evaluate(()=>window.catalogOpenSeries("series-fixture"));
  assert.deepEqual(await page.locator(".series-season[open]").evaluateAll(items=>items.map(e=>e.dataset.season)),["2"]);
  assert.deepEqual(await page.locator('.series-season[data-season="2"] .collection-media-card').evaluateAll(items=>items.map(e=>e.dataset.mediaId)),["episode-2-1","episode-2-2","episode-2-3","episode-2-4"]);
  fs.mkdirSync(evidence,{recursive:true});
  for(const width of [1440,760]){
   await page.setViewportSize({width,height:1000});
   const geometry=await page.evaluate(()=>{
    const rect=selector=>{const r=document.querySelector(selector).getBoundingClientRect();return {x:r.x,y:r.y,right:r.right,bottom:r.bottom,width:r.width};};
    const grid=document.querySelector('.series-season[open] .series-episodes');
    return {poster:rect('#series-common>.series-poster'),copy:rect('.series-overview-copy'),members:rect('#collection-members'),tools:rect('#series-tools'),columns:getComputedStyle(grid).gridTemplateColumns.split(' ').length,cards:[...grid.children].map(e=>({x:e.getBoundingClientRect().x,y:e.getBoundingClientRect().y}))};
   });
   assert.ok(geometry.copy.x>=geometry.poster.right,"synopsis beside poster");
   assert.ok(Math.abs(geometry.copy.y-geometry.poster.y)<2);
   assert.ok(geometry.tools.y>=geometry.members.bottom,"common tools after all seasons");
   assert.ok(geometry.columns>=2,"episodes have multiple columns even at native minimum width");
   assert.equal(geometry.cards[0].y,geometry.cards[1].y);assert.ok(geometry.cards[1].x>geometry.cards[0].x);
   assert.equal(await page.locator(".series-overview-copy .rating-star").count(),5);
   await page.screenshot({path:path.join(evidence,`series-${width}.png`),fullPage:true});
  }
  await page.locator('.series-season[data-season="2"]>summary').click();
  assert.equal(await page.locator('.series-season[data-season="2"] .series-episodes').isVisible(),false);
  await page.evaluate(()=>window.catalogOpenSeries("series-fixture"));
  assert.equal(await page.locator(".series-season[open]").count(),0,"refresh preserves manual folds");
  await page.evaluate(async()=>{window.fixtureAllSeen=true;shellShow("catalog");await window.catalogOpenSeries("series-fixture");});
  assert.deepEqual(await page.locator(".series-season[open]").evaluateAll(items=>items.map(e=>e.dataset.season)),["1"]);
  // An actual EOF UI transition, not a replay/decoder test. Reopen the originating
  // series using the same bridge and wait for its updated personal counters.
  await page.evaluate(()=>{playerState={stage:"player",media_id:"episode-3-1",context:{kind:"series",id:"series-fixture"},native:{ended:true,error:null},next:{media:null},autoplay_seconds:20};renderNext();});
  await page.waitForFunction(()=>playerState.stage==="closed"&&window.fixtureCalls.filter(c=>c.action==="series").length===4);
  assert.equal(await page.locator("#collection-panel").isVisible(),true);
  assert.ok(await page.locator(".series-summary-counts").textContent().then(t=>t.includes("0 épisodes non vus")));
  assert.deepEqual(errors,[]);
  console.log("PASS connected DOM: first unseen season, grid at 1440/760px, header/tools order, manual folds, all-seen fallback, terminal EOF return. No real playback/server.");
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
