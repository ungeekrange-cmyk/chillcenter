"use strict";
const assert=require("node:assert/strict"),path=require("node:path");
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?path.join(process.env.CHILL_PREVIEW_NODE_MODULES,"playwright"):"playwright");
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",args:["--disable-background-networking"]});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000},reducedMotion:"reduce"}),errors=[];
  page.on("pageerror",error=>errors.push(error.message));await page.route(/^https?:/,route=>route.abort());
  await page.addInitScript(()=>{
   const media=(id,title=id)=>({id,title,path:"fixture/"+id+".mp4",year:2024,category:"film",genres:[],directors:[],cast:[],poster:false,available:true,watched:false,rating:null,overview:"Synthetic metadata",series:null,season:null,episode:null});
   window.source=media("source","Avatar The Last Airbender - Aang");window.other=media("other","Different detail");window.fixtureMedia=[window.source,...Array.from({length:119},(_,i)=>media("film-"+i))];
   window.commands=[];let library={video_root:"/fixture/films",additional_video_roots:["/fixture/series"],revision:2};
   window.__TAURI__={core:{invoke:async(name,{command:c})=>{
    window.commands.push(c);
    if(name==="player_command"){
     if(c.action==="start"){window.source.available=false;throw {code:"MEDIA_FILE_MISSING",message:"Synthetic absent file"};}
     return {stage:"closed"};
    }
    if(c.action==="boot")return {stage:"connected",server_id:"fixture",profile_id:"profile",profiles:[{id:"profile",name:"Fixture",revision:1}],library,theme:"light",servers:[]};
    if(c.action==="catalog"){const items=window.fixtureMedia.slice(c.offset||0,(c.offset||0)+48);return {items,has_more:(c.offset||0)+items.length<window.fixtureMedia.length};}
    if(c.action==="sync_catalog")return {changed:false};
    if(c.action==="resolve_appearance")return {ambiance:null,appearance:{mode:"off"}};
    if(c.action==="scan_status")return {running:false,count:null,last_error:null};
    if(c.action==="unavailable")return {items:[],has_more:false,epoch:1};
    if(c.action==="tmdb_status")return {configured:true,auto_enrich:false,source:"stored"};
    if(c.action==="save_library"){library={video_root:c.video_root,additional_video_roots:c.additional_video_roots,revision:3};return {stage:"connected",server_id:"fixture",profile_id:"profile",profiles:[{id:"profile",name:"Fixture",revision:1}],library,theme:"light",servers:[]};}
    if(c.action==="media_detail")return {media:c.media_id==="source"?window.source:window.other,revision:1};
    if(c.action==="related_media")return {items:[]};
    if(c.action==="tmdb_candidates")return {candidates:[{id:42,kind:"movie",title:"Chosen reference",year:2024}]};
    if(c.action==="tmdb_select")return await new Promise(resolve=>{window.confirmAssociation=()=>{window.source.title="Chosen reference";window.source.tmdb_id=42;resolve({saved:true});};});
    throw Error("Unexpected fixture action: "+c.action);
   }}};
  });
  await page.goto("file://"+path.resolve(__dirname,"../ui/connected/index.html"));await page.evaluate(()=>window.chillReady);
  await page.locator("#admin-toggle").click();await page.locator("#configuration-categories").waitFor();
  assert.equal(await page.locator("#configuration-categories button").count(),10);
  await page.locator('#configuration-categories button[data-category="playback"]').click();
  assert.equal(await page.locator("#tmdb-admin").isVisible(),false);assert.equal(await page.locator("#personal-settings").isVisible(),true);
  await page.locator("#configuration-search").fill("SOUS TITRES");assert.equal(await page.locator("#personal-settings").isVisible(),true);
  assert.equal(await page.locator('#configuration-categories button[aria-current="page"]').textContent(),"Tout");
  await page.locator("#configuration-search").fill("no such fixture setting");assert.equal(await page.locator("#configuration-empty").isVisible(),true);
  await page.locator("#configuration-search").fill("");await page.locator('#configuration-categories button[data-category="library"]').click();
  const dimensions=await page.locator("#library-settings").evaluate(card=>({width:card.getBoundingClientRect().width,parent:card.parentElement.getBoundingClientRect().width}));assert.ok(Math.abs(dimensions.width-dimensions.parent)<2,"Full-width settings cards");
  assert.equal(await page.locator("#library-settings h2").textContent(),"Bibliothèque");assert.equal(await page.locator("#library-settings").getByText("Le Mac ne monte pas ce chemin.",{exact:false}).count(),0);
  await page.locator("#library-add-location").click();await page.locator("#additional-library-locations input").last().fill("/fixture/documentaries");
  await page.locator("#library-form button").filter({hasText:"Enregistrer"}).click();
  assert.deepEqual(await page.evaluate(()=>window.commands.find(c=>c.action==="save_library").additional_video_roots),["/fixture/series","/fixture/documentaries"]);
  await page.evaluate(()=>openMedia("source"));await page.locator("#detail-tmdb").getByRole("button",{name:"Rechercher TMDB",exact:true}).click();
  assert.equal(await page.evaluate(()=>window.commands.find(c=>c.action==="tmdb_candidates").query),"","Unedited label uses automatic filename cleanup");
  await page.locator("#detail-tmdb .tmdb-candidates button").click();await page.waitForFunction(()=>Boolean(window.confirmAssociation));
  assert.match(await page.locator("#detail-tmdb .tmdb-association-status").textContent(),/Association en cours avec Chosen reference/);
  assert.equal(await page.locator("#detail-tmdb .tmdb-candidates button").count(),0);assert.equal(await page.locator("body").getAttribute("aria-busy"),null);assert.equal(await page.locator("#search").isDisabled(),false);
  await page.locator("#media-dialog .dialog-close").click();await page.evaluate(()=>openMedia("other"));await page.evaluate(()=>window.confirmAssociation());
  assert.equal(await page.locator("#detail-title").textContent(),"Different detail");assert.equal(await page.locator("#detail-tmdb").getByText("Chosen reference",{exact:true}).count(),0);
  await page.evaluate(()=>openMedia("source"));assert.match(await page.locator("#detail-tmdb .tmdb-association-status").textContent(),/Référence associée.*Chosen reference/);
  await page.locator("#media-dialog .dialog-close").click();await page.locator("#admin-close").click();
  await page.evaluate(()=>scrollTo(0,1200));const scroll=await page.evaluate(()=>scrollY);
  await page.evaluate(()=>window.chillPlay("source"));assert.equal(await page.locator("#player-panel").isVisible(),false);
  assert.equal(await page.locator("body").evaluate(body=>body.classList.contains("player-open")),false);
  assert.equal(await page.evaluate(()=>catalogItems.find(media=>media.id==="source").available),false);assert.equal(await page.evaluate(()=>scrollY),scroll,"Missing launch preserves scroll");
  await page.evaluate(()=>openMedia("source"));assert.equal(await page.locator("#detail-play").isDisabled(),true);assert.equal(await page.locator("#detail-locate").isVisible(),true);
  await page.locator("#detail-locate").click();assert.equal(await page.locator("#library-settings").isVisible(),true);
  assert.deepEqual(errors,[]);
  console.log("PASS connected configuration + matching: 10 categories, global normalized search, full-width cards, multiple locations, immediate TMDB feedback/stale guard, missing-file rollback/tag/scroll/recovery. Synthetic bridge; no provider/server/video.");
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
