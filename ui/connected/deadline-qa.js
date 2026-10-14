window.chillDeadlineQa=async config=>{
 let step="boot";const assert=(ok,text)=>{if(!ok)throw Error(text);};
 const ui=async c=>{for(let i=0;i<20;i++){try{return await window.__TAURI__.core.invoke("ui_command",{command:c});}catch(e){if(e.code!=="CLIENT_BUSY"||i===19)throw e;await new Promise(r=>setTimeout(r,80));}}};
 try{await window.chillReady;if(state.stage!=="connected"){await connect(config.address);if(state.stage==="trust")await run({action:"approve",token:state.token});}
  const name="Deadline QA "+Date.now();state=await ui({action:"create_profile",name});render();await selectProfile(state.profiles.find(p=>p.name===name).id);
  const page=await ui({action:"catalog",offset:0,category:null,q:null,grouped:false});const m=page.items.find(m=>m.title.includes("Fixture"))||page.items[0];assert(m,"fixture");step="native load";await window.chillPlay(m.id);const end=Date.now()+30000;while(!playerState.native?.loaded||playerState.native.paused){if(Date.now()>end)throw Error("native load timeout");await new Promise(r=>setTimeout(r,80));}
  step="guard with buffering/UI/host blocked";const result=await window.__TAURI__.core.invoke("qa_deadline_probe");assert(!result.initially_paused&&result.paused&&!result.rendering&&result.ui_slot_held&&result.host_mutex_held,"deadline watchdog depends on rendering or bridge locks");await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"native-Tauri-independent-deadline-pause-rendering-off-ui-and-host-locks-held"});
 }catch(e){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+(e.message||e.code||e)});}
};
