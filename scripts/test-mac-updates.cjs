"use strict";
// Real update component in Chromium, synthetic IPC only. Deliberately hold
// startup, server response and installation independently; no family data.
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path");
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+"/playwright":"playwright");
(async()=>{
 const browser=await chromium.launch({executablePath:"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",headless:true,args:["--disable-background-networking","--no-first-run"]});
 try{
  const page=await browser.newPage(),errors=[];page.on("pageerror",error=>errors.push(error.message));
  await page.setContent('<button id="admin-toggle">Configuration</button><input id="search"><section id="admin-panel"><div class="admin-grid"></div></section>');
  await page.evaluate(()=>{
   window.$=id=>document.getElementById(id);
   window.node=(tag,text,cls)=>{const e=document.createElement(tag);if(text)e.textContent=text;if(cls)e.className=cls;return e;};
   window.chillReady=new Promise(()=>{});
   window.f={calls:[],polls:[],local:{installed:{mac:{version:"0.7.1"}},policies:{},available:{},operations:[{component:"mac",kind:"check",status:"running"}],busy:true}};
   window.setTimeout=(callback,delay)=>{f.polls.push({callback,delay});return f.polls.length;};
   window.__TAURI__={core:{invoke:async(name,{command})=>{
    f.calls.push({...command});
    if(command.action==="status")return structuredClone(f.local);
    if(command.action==="server_status")return new Promise(()=>{});
    if(command.action==="install")return new Promise((resolve,reject)=>{f.finishInstall=resolve;f.failInstall=reject;});
    throw Error("Unexpected action "+command.action);
   }}};
  });
  await page.addScriptTag({content:fs.readFileSync(path.join(__dirname,"../ui/connected/updates.js"),"utf8")});
  await page.waitForFunction(()=>f.polls.length===1);
  assert.equal(await page.evaluate(()=>f.polls[0].delay),10000,"A startup check polls without waiting for chillReady");
  assert.equal(await page.locator('#mac-update-badge').isHidden(),true,"A check alone does not advertise installation");
  assert.equal(await page.locator('[data-update-component="mac"] form').count(),0);
  await page.locator('#search').fill('usable while server is held');
  await page.evaluate(()=>{f.local.busy=false;f.local.available={mac:{version:"0.7.2"}};f.local.operations=[{component:"mac",kind:"check",status:"succeeded",result_code:"AVAILABLE"}];f.polls.shift().callback();});
  await page.locator('#mac-update-badge').waitFor({state:'visible'});
  assert.equal(await page.evaluate(()=>f.calls.filter(c=>c.action==='server_status').length),1,"A held server request is never duplicated and cannot block local refresh");
  const card=page.locator('[data-update-component="mac"]');
  // The WebView may have reloaded during a native installation. There is no
  // JavaScript in-flight flag left; the backend remains the authority.
  await page.evaluate(async()=>{f.local.busy=true;await chillUpdates.refresh();});
  assert.equal(await page.locator('#mac-update-badge').isDisabled(),true);
  assert.equal(await card.getByRole('button',{name:'Installer maintenant',exact:true}).isDisabled(),true);
  await page.locator('#mac-update-badge').evaluate(button=>button.onclick());
  assert.equal(await page.evaluate(()=>f.calls.filter(c=>c.action==='install').length),0);
  await page.evaluate(async()=>{f.local.busy=false;await chillUpdates.refresh();});
  await page.locator('#mac-update-badge').click();
  await card.getByRole('button',{name:'Installer maintenant',exact:true}).evaluate(button=>{button.onclick();button.onclick();});
  assert.equal(await page.evaluate(()=>f.calls.filter(c=>c.action==='install').length),1,"Badge and card share one install flight");
  assert.equal(await page.locator('#mac-update-badge').getAttribute('aria-label'),'Installation en cours…');
  await page.evaluate(()=>f.failInstall({message:'Paquet de test refusé'}));
  await page.waitForFunction(()=>document.querySelector('[data-update-component="mac"]').textContent.includes('Paquet de test refusé'));
  await page.evaluate(()=>chillUpdates.refresh());
  assert.match(await card.textContent(),/Paquet de test refusé/,'Polling must not erase the failure');
  assert.match(await page.locator('#mac-update-badge').getAttribute('aria-label'),/Réessayer/);
  await page.locator('#mac-update-badge').click();
  assert.equal(await page.evaluate(()=>f.calls.filter(c=>c.action==='install').length),2);
  assert.deepEqual(await page.evaluate(()=>f.calls.filter(c=>c.action==='install').at(-1)),{action:'install',component:'mac'});
  await page.evaluate(()=>{f.local.installed.mac.version='0.7.2';f.local.available={};f.local.operations=[{component:'mac',kind:'check',status:'succeeded',result_code:'NO_UPDATE'}];f.finishInstall({});});
  await page.locator('#mac-update-badge').waitFor({state:'hidden'});
  await page.evaluate(async()=>{f.local.available={mac:{version:'0.7.3'}};f.local.operations=[{component:'mac',kind:'check',status:'failed',result_code:'CHECK_FAILED'}];await chillUpdates.refresh();});
  assert.equal(await page.locator('#mac-update-badge').isHidden(),true,'Failed recheck cannot advertise stale availability');
  assert.equal(await card.getByRole('button',{name:'Installer maintenant',exact:true}).isHidden(),true);
  assert.deepEqual(errors,[]);
  console.log('PASS Mac updates: pending startup/server independence, fast polling, no policy, badge/card single install, persistent failure/retry, no stale badge. Synthetic IPC.');
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
