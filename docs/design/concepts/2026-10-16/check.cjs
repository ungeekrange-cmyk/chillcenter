// Concept-only checks, no product, server or native-player claims.
const {chromium}=require('/private/tmp/marvin-cockpit.NmYyTL/package-manager/node_modules/playwright');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const path=require('node:path');
const fs=require('node:fs/promises');
(async()=>{const out=path.join(__dirname,'evidence');await fs.mkdir(out,{recursive:true});
const browser=await chromium.launch({executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run']});
try{const page=await browser.newPage({viewport:{width:1440,height:1120}});page.setDefaultTimeout(6000);page.setDefaultNavigationTimeout(10000);const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.route('http://**/*',r=>r.abort());await page.route('https://**/*',r=>r.abort());
await page.goto(pathToFileURL(path.join(__dirname,'events.html')).href);
for(const event of ['christmas','halloween'])for(const mode of ['dark','light']){
await page.locator('[data-event-choice="'+event+'"]').click();await page.locator('[data-mode-choice="'+mode+'"]').click();await page.locator('#scene').evaluate(img=>img.decode());
assert(await page.locator('#scene').evaluate(img=>img.naturalWidth>1000));assert.equal(await page.locator('#scene').evaluate(img=>getComputedStyle(img).objectPosition),'100% 50%');
if(event==='christmas')assert.equal(await page.locator('.side').evaluate(el=>getComputedStyle(el).backgroundColor),mode==='dark'?'rgb(112, 16, 25)':'rgb(186, 48, 40)');
assert.equal(await page.locator('.'+(event==='christmas'?'snow':'bats')+' i').count(),event==='christmas'?14:5);
await page.screenshot({path:path.join(out,event+'-'+mode+'.png'),fullPage:true});}
await page.locator('[data-width="440"]').click();await page.waitForFunction(()=>document.getElementById('app').getBoundingClientRect().width<=441&&document.getElementById('app').classList.contains('narrow'));assert.equal(await page.locator('.brand').evaluate(e=>Math.round(e.getBoundingClientRect().width)),45);
await page.screenshot({path:path.join(out,'halloween-narrow.png'),fullPage:true});
await page.locator('#animate').uncheck();assert.equal(await page.locator('.bats').isVisible(),false);
await page.emulateMedia({reducedMotion:'reduce'});await page.locator('#animate').check();assert.equal(await page.locator('.bats').isVisible(),false);
await page.emulateMedia({reducedMotion:'no-preference'});
await page.evaluate(()=>{Object.defineProperty(document,'hidden',{configurable:true,get:()=>true});document.dispatchEvent(new Event('visibilitychange'))});assert.equal(await page.locator('.bats i').first().evaluate(e=>getComputedStyle(e).animationPlayState),'paused');
for(const width of [320,390,760,1440]){await page.setViewportSize({width,height:1100});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'Theme overflow at '+width)}
console.log('Theme checks complete; starting filters.');await page.setViewportSize({width:1120,height:1000});await page.goto(pathToFileURL(path.join(__dirname,'filters-preview.html')).href,{waitUntil:'domcontentloaded'});const frame=page.frameLocator('iframe');
assert.equal(await frame.locator('.cc-film').count(),14);await frame.locator('#cc-genres').click();await frame.getByRole('button',{name:'Science-fiction',exact:true}).click();assert.equal(await frame.locator('.cc-film').count(),4);await frame.getByRole('button',{name:'Comédie',exact:true}).click();assert.equal(await frame.locator('.cc-film').count(),7);
assert.equal(await frame.locator('.cc-window').evaluate(e=>getComputedStyle(e).display),'grid');assert.equal(await frame.locator('.cc-side').evaluate(e=>getComputedStyle(e).backgroundColor),'rgb(23, 34, 28)');
await frame.locator('#cc-years').click();const values=await frame.locator('.cc-year').allTextContents();assert.deepEqual(values,[...values].sort((a,b)=>Number(b)-Number(a)));
await frame.getByRole('button',{name:'2025',exact:true}).click();assert.equal(await frame.locator('.cc-film').count(),1);await frame.getByRole('button',{name:'2018',exact:true}).click();assert.equal(await frame.locator('.cc-film').count(),4);assert.match(await frame.locator('#cc-year-n').innerText(),/2018–2025/);
await page.screenshot({path:path.join(out,'filters-range.png'),fullPage:true});await page.keyboard.press('Escape');assert.equal(await frame.locator('#cc-year-pop').isVisible(),false);
await frame.locator('#cc-years').click();await frame.getByRole('button',{name:'2020',exact:true}).click();assert.match(await frame.locator('#cc-year-n').innerText(),/2020/);await frame.locator('#cc-reset').click();assert.equal(await frame.locator('.cc-film').count(),14);
for(const width of [320,390,760,1120]){await page.setViewportSize({width,height:1000});await page.goto(pathToFileURL(path.join(__dirname,'filters-preview.html')).href,{waitUntil:'domcontentloaded'});await frame.locator('#cc-year-list button').first().waitFor({state:'attached'});await frame.locator('#cc-years').click();await frame.locator('#cc-year-pop').waitFor({state:'visible'});const b=await frame.locator('#cc-year-pop').boundingBox();assert(b&&b.x>=0&&b.x+b.width<=width,'Year panel bounds at '+width);await frame.locator('#cc-year-list button').first().focus();await page.keyboard.press('Escape');await frame.locator('#cc-year-pop').waitFor({state:'hidden'});}
await page.setViewportSize({width:736,height:850});await page.goto(pathToFileURL(path.join(__dirname,'sidebar-preview.html')).href,{waitUntil:'domcontentloaded'});const sidebar=page.frameLocator('iframe');
await sidebar.locator('.cc-snow i').first().waitFor({state:'attached'});assert.equal(await sidebar.locator('.cc-snow i').count(),14);assert.equal(await sidebar.locator('.cc-bats i').count(),5);await sidebar.locator('.cc-logo').first().evaluate(img=>img.decode());
assert.equal(await sidebar.locator('.cc-logo').first().evaluate(img=>img.complete&&img.naturalWidth>0),true);
const first=await sidebar.locator('.cc-snow i').first().evaluate(el=>getComputedStyle(el).transform);await sidebar.locator('.cc-snow i').first().evaluate((el,initial)=>new Promise(resolve=>{function sample(){if(getComputedStyle(el).transform!==initial)resolve();else requestAnimationFrame(sample)}sample()}),first);
assert.equal(await sidebar.locator('.cc-panel').first().evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(112, 16, 25)');assert.equal(await sidebar.locator('.cc-halloween').evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(53, 32, 20)');
await page.screenshot({path:path.join(out,'sidebar-dark.png'),fullPage:true});await sidebar.locator('button[data-mode="light"]').click();assert.equal(await sidebar.locator('.cc-panel').first().evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(186, 48, 40)');assert.equal(await sidebar.locator('.cc-halloween').evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(245, 212, 167)');await page.screenshot({path:path.join(out,'sidebar-light.png'),fullPage:true});
await sidebar.locator('#cc-sidebar-animate').uncheck();assert.equal(await sidebar.locator('.cc-snow').isVisible(),false);await sidebar.locator('#cc-sidebar-animate').check();await page.emulateMedia({reducedMotion:'reduce'});assert.equal(await sidebar.locator('.cc-bats').isVisible(),false);await page.emulateMedia({reducedMotion:'no-preference'});
await page.setViewportSize({width:320,height:1400});const inner=await page.locator('iframe').contentFrame();assert(await inner.locator('#chill-sidebar-events').evaluate(el=>el.scrollWidth<=el.clientWidth));
assert.deepEqual(errors,[]);console.log('PASS: 4 theme/mode renders; right anchor; responsive320–1440; animations off/reduced-motion/hidden; genre OR; descending years; inclusive range; third-year reset; Escape; responsive popovers; sidebar logos/motion/light-dark/mobile. No runtime integration exercised.');
}finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
