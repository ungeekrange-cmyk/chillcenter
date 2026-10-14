window.chillCatalogQa=async config=>{
 let step="boot";const assert=(ok,text)=>{if(!ok)throw Error(text);};
 const wait=async fn=>{const end=Date.now()+25000;while(!fn()){if(Date.now()>end)throw Error("timeout "+$("message").textContent+" / "+$("player-note").textContent+" / "+JSON.stringify(playerState.native));await new Promise(r=>setTimeout(r,70));}};
 try{
  await window.chillReady;if(state.stage!=="connected"){
   await connect(config.address);if(state.stage==="trust")await run({action:"approve",token:state.token});
   if(!state.profiles.length)await run({action:"create_profile",name:"Catalogue QA"});await selectProfile(state.profiles[0].id);
  }
  await run({action:"save_library",video_root:config.root,expected_revision:state.library.revision});await run({action:"start_scan"},false);
  for(let i=0;i<150;i++){const s=await invoke({action:"scan_status"});if(!s.running)break;await new Promise(r=>setTimeout(r,100));}
  await loadPage();const page=await invoke({action:"catalog",offset:0,category:null,q:null});assert(page.items.length>=2,"two media fixtures required");const [a,b]=page.items,profile=state.profile_id;
  step="fiche CAS";await openMedia(a.id);$("detail-edit").click();$("edit-overview").value="Résumé modifié depuis Tauri";$("metadata-form").requestSubmit();await wait(()=>!busy&&detailView?.media.overview==="Résumé modifié depuis Tauri");
  const current=await invoke({action:"media_detail",media_id:a.id});assert(current.revision>1,"revision not incremented");
  let conflict=false;try{await invoke({action:"edit_media",media_id:a.id,expected_revision:current.revision-1,patch:{overview:"stale"}});}catch(e){conflict=e.status===409;}assert(conflict,"CAS conflict must be409");$("media-dialog").close();
  step="playlist UI";await openCollections();$("collection-create-toggle").click();$("collection-create").elements.name.value="Playlist Tauri "+Date.now();$("collection-create").requestSubmit();await wait(()=>!busy&&collectionView?.id&&!$("collection-panel").hidden);let id=collectionView.id;
  await invoke({action:"add_playlist",id,expected_revision:collectionView.revision,media_ids:[a.id,b.id]});await openCollection(id);assert(collectionView.media_ids.length===2,"membership missing");
  const revision=collectionView.revision;await invoke({action:"reorder_playlist",id,expected_revision:revision,expected_order:[a.id,b.id],media_ids:[b.id,a.id]});await openCollection(id);assert(collectionView.media_ids[0]===b.id,"order failed");
  const renamed="Playlist renommée "+Date.now();$("collection-name").value=renamed;$("collection-rename").requestSubmit();await wait(()=>!busy&&collectionView?.name===renamed);shellShow("catalog");
  step="saga commune UI";await openCollections("saga");$("collection-create-toggle").click();$("collection-create").elements.name.value="Saga Tauri "+Date.now();$("collection-create").requestSubmit();await wait(()=>!busy&&collectionKind==="saga"&&!$("collection-panel").hidden);const sagaId=collectionView.id;
  await invoke({action:"add_saga",id:sagaId,expected_revision:collectionView.revision,media_ids:[a.id,b.id]});await openCollection(sagaId,"saga");assert(collectionView.media_ids.length===2,"saga members missing");shellShow("catalog");
  step="isolation profils";await run({action:"create_profile",name:"Autre QA "+Date.now()});const other=state.profiles.find(p=>p.id!==profile&&p.name.startsWith("Autre QA"));await selectProfile(other.id);assert(!(await invoke({action:"playlists"})).items.some(p=>p.id===id),"playlist leaked");assert((await invoke({action:"sagas"})).items.some(s=>s.id===sagaId),"saga not common");assert(!!$("collection-panel").hidden,"old dialog persists");await selectProfile(profile);
  step="external native subtitle";await window.chillPlay(a.id);await wait(()=>playerState.native?.loaded);if(playerState.native.ended){await playerCall({action:"seek",position:0});await playerCall({action:"resume"});}await wait(()=>playerState.native?.loaded&&!playerState.native.paused);assert(playerState.external_subtitles.length,"external fixture missing");const ext=playerState.external_subtitles[0];await playerCall({action:"external_subtitle",id:ext.id});await wait(()=>playerState.native.tracks.some(t=>t.kind==="Subtitle"&&t.title===ext.title));await playerCall({action:"stop"});
  await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"Tauri-detail-CAS-playlist-order-rename-common-saga-profile-isolation-external-subtitle"});
 }catch(e){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+e.message});}
};
