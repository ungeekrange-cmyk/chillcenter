window.chillFormatsQa=async config=>{
 let step="boot";const assert=(ok,s)=>{if(!ok)throw Error(s);};
 const trace=detail=>window.__TAURI__.core.invoke("qa_result",{success:false,detail:"TRACE "+detail});
 const sleep=ms=>new Promise(r=>setTimeout(r,ms));const wait=async f=>{const end=Date.now()+30000;while(!f()){if(Date.now()>end)throw Error("timeout "+step);await sleep(80);}};
 const ui=async c=>{for(let i=0;i<20;i++){try{return await window.__TAURI__.core.invoke("ui_command",{command:c});}catch(e){if(e.code!=="CLIENT_BUSY"||i===19)throw e;await sleep(80);}}};
 const player=async c=>{await trace("command "+c.action+" "+(c.speed??""));await wait(()=>!playerBusy);playerBusy=true;try{for(let i=0;i<20;i++){try{playerState=await Promise.race([window.__TAURI__.core.invoke("player_command",{command:c}),sleep(10000).then(()=>{throw Error("invoke timeout "+c.action);})]);renderPlayer();return playerState;}catch(e){if(e.code!=="CLIENT_BUSY"||i===19)throw e;await sleep(80);}}}finally{playerBusy=false;}};
 try{await window.chillReady;if(state.stage!=="connected"){await connect(config.address);if(state.stage==="trust")await run({action:"approve",token:state.token});}
  const name="Formats QA "+Date.now();state=await ui({action:"create_profile",name});render();await selectProfile(state.profiles.find(p=>p.name===name).id);await ui({action:"start_scan"});for(let i=0;i<150;i++){if(!(await ui({action:"scan_status"})).running)break;await sleep(80);}
  await trace("scan complete");const p=await ui({action:"catalog",offset:0,category:null,q:null,grouped:false});
  for(const file of ["Tracks (2026).mkv","Fixture (2026).mp4","Second (2026).avi"]){
   const m=p.items.find(x=>x.path.endsWith(file));assert(m,"fixture missing: "+file);step=file+" native load";await trace(step);await window.chillPlay(m.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);await wait(()=>playerState.native?.debug_nonblank);assert(!playerState.native.error,"decode error");await trace("native image "+file);
   if(file.endsWith(".mkv")){
    step="MKV audio and ASS";await trace(step);const audio=playerState.native.tracks.filter(t=>t.kind==="Audio"),subs=playerState.native.tracks.filter(t=>t.kind==="Subtitle");assert(audio.length>=2&&subs.length>=1,"integrated tracks missing");assert(audio.some(t=>t.selected&&/^(eng|en)$/.test(t.language)),"English audio default missing");await player({action:"audio",track_id:audio.find(t=>t.language==="fra").id});await wait(()=>playerState.native.tracks.some(t=>t.kind==="Audio"&&t.language==="fra"&&t.selected));
    const external=playerState.external_subtitles.find(s=>/fra|fr/.test(s.language));assert(external&&!external.unavailable,"external ASS missing");await player({action:"external_subtitle",id:external.id});await wait(()=>playerState.native.tracks.some(t=>t.kind==="Subtitle"&&t.selected));
   }
   step=file+" navigation and rates";await trace(step);await player({action:"seek",position:3});await wait(()=>playerState.position>=2&&playerState.position<8);
   for(const speed of [0.5,0.75,1,1.25,1.5,1.75,2]){const r=await player({action:"speed",speed});assert(r.native.speed===speed,"speed "+speed);}
   await player({action:"stop"});hidePlayer();
   if(m.category!=="serie"){await window.chillPlay(m.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);assert(playerState.native.speed===1,"film must reopen at 1x");await player({action:"stop"});hidePlayer();}
  }
  const tracksMedia=p.items.find(x=>x.path.endsWith("Tracks (2026).mkv"));
  for(const language of ["ja","fr"]){
   step="language fallback "+language;await trace(step);
   state=await ui({action:"save_preferences",audio_language:language,subtitle_language:language,autoplay_seconds:null});render();
   await window.chillPlay(tracksMedia.id);await wait(()=>playerState.native?.loaded&&!playerState.native.paused);await wait(()=>playerState.native?.debug_nonblank);
   const n=playerState.native,audio=n.tracks.filter(t=>t.kind==="Audio"),subtitles=n.tracks.filter(t=>t.kind==="Subtitle");
   assert(!subtitles.some(t=>t.selected),"absent or ambiguous subtitle must not auto-select");
   if(language==="ja"){
    const fallback=audio.find(t=>t.original)||audio[0];assert(audio.find(t=>t.selected)?.id===fallback.id,"missing audio preference must use original or first track");
    assert((n.notice||playerState.note||"").includes("indisponible"),"missing preferred language notice absent");
   }else{
    assert(audio.some(t=>t.selected&&/^(fra|fr)$/.test(t.language)),"French audio preference not selected");
    assert(playerState.external_subtitles.filter(t=>!t.unavailable&&/^(fra|fr)$/.test(t.language)).length>=2,"ambiguity fixture missing");
    assert((playerState.note||"").includes("Plusieurs sous-titres"),"ambiguous external tracks must request manual choice");
   }
   await player({action:"stop"});hidePlayer();
  }
  await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"native-Tauri-MKV-MP4-AVI-integrated-audio-ASS-external-ASS-seven-rates-seek-film-reset-language-fallback-ambiguous-subtitles"});
 }catch(e){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+(e.message||e.code||e)});}
};
