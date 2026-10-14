// Debug-only harness. Native file chooser seam uses its private QA directory;
// this proves Tauri/pinned streaming, not OS dialog interaction.
window.chillArchiveQa=async config=>{
  let step="connect";const assert=(ok,text)=>{if(!ok)throw Error(text);};
  const native=command=>window.__TAURI__.core.invoke("admin_command",{command});
  const waitJob=async(id,stage)=>{const end=Date.now()+30000;while(Date.now()<end){const job=await native({action:"status",id});if(job.stage===stage)return job;if(job.stage==="failed"||job.stage==="cancelled")throw Error(job.error||job.stage);await new Promise(r=>setTimeout(r,75));}throw Error("timeout "+stage);};
  try{
    await window.chillReady;if(state.stage!=="connected"){await connect(config.address);if(state.stage==="trust")await run({action:"approve",token:state.token});}
    const name="Archives QA "+Date.now();await run({action:"create_profile",name});const profile=state.profiles.find(p=>p.name===name);assert(profile,"profile created");await selectProfile(profile.id);
    const before=(await invoke({action:"refresh"})).profiles.length;
    step="native export background/pinned file save";const started=await native({action:"export"});const ready=await waitJob(started.id,"ready");assert(ready.result.bytes>0,"empty export");await native({action:"save",id:started.id});await waitJob(started.id,"saved");
    step="native streamed import preview";const uploaded=await native({action:"import"});const preview=(await waitJob(uploaded.id,"preview")).result;assert(preview.counts.media>=2&&preview.profiles.length===before,"preview scope");assert((await invoke({action:"refresh"})).profiles.length===before,"preview wrote database");
    step="explicit complete mapping / no plan mutation";const map={};for(let i=0;i<preview.profiles.length;i++)map[preview.profiles[i].id]={kind:"new",name:"Import QA "+Date.now()+" "+i};const mapping={profile_map:map,root_prefix:"",conflict_policy:"keep_destination"};await native({action:"plan",id:uploaded.id,mapping});const plan=await waitJob(uploaded.id,"planned");assert(plan.result.plan_id,"missing plan");assert((await invoke({action:"refresh"})).profiles.length===before,"plan wrote database");
    step="confirmed atomic import";await native({action:"apply",id:uploaded.id,confirmed:true});const applied=await waitJob(uploaded.id,"applied");assert(!applied.result.already_imported,"unexpected receipt");assert((await invoke({action:"refresh"})).profiles.length===before+preview.profiles.length,"profiles mapping lost");
    step="stale profile-bound job";await selectProfile(state.profiles.find(p=>p.id!==profile.id).id);let rejected=false;try{await native({action:"plan",id:uploaded.id,mapping});}catch(e){rejected=e.code==="ADMIN_STALE";}assert(rejected,"old-profile plan accepted");
    await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"native-Tauri-pinned-ZIP-export-preview-map-confirm-profile-invalidation"});
  }catch(error){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+(error.message||JSON.stringify(error))});}
};
