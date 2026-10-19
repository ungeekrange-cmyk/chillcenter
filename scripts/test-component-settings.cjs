// Real shared settings UI, synthetic bridge only. No GitHub, Pi or media access.
"use strict";
const assert=require("node:assert/strict"),path=require("node:path");
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+"/playwright":"playwright");
(async()=>{
 const browser=await chromium.launch({executablePath:"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",headless:true,args:["--disable-background-networking","--no-first-run"]});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];
  page.setDefaultTimeout(8000);page.on("pageerror",e=>errors.push(e.message));await page.route(/^https?:/,r=>r.abort());
  await page.addInitScript(()=>{
   localStorage.setItem("chillcenter-interface-language","fr");
   const media={id:"fixture-film",title:"Test movie",path:"fixture.mp4",category:"film",year:2026,genres:[],directors:[],cast:[],available:true,watched:false,rating:null,poster:null,series:null,season:null,episode:null};
   const policy={revision:1,mode:"automatic",hour:2,minute:0};
   window.fixtureUpdates={calls:[],legacy:false,local:{installed:{mac:{version:"0.7.0"},themes:{version:"0.7.0"},api:"2026-10-14"},policies:{mac:{...policy,mode:"check"},themes:{...policy}},operations:[],available:{mac:{version:"0.7.1"}},timezone:"+01:00"},server:{installed:{server:{version:"0.7.0"},web:{version:"0.6.9"},themes:{version:"0.7.0"},api:"2026-10-14"},policies:{server:{...policy},web:{...policy},themes:{...policy}},operations:[],helper_available:true,timezone:"+01:00"}};
   window.__TAURI__={core:{invoke:async(name,{command:c})=>{
    const f=fixtureUpdates;f.calls.push({name,...c});
    if(name==="update_command"){
     if(c.action==="status")return f.local;
     if(c.action==="server_status")return f.legacy?{unsupported:true,installed:{server:{version:"0.6.0"}}}:f.server;
     if(c.action==="check")return new Promise(resolve=>{f.finishCheck=()=>resolve({operation:{status:"pending"}});});
     if(c.action==="policy"){Object.assign(f.local.policies[c.component],{mode:c.mode,hour:c.hour,minute:c.minute,revision:2});return {};}
     return {};
    }
    if(c.action==="boot")return {stage:"connected",server_id:"fixture",profile_id:"profile",profiles:[{id:"profile",name:"Test",revision:1}],library:{revision:1},theme:"dark",servers:[]};
    if(c.action==="catalog")return {items:[media],has_more:false,series_counters:{}};
    if(c.action==="sync_catalog")return {changed:false};
    if(c.action==="resolve_appearance")return {ambiance:null,appearance:{mode:"off",manual:"winter"}};
    if(c.action==="media_detail")return {media,revision:1};
    if(c.action==="related_media")return {items:[]};
    if(c.action==="scan_status")return {running:false,count:null,last_error:null};
    if(c.action==="unavailable")return {items:[],has_more:false,epoch:1};
    if(c.action==="tmdb_status")return {configured:false};
    throw Error("Unexpected synthetic action "+name+":"+c.action);
   }}};
  });
  await page.goto("file://"+path.resolve(__dirname,"../ui/connected/index.html"));await page.evaluate(()=>window.chillReady);
  await page.locator("#admin-toggle").click();await page.locator('#configuration-categories button[data-category="general"]').click();
  const settings=page.locator("#update-settings");await settings.getByText("Client Mac : 0.7.0",{exact:true}).waitFor();
  assert.match(await settings.textContent(),/Client web · Serveur · 0.6.9/);
  assert.match(await settings.textContent(),/API : 2026-10-14/);
  const forms=settings.locator("form");assert.equal(await forms.count(),5);
  assert.equal(await forms.first().locator('option[value="automatic"]').count(),0,"Mac cannot silently install");
  const time=forms.first().locator('input[type="time"]');await time.fill("03:15");await time.focus();
  await page.evaluate(()=>window.chillUpdates.refresh());
  assert.equal(await time.inputValue(),"03:15","Status polling preserves the active edit");
  await forms.first().getByRole("button",{name:"Enregistrer",exact:true}).click();
  await page.waitForFunction(()=>fixtureUpdates.calls.some(c=>c.name==="update_command"&&c.action==="policy"));
  const sent=await page.evaluate(()=>fixtureUpdates.calls.find(c=>c.name==="update_command"&&c.action==="policy"));
  assert.equal(sent.expected_revision,1);assert.equal(sent.hour,3);assert.equal(sent.minute,15);
  await settings.getByRole("button",{name:"Vérifier maintenant",exact:true}).first().click();
  await page.waitForFunction(()=>Boolean(fixtureUpdates.finishCheck));
  assert.equal(await page.locator("#search").isEnabled(),true);
  await page.locator("#admin-close").click();await page.locator(".media-card h2").click();
  assert.equal(await page.locator("#detail-title").textContent(),"Test movie","Catalogue remains usable during a held update request");
  await page.evaluate(()=>fixtureUpdates.finishCheck());await page.locator("#media-dialog .dialog-close").click();
  await page.locator("#admin-toggle").click();await page.evaluate(async()=>{fixtureUpdates.legacy=true;await window.chillUpdates.refresh(true);});
  assert.match(await settings.textContent(),/Ce serveur ne gère pas encore/);
  assert.match(await settings.textContent(),/Serveur : 0.6.0/);
  assert.equal(await settings.locator("form").count(),2,"Only unsupported server controls disappear");
  assert.deepEqual(errors,[]);
  console.log("PASS component settings: actual versions, native consent boundary, independent slow IPC/catalogue use, CAS policy, draft preservation and old-server fallback. Synthetic bridge only.");
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
