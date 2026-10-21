// Optional isolated visual QA. Uses the already-installed Playwright test runtime.
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const path=require('node:path');
const {pathToFileURL}=require('node:url');
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES
 ?path.join(process.env.CHILL_PREVIEW_NODE_MODULES,'playwright'):'playwright');

(async()=>{
 const evidence=process.env.CHILL_RABBIT_EVIDENCE;
 assert.ok(evidence&&path.isAbsolute(evidence),'Set absolute CHILL_RABBIT_EVIDENCE');
 await fs.mkdir(evidence,{recursive:true});
 const browser=await chromium.launch({headless:true,
  ...(process.env.CHILL_PREVIEW_BROWSER?{executablePath:process.env.CHILL_PREVIEW_BROWSER}:{}),
  args:['--disable-background-networking']});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.goto(pathToFileURL(path.join(__dirname,'events.html')).href);
  await page.locator('.rabbit-walk img').evaluateAll(images=>Promise.all(images.map(image=>image.decode())));
  const svg=await fs.readFile(path.join(__dirname,'assets/rabbit-silhouette.svg'));
  const pixels=await page.evaluate(async source=>{
   const image=new Image();image.src=source;await image.decode();
   const canvas=document.createElement('canvas');canvas.width=80;canvas.height=64;
   const context=canvas.getContext('2d');context.drawImage(image,0,0,80,64);
   const rgba=context.getImageData(0,0,80,64).data,solid=new Set(),mask=new Set();let transparent=0;
   for(let i=0;i<80*64;i++){
    if(rgba[i*4+3]===255)solid.add([...rgba.slice(i*4,i*4+3)].join(','));
    if(rgba[i*4+3]>127)mask.add(i);else if(rgba[i*4+3]===0)transparent++;
   }
   let components=0;
   while(mask.size){components++;const queue=[mask.values().next().value];mask.delete(queue[0]);
    for(let index=0;index<queue.length;index++){
     const pixel=queue[index],x=pixel%80,y=Math.floor(pixel/80);
     for(const [dx,dy]of [[1,0],[-1,0],[0,1],[0,-1]]){
      const nx=x+dx,ny=y+dy,n=ny*80+nx;
      if(nx>=0&&nx<80&&ny>=0&&ny<64&&mask.delete(n))queue.push(n);
     }
    }
   }
   return{colors:[...solid],components,transparent};
  },'data:image/svg+xml;base64,'+svg.toString('base64'));
  // SVG antialias compositing rounds channels by one level at overlaps/edges.
  // The source has exactly one fill; no secondary artwork color is permitted.
  assert.ok(pixels.colors.every(color=>color.split(',').every((v,i)=>Math.abs(Number(v)-[124,137,102][i])<=1)));
  assert.equal(pixels.components,1,'one connected silhouette');assert.ok(pixels.transparent>0);
  for(const mode of ['light','dark']){
   if(mode==='dark')await page.locator('#mode').click();
   await page.evaluate(()=>{for(const a of document.getAnimations()){a.pause();
    if(a.animationName==='rabbit-roam')a.currentTime=a.effect.target.matches(':nth-of-type(2)')?0:8000;}});
   for(const phase of [0,350,700,1050,1400]){
    const poses=await page.evaluate(time=>{
     for(const a of document.getAnimations())if(a.animationName.startsWith('rabbit-hop'))a.currentTime=time;
     return[...document.querySelectorAll('.rabbit-walk img')].map(image=>{
      const box=image.getBoundingClientRect(),m=new DOMMatrix(getComputedStyle(image).transform);
      return{width:box.width,height:box.height,a:m.a,b:m.b,c:m.c,d:m.d,y:m.f};
     });
    },phase);
    for(const [index,pose]of poses.entries()){
     assert.ok(Math.abs(pose.width-64)<.05&&Math.abs(pose.height-51.2)<.05);
     assert.equal(pose.a,index===0?1:-1);assert.equal(pose.d,1);
     assert.equal(pose.b,0);assert.equal(pose.c,0);assert.ok(pose.y>=-3.01&&pose.y<=.01);
    }
   }
   await page.screenshot({path:path.join(evidence,`easter-solid-${mode}.png`),fullPage:true});
  }
  await page.locator('#valentine').click();
  assert.equal(await page.locator('.rabbit-walk:visible').count(),0);
  await page.emulateMedia({reducedMotion:'reduce'});await page.locator('#easter').click();
  const stopped=await page.locator('.rabbit-walk,.rabbit-walk img').evaluateAll(nodes=>nodes.every(n=>getComputedStyle(n).animationName==='none'));
  assert.equal(stopped,true);assert.deepEqual(errors,[]);
  await page.goto(pathToFileURL(path.join(__dirname,'rabbit-comparison.html')).href);
  await page.screenshot({path:path.join(evidence,'rabbit-comparison.png'),fullPage:true});
  console.log(JSON.stringify({status:'PASS',...pixels,poseChecks:20,reducedMotion:true,valentineUnchanged:true}));
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1});
