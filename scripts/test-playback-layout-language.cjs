"use strict";
const assert=require("node:assert/strict"),path=require("node:path");
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?path.join(process.env.CHILL_PREVIEW_NODE_MODULES,"playwright"):"playwright");
const root=process.env.CHILL_UI_ROOT||path.resolve(__dirname,".."),layoutOnly=Boolean(process.env.CHILL_LAYOUT_ONLY);
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",args:["--disable-background-networking"]});
 try{
  for(const locale of layoutOnly?["fr-FR"]:["en-US","fr-FR","de-DE"]){
   const context=await browser.newContext({viewport:{width:1440,height:1000},locale,reducedMotion:"reduce"}),page=await context.newPage(),errors=[];
   page.on("pageerror",error=>errors.push(error.message));await page.route(/^https?:/,route=>route.abort());
   await page.addInitScript(()=>{
    window.commands=[];
    const items=Array.from({length:160},(_,i)=>({id:"fixture-"+i,title:i===0?"Informations à compléter":"Fixture "+i,category:"film",year:2024,genres:["Comédie"],directors:[],cast:[],available:true,watched:false,poster:true,overview:"Résumé original non traduit",rating:null,path:"fixture/film.mp4"}));
    const view=()=>({stage:"connected",server_id:"fixture",profile_id:"profile",profiles:[{id:"profile",name:"Français",revision:1}],library:{revision:1},language:localStorage.getItem("fixture-language")||"auto",theme:"dark"});
    let playing=false;
    window.__TAURI__={core:{invoke:async(name,{command:c})=>{
     window.commands.push(c);
     if(name==="player_command"){
      if(c.action==="stop"){playing=false;return {stage:"closed"};}
      playing=true;return {stage:"player",media_id:"fixture-0",title:items[0].title,position:12,duration:120,context:{kind:"standalone"},native:{loaded:true,paused:false,buffering:false,tracks:[],position:12,duration:120,fullscreen:false}};
     }
     if(c.action==="boot"||c.action==="refresh")return view();
     if(c.action==="save_language"){localStorage.setItem("fixture-language",c.language);return {language:c.language};}
     if(c.action==="catalog")return {items:items.slice(c.offset,c.offset+48),has_more:c.offset+48<items.length};
     if(c.action==="media_detail")return {media:items.find(m=>m.id===c.media_id),revision:1};
     if(c.action==="artwork")return {src:"data:image/svg+xml;base64,"+btoa('<svg xmlns="http://www.w3.org/2000/svg" width="20" height="30"><rect width="20" height="30" fill="orange"/></svg>')};
     if(c.action==="related_media")return {items:[]};if(c.action==="sync_catalog")return {changed:false};
     if(c.action==="resolve_appearance")return {ambiance:null,appearance:{mode:"off"}};
     if(c.action==="tmdb_status")return {configured:false};if(c.action==="scan_status")return {running:false,count:null};
     if(c.action==="unavailable")return {items:[],has_more:false};
     throw Error("Unexpected fixture action: "+c.action);
    }}};
   });
   await page.goto("file://"+path.join(root,"ui/connected/index.html"));await page.evaluate(()=>window.chillReady);
   if(!layoutOnly){
    const expected=locale.startsWith("fr")?"fr":"en";
    assert.equal(await page.locator("html").getAttribute("lang"),expected);
    assert.equal(await page.locator("#catalog-title").textContent(),expected==="fr"?"Toute la collection":"Entire library");
    assert.equal(await page.locator('#media article[data-media-id="fixture-0"] h2').textContent(),"Informations à compléter","metadata is never translated");
   }
   await page.evaluate(()=>openMedia("fixture-0"));await page.locator("#detail-play").click();
   // Reproduce a normal background refresh while CSS removes the catalogue
   // from layout. This must not publish a zero-width, single-column window.
   await page.evaluate(()=>{catalogWindowKey="";renderCatalogWindow();});
   await page.locator("#player-close").click();await page.locator("#media-dialog").waitFor();
   await page.locator("#media-dialog .dialog-close").click();
   await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
   const dimensions=await page.locator("#media article .poster").first().evaluate(n=>({width:n.getBoundingClientRect().width,height:n.getBoundingClientRect().height}));
   assert.ok(dimensions.width<350&&dimensions.width>150,"poster columns restored: "+JSON.stringify(dimensions));
   assert.ok(Math.abs(dimensions.height/dimensions.width-1.5)<0.05,"portrait ratio restored: "+JSON.stringify(dimensions));
   if(!layoutOnly){
    await page.locator("#admin-toggle").click();await page.locator('#configuration-categories [data-category="general"]').click();
    await page.locator("#interface-language").selectOption(locale.startsWith("fr")?"en":"fr");
    const expected=locale.startsWith("fr")?"en":"fr";
    await page.waitForFunction(lang=>document.documentElement.lang===lang,expected);await page.evaluate(()=>window.chillReady);
    assert.equal(await page.locator("#interface-language").inputValue(),expected);
    assert.equal(await page.locator("#admin-panel").isVisible(),true,"return to Settings after language change");
    assert.equal(await page.locator("#profile-name").textContent(),"Français","profile name unchanged");
   }
   assert.deepEqual(errors,[]);await context.close();
  }
  console.log("PASS playback → detail → catalogue restores portrait grid; automatic EN/FR, fallback and persisted override preserve metadata (synthetic bridge)");
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
