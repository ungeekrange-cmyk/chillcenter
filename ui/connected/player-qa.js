window.chillPlayerQa=async config=>{
 let step="boot";
 const assert=(ok,s)=>{if(!ok)throw Error(s);};
 const wait=async(check,seconds=25)=>{const end=Date.now()+seconds*1000;while(!check()){if(Date.now()>end)throw Error("timeout: "+$("message").textContent+" / "+$("player-note").textContent);await new Promise(r=>setTimeout(r,60));}};
 const call=async cmd=>{await wait(()=>!playerBusy);playerBusy=true;try{let failure;for(let attempt=0;attempt<20;attempt++){try{const v=await window.__TAURI__.core.invoke("player_command",{command:cmd});playerState=v;renderPlayer();return v;}catch(e){failure=e;if(e.code!=="CLIENT_BUSY")break;await new Promise(r=>setTimeout(r,80));}}throw Error("player_command action="+cmd.action+" status="+(failure?.status??"none")+" code="+(failure?.code??"none"));}finally{playerBusy=false;}};
 try{
  await window.chillReady;
  if(state.stage!=="connected"){
   step="connect";await connect(config.address);assert(state.stage==="trust","certificate consent required");await run({action:"approve",token:state.token});
   step="profile";await run({action:"create_profile",name:"Native QA"});await selectProfile(state.profiles[0].id);
   step="root";await run({action:"save_library",video_root:config.root,expected_revision:state.library.revision});await run({action:"start_scan"},false);
   await wait(()=>!busy);for(let i=0;i<100;i++){const s=await run({action:"scan_status"},false);if(s&&!s.running)break;await new Promise(r=>setTimeout(r,80));}
   await loadPage();
  }
  const catalog=await invoke({action:"catalog",offset:0,category:null,q:null});const m=catalog.items[0];assert(m?.id,"fixture catalog missing");
  await invoke({action:"set_watched",media_id:m.id,watched:false});
  step="native load";await window.chillPlay(m.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);
  await wait(()=>playerState.native?.debug_nonblank===true);assert(playerState.native.tracks.some(t=>t.kind==="Audio"),"audio missing");assert(playerState.native.tracks.some(t=>t.kind==="Subtitle"),"subtitle missing");
  step="manual tracks persist pause/reload";const chosenAudio=playerState.native.tracks.find(t=>t.kind==="Audio"&&t.selected)||playerState.native.tracks.find(t=>t.kind==="Audio");await call({action:"audio",track_id:chosenAudio.id});await call({action:"subtitle",track_id:null});assert(playerState.native.tracks.filter(t=>t.kind==="Subtitle").every(t=>!t.selected),"subtitle off failed");await call({action:"pause"});assert(playerState.native.paused&&!playerState.active,"pause must release lease");await call({action:"resume"});await wait(()=>playerState.native?.loaded&&!playerState.native.paused);assert(playerState.native.tracks.some(t=>t.kind==="Audio"&&t.id===chosenAudio.id&&t.selected),"manual audio lost after reload");assert(playerState.native.tracks.filter(t=>t.kind==="Subtitle").every(t=>!t.selected),"manual subtitle off lost after reload");
  step="near-zero seek";await call({action:"pause"});await call({action:"seek",position:0.5});assert(Math.abs(playerState.position-0.5)<0.2,"paused target not saved");await call({action:"resume"});await wait(()=>playerState.native?.loaded&&!playerState.native.paused);
  step="navigation bounds";await call({action:"jump",seconds:-10});await wait(()=>playerState.position<2);await call({action:"jump",seconds:30});await wait(()=>playerState.position>29&&playerState.position<34);
  step="speed";await call({action:"speed",speed:1.5});assert(playerState.native.speed===1.5,"speed failed");await call({action:"speed",speed:1});
  step="fullscreen";await call({action:"fullscreen"});await new Promise(r=>setTimeout(r,800));await call({action:"mount",rect:videoRect()});await call({action:"fullscreen"});await new Promise(r=>setTimeout(r,800));await call({action:"mount",rect:videoRect()});
  step="final window watch";await call({action:"seek",position:playerState.duration-12});await wait(()=>playerState.native?.ended,25);
  assert(!playerState.native.error,"unexpected EOF error");
  await call({action:"stop"});await loadPage();const updated=await invoke({action:"catalog",offset:0,category:null,q:null});assert(updated.items.find(x=>x.id===m.id).watched,"ten real seconds did not mark watched");
  step="reopen";await window.chillPlay(m.id);await wait(()=>playerState.native?.loaded);await call({action:"stop"});
  await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"native-Tauri-image-controls-tracks-fullscreen-resume-watched-EOF"});
 }catch(e){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+e.message});}
};
