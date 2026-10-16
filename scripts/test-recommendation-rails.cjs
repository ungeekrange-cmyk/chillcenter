"use strict";
const assert=require("node:assert/strict"),path=require("node:path");
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?path.join(process.env.CHILL_PREVIEW_NODE_MODULES,"playwright"):"playwright");
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",args:["--disable-background-networking"]});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:900},reducedMotion:"reduce"}),errors=[];
  page.on("pageerror",e=>errors.push(e.message));await page.route(/^https?:/,r=>r.abort());
  await page.addInitScript(()=>{
   const image="data:image/svg+xml,"+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300"><rect width="200" height="300" fill="#9b5428"/></svg>');
   const film=id=>({id,title:"Film "+id,path:"",category:"film",year:2024,genres:["Drama"],cast:["Actor"],directors:["Director"],poster:true,available:true,watched:false,rating:null,overview:"Fixture",series:null,season:null,episode:null,runtime_minutes:null});
   const shows=[{...film("show-old"),category:"serie",title:"Same show",series:"Same show",tmdb_id:1,series_key:"tmdb:tv:1"},{...film("show-new"),category:"serie",title:"Same show",series:"Same show",tmdb_id:2,series_key:"tmdb:tv:2"}];
   window.recommendations=[...Array.from({length:18},(_,i)=>({...film("suggestion-"+i),series_key:null,matches:["Genres : Drama"]})),...shows];
   const source=film("source");window.calls=[];window.aiCalls=[];
   window.__TAURI__={core:{invoke:async(name,{command:c})=>{
    if(name==="ai_command"){
     window.aiCalls.push(c);
     if(c.action==="start")return {id:"advice",stage:"preview",mode:"search",items:window.recommendations,plan:{message:"Fixture grounded in genres and people",changes:[],matched_ids:["show-new",...window.recommendations.filter(m=>m.id!=="show-new").map(m=>m.id)]}};
     if(c.action==="cancel")return {stage:"cancelled"};
     throw Error("Unexpected AI fixture action "+c.action);
    }
    window.calls.push(c);
    if(c.action==="boot")return {stage:"connected",server_id:"fixture-server",profile_id:"fixture-profile",profiles:[{id:"fixture-profile",name:"Fixture",revision:1}],library:{revision:1},theme:"dark",servers:[]};
    if(c.action==="catalog")return {items:[],has_more:false};if(c.action==="sync_catalog")return {changed:false};
    if(c.action==="resolve_appearance")return {ambiance:null,appearance:{mode:"off"}};
    if(c.action==="media_detail")return {media:c.media_id==="source"?source:window.recommendations.find(m=>m.id===c.media_id),revision:1};
    if(c.action==="related_media")return {items:[...window.recommendations,{...shows[0],id:"duplicate"},{...shows[0],id:"raw-episode",episode:3,title:"Forbidden episode"}]};
    if(c.action==="artwork")return {src:image};
    if(c.action==="series"){const show=shows.find(m=>m.series_key===c.name);return {name:c.name,episodes:[{...show,season:1,episode:1}],metadata:{name:show.title},season_metadata:{},revision:1,counters:{unwatched:1},tmdb_id:show.tmdb_id};}
    throw Error("Unexpected fixture action "+c.action);
   }}};
  });
  await page.goto("file://"+path.resolve(__dirname,"../ui/connected/index.html"));await page.evaluate(()=>window.chillReady);
  await page.evaluate(()=>openMedia("source"));await page.locator("#related-media .recommendation-card").first().waitFor();
  for(const [selector,expectedFirst] of [["#related-media","Film suggestion-0"],["#advice-results","Same show"]]){
   if(selector==="#advice-results"){
    await page.evaluate(()=>$("media-dialog").close());await page.locator("#advice-toggle").click();
    await page.locator("#advice-request").fill("Des films ou séries avec Actor, dans le genre Drama");await page.locator("#advice-start").click();
    await page.locator("#advice-results .recommendation-card").first().waitFor();assert.equal(await page.evaluate(()=>window.aiCalls.find(c=>c.action==="start").recommendations),true);
   }
   const cards=page.locator(selector+" .recommendation-card"),rail=page.locator(selector+" .recommendation-rail");assert.equal(await cards.count(),20);
   assert.equal(await cards.first().locator(".recommendation-title").textContent(),expectedFirst);
   const tops=await cards.evaluateAll(nodes=>nodes.map(n=>Math.round(n.getBoundingClientRect().top)));assert.equal(new Set(tops).size,1,"Only one row");
   assert.equal(await page.locator(selector).getByText("Forbidden episode",{exact:true}).count(),0);
   await page.locator(selector+' button[aria-label="Voir les recommandations suivantes"]').click();await page.waitForFunction(selector=>document.querySelector(selector+" .recommendation-rail").scrollLeft>0,selector);
   await rail.evaluate(e=>e.scrollLeft=e.scrollWidth);await cards.last().scrollIntoViewIfNeeded();assert.ok(await cards.last().isVisible());
   await page.locator(selector+' button[aria-label="Voir les recommandations précédentes"]').click();
  }
  const before=await page.evaluate(()=>window.calls.length);await page.locator("#advice-results .recommendation-card").first().click();
  await page.waitForFunction(before=>window.calls.slice(before).some(c=>c.action==="series"&&c.name==="tmdb:tv:2"),before);
  assert.deepEqual(errors,[]);
  console.log("PASS recommendation rails: related+advice, 20 single-row cards, all reachable by horizontal scroll, no episodes, distinct homonymous shows, provider ranking and series navigation. No provider/server/video.");
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
