// Invoked only by the debug host with an explicitly isolated fixture directory.
window.chillQaSmoke=async config=>{
  let step="startup";
  const assert=(ok,text)=>{if(!ok)throw Error(text);};
  const wait=async check=>{const deadline=Date.now()+20000;while(!check()){if(Date.now()>deadline)throw Error("timeout at "+step+": "+$("message").textContent);await new Promise(r=>setTimeout(r,40));}};
  const settled=async check=>wait(()=>!busy&&check());
  const create=async name=>{const form=$("create-admin");form.elements.name.value=name;form.requestSubmit();await settled(()=>state.profiles.some(p=>p.name===name));};
  try {
    await window.chillReady;
    if(config.mode==="renewal_notice"){
      step="automatic expired-key replacement notice";
      assert(state.stage==="connected"&&state.notice?.includes("acceptée automatiquement"),"replacement notice missing in connected view");
      assert(!$("connection-notice").hidden&&$("connection-notice").textContent===state.notice,"visible notice missing");
      await loadPage();await settled(()=>document.querySelectorAll(".media-card").length===1);
      assert(!$("connection-notice").hidden,"catalog erased trust notice");
      $("admin-toggle").click();await settled(()=>!$("admin-panel").hidden);
      assert(!$("connection-notice").hidden,"refresh erased trust notice");
    } else if(config.mode==="reopen_delete"){
      step="remembered profile";assert(state.stage==="connected"&&state.profiles.find(p=>p.id===state.profile_id)?.name==="QA B","last profile not restored");
      assert(document.querySelectorAll(".media-card").length===1,"catalog not restored");
      $("admin-toggle").click();await settled(()=>!$("admin-panel").hidden);
      let row=Array.from(document.querySelectorAll(".profile-row")).find(r=>r.querySelector("input").value==="QA A2");
      step="cancel deletion";row.querySelectorAll("button")[1].click();$("cancel-delete").click();assert(state.profiles.length===2,"cancel changed profiles");
      step="delete other profile";row.querySelectorAll("button")[1].click();$("confirm-delete-button").click();await settled(()=>state.profiles.length===1);
      step="delete last profile";document.querySelector(".profile-row button.danger").click();$("confirm-delete-button").click();await settled(()=>state.profiles.length===0&&!state.profile_id);
      assert(!$("profile-panel").hidden&&!$("admin-panel").hidden,"zero profiles blocks setup");
      await create("QA C");$("admin-close").click();document.querySelector(".profile-choice").click();await settled(()=>document.querySelectorAll(".media-card").length===1);
    } else {
      step="first connection";$("address").value=config.address;$("connect-form").requestSubmit();await settled(()=>state.stage==="trust");
      assert($("fingerprint").textContent.length>30,"fingerprint missing");
      step="refuse consent";$("reject-trust").click();await settled(()=>state.stage==="server");
      $("address").value=config.address;$("connect-form").requestSubmit();await settled(()=>state.stage==="trust");
      step="approve consent";$("approve-trust").click();await settled(()=>state.stage==="connected");assert(!state.profile_id&&state.profiles.length===0,"first profile not explicit");
      step="library root";$("admin-toggle").click();await settled(()=>!$("admin-panel").hidden);$("video-root").value=config.root;$("library-form").requestSubmit();await settled(()=>state.library.video_root===config.root);
      step="create A";await create("QA A");$("admin-close").click();document.querySelector(".profile-choice").click();await settled(()=>Boolean(state.profile_id));
      step="scan";await wait(()=>!busy);$("admin-toggle").click();await settled(()=>!$("admin-panel").hidden);$("scan").click();await settled(()=>document.querySelectorAll(".media-card").length===1);
      step="personal state A";document.querySelector(".media-card .watched-toggle").click();await settled(()=>document.querySelector(".media-card .watched-toggle")?.textContent==="Vu ✓");
      const rate=document.querySelector(".media-card select");rate.value="5";rate.dispatchEvent(new Event("change"));await settled(()=>document.querySelector(".media-card select")?.value==="5");
      $("audio-language").value="fr";$("autoplay").value="off";$("preferences-form").requestSubmit();await settled(()=>state.preferences.audio_language==="fr"&&state.preferences.autoplay_seconds===null);
      step="rename A";const row=document.querySelector(".profile-row");row.querySelector("input").value="QA A2";row.querySelector("button").click();await settled(()=>state.profiles.some(p=>p.name==="QA A2"));
      step="create B";await create("QA B");const b=state.profiles.find(p=>p.name==="QA B");$("admin-close").click();$("profile-switch").value=b.id;$("profile-switch").dispatchEvent(new Event("change"));await settled(()=>state.profile_id===b.id&&document.querySelectorAll(".media-card").length===1);
      assert(document.querySelector(".media-card .watched-toggle").textContent==="Non vu"&&document.querySelector(".media-card select").value==="","profile state mixed");assert(state.preferences.audio_language==="en","preferences mixed");
    }
    await window.__TAURI__.core.invoke("qa_result",{success:true,detail:config.mode||"connect-consent-profiles-library-scan-personal-state"});
  } catch(error){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+error.message});}
};
