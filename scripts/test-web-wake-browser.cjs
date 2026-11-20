// Real Chromium video and Screen Wake Lock on a loopback secure context.
// Synthetic canvas stream only: no household profile, server, media or history.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const {chromium}=require(process.env.CHILL_PREVIEW_NODE_MODULES?process.env.CHILL_PREVIEW_NODE_MODULES+'/playwright':'playwright');
(async()=>{
 const root=path.resolve(__dirname,'../web/dist');
 const server=http.createServer((req,res)=>{
  if(req.url==='/'){res.setHeader('Content-Type','text/html');return res.end('<!doctype html><video muted playsinline></video><canvas width="64" height="64"></canvas><script src="/watch-clock.js"></script><script src="/player-web.js"></script>');}
  if(!['/watch-clock.js','/player-web.js'].includes(req.url)){res.statusCode=404;return res.end();}
  res.setHeader('Content-Type','text/javascript');res.end(fs.readFileSync(path.join(root,req.url.slice(1))));
 });
 let browser;
 try {
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  browser=await chromium.launch({executablePath:process.env.CHILL_CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:['--disable-background-networking','--no-first-run','--autoplay-policy=no-user-gesture-required']});
  const page=await browser.newPage();await page.goto('http://127.0.0.1:'+server.address().port);
  const evidence=await page.evaluate(async()=>{
   if(!isSecureContext||!navigator.wakeLock)throw Error('Real Screen Wake Lock API unavailable in test browser');
   const grants=[],nativeRequest=navigator.wakeLock.request.bind(navigator.wakeLock);
   const player=new ChillPlayerCore.WebPlayer({wakeLock:{async request(type){const s=await nativeRequest(type);grants.push(s);return s;}},now:()=>performance.now(),document,page:window,request:async()=>({}),clearInterval(){}});
   if(player.wake.active)throw Error('Opening acquired wake lock');
   const video=document.querySelector('video'),canvas=document.querySelector('canvas'),ctx=canvas.getContext('2d');
   const frames=setInterval(()=>{ctx.fillStyle='teal';ctx.fillRect(0,0,64,64);},40);
   video.srcObject=canvas.captureStream(25);await video.play();
   player.engine={video,snapshot:()=>({loaded:video.readyState>=2,paused:video.paused,ended:video.ended,buffering:false,seeking:false,error:null,position:video.currentTime,duration:60,speed:1}),pause:()=>video.pause(),close(){video.pause();video.srcObject.getTracks().forEach(t=>t.stop());}};
   player.ctx={profile:'fixture'};player.info={media:{title:'Synthetic video'}};player.context={kind:'standalone'};player.lease={session_id:'fixture',generation:1,media_id:'fixture',media_revision:1};
   const until=async predicate=>{const end=performance.now()+6000;while(!predicate()){if(performance.now()>end)throw Error('Timed out waiting for wake lock transition');await player.status();await new Promise(r=>setTimeout(r,50));}};
   try {
    await until(()=>grants.length===1&&player.wake.lock===grants[0]);
    if(grants[0].released)throw Error('No actual screen protection');
    await grants[0].release();
    await until(()=>grants.length===2&&player.wake.lock===grants[1]);
    if(video.paused||grants[1].released)throw Error('Recovery interrupted playback');
    video.pause();await until(()=>grants[1].released);
    const afterPause=grants.length;await new Promise(r=>setTimeout(r,1200));await player.status();
    if(grants.length!==afterPause)throw Error('Paused video reacquired wake lock');
    await video.play();await until(()=>grants.length===3&&player.wake.lock===grants[2]);
    await player.stop(false);await until(()=>grants[2].released);
    return {secure_context:isSecureContext,actual_api:true,actual_video:true,grants:grants.length,recovered_without_pausing:true,pause_released:true,resume_reacquired:true,stop_released:true};
   } finally {clearInterval(frames);video.srcObject?.getTracks().forEach(t=>t.stop());}
  });
  assert.equal(evidence.grants,3);console.log('PASS real Chromium wake lock: '+JSON.stringify(evidence));
 } finally {await browser?.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
