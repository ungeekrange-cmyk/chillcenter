window.chillPlayerQa=async config=>{
 if(config.mode==="next_series")return window.chillNextSeriesQa(config);
 if(config.mode&&config.mode.startsWith("transfer_"))return window.chillTransferQa(config);
 let step="boot",startedAt=Date.now();
 const assert=(ok,s)=>{if(!ok)throw Error(s);};
 const setStep=async next=>{step=next;await window.__TAURI__.core.invoke("qa_result",{success:false,detail:"TRACE player phase="+next+" elapsed_ms="+(Date.now()-startedAt)+" document_hidden="+document.hidden});};
 const wait=async(check,seconds=25)=>{const end=Date.now()+seconds*1000;while(!check()){if(Date.now()>end)throw Error("timeout: "+$("message").textContent+" / "+$("player-note").textContent);await new Promise(r=>setTimeout(r,60));}};
 const call=async cmd=>{await wait(()=>!playerBusy);playerBusy=true;try{let failure;for(let attempt=0;attempt<20;attempt++){try{const v=await window.__TAURI__.core.invoke("player_command",{command:cmd});playerState=v;renderPlayer();return v;}catch(e){failure=e;if(e.code!=="CLIENT_BUSY")break;await new Promise(r=>setTimeout(r,80));}}throw Error("player_command action="+cmd.action+" status="+(failure?.status??"none")+" code="+(failure?.code??"none"));}finally{playerBusy=false;}};
 try{
  await setStep("boot");
  await window.chillReady;
  if(state.stage!=="connected"){
   await setStep("connect");await connect(config.address);assert(state.stage==="trust","certificate consent required");await run({action:"approve",token:state.token});
   await setStep("profile");await run({action:"create_profile",name:"Native QA"});await selectProfile(state.profiles[0].id);
   await setStep("root");await run({action:"save_library",video_root:config.root,expected_revision:state.library.revision});await run({action:"start_scan"},false);
   await wait(()=>!busy);for(let i=0;i<100;i++){const s=await run({action:"scan_status"},false);if(s&&!s.running)break;await new Promise(r=>setTimeout(r,80));}
   await loadPage();
  }
  const catalog=await invoke({action:"catalog",offset:0,category:null,q:null});const m=catalog.items[0];assert(m?.id,"fixture catalog missing");
  await invoke({action:"set_watched",media_id:m.id,watched:false});
  await setStep("native load");await window.chillPlay(m.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);
  await wait(()=>playerState.native?.debug_nonblank===true);assert(playerState.native.tracks.some(t=>t.kind==="Audio"),"audio missing");assert(playerState.native.tracks.some(t=>t.kind==="Subtitle"),"subtitle missing");
  await setStep("native overlay hit-test");
  await wait(()=>playerState.native?.debug_surface?.webview_above_video&&playerState.native.debug_surface.webview_receives_pointer);
  showPlayerControls();assert(!$("player-panel").classList.contains("controls-hidden"),"overlay controls must be shown before hit-test");const pauseRect=$("player-pause").getBoundingClientRect();assert($("player-pause").contains(document.elementFromPoint(pauseRect.x+pauseRect.width/2,pauseRect.y+pauseRect.height/2)),"overlay DOM hit-test failed");
  await setStep("overlay inactivity");document.activeElement?.blur();playerHover=false;showPlayerControls();await wait(()=>$("player-panel").classList.contains("controls-hidden"),6);
  $("player-panel").dispatchEvent(new PointerEvent("pointermove",{bubbles:true}));assert(!$("player-panel").classList.contains("controls-hidden"),"pointer did not reveal overlay");
  $("player-audio").focus();await new Promise(r=>setTimeout(r,2700));assert(!$("player-panel").classList.contains("controls-hidden"),"focused menu hidden");$("player-audio").blur();
  await setStep("manual tracks persist pause/reload");const chosenAudio=playerState.native.tracks.find(t=>t.kind==="Audio"&&t.selected)||playerState.native.tracks.find(t=>t.kind==="Audio");await call({action:"audio",track_id:chosenAudio.id});await call({action:"subtitle",track_id:null});assert(playerState.native.tracks.filter(t=>t.kind==="Subtitle").every(t=>!t.selected),"subtitle off failed");await call({action:"pause"});assert(playerState.native.paused&&!playerState.active,"pause must release lease");await call({action:"resume"});await wait(()=>playerState.native?.loaded&&!playerState.native.paused);assert(playerState.native.tracks.some(t=>t.kind==="Audio"&&t.id===chosenAudio.id&&t.selected),"manual audio lost after reload");assert(playerState.native.tracks.filter(t=>t.kind==="Subtitle").every(t=>!t.selected),"manual subtitle off lost after reload");
  await setStep("near-zero seek");await call({action:"pause"});await call({action:"seek",position:0.5});assert(Math.abs(playerState.position-0.5)<0.2,"paused target not saved");await call({action:"resume"});await wait(()=>playerState.native?.loaded&&!playerState.native.paused);
  await setStep("navigation bounds");await call({action:"jump",seconds:-10});await wait(()=>playerState.position<2);await call({action:"jump",seconds:30});await wait(()=>playerState.position>29&&playerState.position<34);
  await setStep("speed");await call({action:"speed",speed:1.5});assert(playerState.native.speed===1.5,"speed failed");await call({action:"speed",speed:1});
  await setStep("volume");await call({action:"volume",volume:37});assert(playerState.native.volume===37,"volume failed");await call({action:"volume",volume:100});
  await setStep("fullscreen geometry");const normal=videoRect();await call({action:"fullscreen",enabled:true});await wait(()=>playerState.native?.fullscreen);await new Promise(r=>setTimeout(r,500));await call({action:"mount",rect:videoRect()});
  const full=playerState.native.debug_surface;assert(full.webview_above_video&&full.webview_receives_pointer,"fullscreen overlay native order lost");
  assert(Math.abs(full.frame.width-full.bounds.width)<2&&Math.abs(full.frame.height-full.bounds.height)<2&&Math.abs(full.frame.x)<2&&Math.abs(full.frame.y)<2,"native fullscreen surface does not fill content bounds");
  const safe=full.screen_safe_bounds;
  assert(safe&&Math.abs(full.frame.width-safe.width)<3&&Math.abs(full.frame.height-safe.height)<3,"native fullscreen surface does not fill screen safe area "+JSON.stringify({frame:full.frame,safe,screen:full.screen}));
  assert(videoRect().width>=window.innerWidth-2&&videoRect().height>=window.innerHeight-2,"DOM fullscreen cutout constrained");
  await window.__TAURI__.core.invoke("qa_result",{success:false,detail:"TRACE fullscreen native "+JSON.stringify(full)});
  await setStep("Escape restores window");document.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}));await wait(()=>!playerState.native?.fullscreen&&!document.body.classList.contains("player-fullscreen"));await new Promise(r=>setTimeout(r,500));await call({action:"mount",rect:videoRect()});assert(Math.abs(videoRect().width-normal.width)<3&&Math.abs(videoRect().height-normal.height)<3,"normal surface not restored");
  await setStep("final window watch");await call({action:"seek",position:playerState.duration-12});await wait(()=>playerState.native?.ended,25);
  assert(!playerState.native.error,"unexpected EOF error");
  await call({action:"stop"});await loadPage();const updated=await invoke({action:"catalog",offset:0,category:null,q:null});assert(updated.items.find(x=>x.id===m.id).watched,"ten real seconds did not mark watched");
  await setStep("reopen");await window.chillPlay(m.id);await wait(()=>playerState.native?.loaded);await call({action:"stop"});
  await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"native-image-AppKit-z-order-hit-test-overlay-idle-focus-volume-fullscreen-geometry-Escape-tracks-resume-watched-EOF"});
 }catch(e){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+e.message});}
};
window.chillNextSeriesQa=async config=>{
 let step="boot",original=[],outcome={success:false,detail:"boot"},startedAt=Date.now();const assert=(x,s)=>{if(!x)throw Error(s)};const setStep=async next=>{step=next;await window.__TAURI__.core.invoke("qa_result",{success:false,detail:"TRACE player-next phase="+next+" elapsed_ms="+(Date.now()-startedAt)+" document_hidden="+document.hidden});};const wait=async f=>{const end=Date.now()+30000;while(!f()){if(Date.now()>end)throw Error("timeout "+step);await new Promise(r=>setTimeout(r,80));}};const call=async c=>{let failure;for(let i=0;i<20;i++){try{const v=await window.__TAURI__.core.invoke("player_command",{command:c});playerState=v;renderPlayer();return v}catch(e){failure=e;if(e.code!=="CLIENT_BUSY")break;await new Promise(r=>setTimeout(r,80));}}throw Error("player_command action="+c.action+" status="+(failure?.status??"none")+" code="+(failure?.code??"none"));};
 try{await setStep("boot");await window.chillReady;if(state.stage!=="connected"){await setStep("connect");await connect(config.address);if(state.stage==="trust")await run({action:"approve",token:state.token});}await setStep("profile");const name="Next QA "+Date.now();await run({action:"create_profile",name});const profile=state.profiles.find(p=>p.name===name);assert(profile,"synthetic profile missing");await selectProfile(profile.id);await setStep("library");await run({action:"save_library",video_root:config.root,expected_revision:state.library.revision});await run({action:"start_scan"},false);for(let i=0;i<100;i++){const s=await invoke({action:"scan_status"});if(!s.running)break;await new Promise(r=>setTimeout(r,80));}
  const p=await invoke({action:"catalog",offset:0,category:null,q:null});const [a,b]=p.items;assert(a&&b,"two fixtures");for(const [m,season] of [[a,1],[b,2]]){const d=await invoke({action:"media_detail",media_id:m.id});original.push(d);await invoke({action:"edit_media",media_id:m.id,expected_revision:d.revision,patch:{category:"serie",series:"QA Saison",season,episode:1}})}
  await setStep("series speed");await window.chillPlay(a.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);await call({action:"speed",speed:1.5});await call({action:"stop"});await window.chillPlay(b.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);assert(playerState.native.speed===1.5,"series speed not persisted");
  await setStep("next banner");await call({action:"stop"});await window.chillPlay(a.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);await call({action:"seek",position:playerState.duration-12});await wait(()=>playerState.native?.ended);assert(playerState.next?.new_season===true,"new season missing");assert(!$("player-next").hidden&&$("player-next-season").textContent==="Nouvelle saison","banner missing");$("player-next-cancel").click();assert($("player-next-count").textContent.includes("annulé"),"cancel missing");await call({action:"next",expected_id:b.id});await wait(()=>playerState.native?.loaded&&!playerState.native.paused);await call({action:"stop"});outcome={success:true,detail:"next-now-cancel-new-season-series-speed"};
 }catch(e){outcome={success:false,detail:step+": "+e.message};}finally{for(const d of original){try{const m=d.media;await invoke({action:"edit_media",media_id:m.id,expected_revision:d.revision+1,patch:{category:m.category,series:m.series,season:m.season,episode:m.episode}})}catch(_){outcome={success:false,detail:"restore failed"};}}await window.__TAURI__.core.invoke("qa_result",outcome);}
};
window.chillTransferQa=async config=>{
 let step="boot",startedAt=Date.now();const assert=(ok,s)=>{if(!ok)throw Error(s);};const setStep=async next=>{step=next;await window.__TAURI__.core.invoke("qa_result",{success:false,detail:"TRACE player-transfer phase="+next+" elapsed_ms="+(Date.now()-startedAt)+" document_hidden="+document.hidden});};
 const wait=async(check,seconds=35)=>{const end=Date.now()+seconds*1000;while(!check()){if(Date.now()>end)throw Error("timeout "+step+" / "+$("player-note").textContent);await new Promise(r=>setTimeout(r,80));}};
 const call=async cmd=>{for(let i=0;i<20;i++){try{const value=await window.__TAURI__.core.invoke("player_command",{command:cmd});playerState=value;renderPlayer();return value;}catch(e){if(e.code!=="CLIENT_BUSY"||i===19)throw Error("player_command action="+cmd.action+" status="+(e.status??"none")+" code="+(e.code??"none"));await new Promise(r=>setTimeout(r,80));}}};
 try{
  await setStep("boot");await window.chillReady;
  await setStep("connect");if(state.stage!=="connected"){await connect(config.address);if(state.stage==="trust")await run({action:"approve",token:state.token});}
  let profile=state.profiles.find(p=>p.name==="Transfer QA");
  if(!profile&&config.mode==="transfer_a"){await setStep("profile");await run({action:"create_profile",name:"Transfer QA"});profile=state.profiles.find(p=>p.name==="Transfer QA");}
  assert(profile,"Transfer QA profile absent: start device A first");await selectProfile(profile.id);
  if(config.mode==="transfer_a"){await setStep("library");await run({action:"save_library",video_root:config.root,expected_revision:state.library.revision});await run({action:"start_scan"},false);for(let i=0;i<150;i++){const s=await invoke({action:"scan_status"});if(!s.running)break;await new Promise(r=>setTimeout(r,80));}}
  await setStep("catalog");const page=await invoke({action:"catalog",offset:0,category:null,q:null});const media=page.items.find(x=>x.title.includes("Fixture"))||page.items[0];assert(media?.id,"fixture catalog missing");
  await setStep("start");await window.chillPlay(media.id);if(config.mode!=="transfer_a"){await setStep("transfer offer");await wait(()=>playerState.stage==="transfer");}
  const initial=playerState.position||0;
  if(config.mode==="transfer_a"){
   await wait(()=>playerState.native?.loaded&&!playerState.native.paused);await wait(()=>playerState.native?.debug_nonblank===true);
   await setStep("await confirmed transfer");await wait(()=>playerState.native?.paused===true&&!playerState.active,55);assert((playerState.position||0)>=initial,"A final position regressed");await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"transfer-A-SSE-native-pause-final-ACK"});return;
  }
  if(config.mode==="transfer_refuse"){await call({action:"refuse_transfer"});assert(playerState.stage!=="transfer","refusal did not close proposal");await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"transfer-B-refused-no-forced-takeover"});return;}
  await setStep("confirm");await call({action:"confirm_transfer"});await wait(()=>playerState.native?.loaded&&!playerState.native.paused,40);await wait(()=>playerState.native?.debug_nonblank===true);assert((playerState.position||0)>0.5,"B did not retain A resume position");await call({action:"stop"});await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"transfer-B-confirmed-native-image-resume"});
 }catch(e){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+e.message});}
};
