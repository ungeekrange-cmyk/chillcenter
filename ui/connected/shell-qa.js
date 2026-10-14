// Debug host only. Requires a fresh loopback server and synthetic media/CLI.
window.chillShellQa=async config=>{
 let step="boot";
 const assert=(ok,text)=>{if(!ok)throw Error(text);};
 const wait=async check=>{const end=Date.now()+25000;while(!check()){if(Date.now()>end)throw Error("timeout "+$("message").textContent);await new Promise(r=>setTimeout(r,80));}};
 const call=async(name,command)=>{for(let i=0;i<50;i++){try{return await window.__TAURI__.core.invoke(name,{command});}catch(e){if(e.code!=="CLIENT_BUSY"||i===49)throw e;await new Promise(r=>setTimeout(r,100));}}};
 const ui=command=>call("ui_command",command),ai=command=>call("ai_command",command);
 try{
  await window.chillReady;assert(config.cli,"Synthetic CLI required");
  step="connect";if(state.stage!=="connected"){await connect(config.address);if(state.stage==="trust")await run({action:"approve",token:state.token});}
  const name="Shell QA "+Date.now();state=await ui({action:"create_profile",name});render();await selectProfile(state.profiles.find(p=>p.name===name).id);
  state=await ui({action:"save_library",video_root:config.root,expected_revision:state.library.revision});render();await ui({action:"start_scan"});
  for(let i=0;i<100;i++){if(!(await ui({action:"scan_status"})).running)break;await new Promise(r=>setTimeout(r,100));}
  await loadPage();const media=(await ui({action:"catalog",offset:0})).items;assert(media.length>=4,"Four synthetic videos required");
  step="shell placement";
  assert($("brand-logo").getBoundingClientRect().width===96,"Logo size");
  assert(document.querySelector(".brand").children.length===1,"Duplicated brand text");
  for(const id of ["refresh-catalog","tmdb-enrich","organize-ai","server-switch","scan"]){assert($(id).closest("#admin-panel"),id+" outside administration");}
  assert(!$("admin-panel").hidden===false,"Admin initially visible");
  assert(!document.querySelector("#pagination"),"Visible catalogue paging");
  step="wide detail";await openMedia(media[0].id);assert($("media-dialog").getBoundingClientRect().width>850,"Detail too small");assert($("detail-availability").hidden,"Available badge should not show");$("media-dialog").close();
  step="global search";$("search").value="Tracks";$("search").dispatchEvent(new Event("input"));await wait(()=>catalogItems.length===1&&catalogItems[0].title.includes("Tracks"));$("search").value="";await loadPage();
  step="series counters and drilldown";
  for(const [index,m]of media.slice(0,2).entries()){const d=await ui({action:"media_detail",media_id:m.id});await ui({action:"edit_media",media_id:m.id,expected_revision:d.revision,patch:{category:"serie",series:"Shell Series",season:index+1,episode:1}});await ui({action:"set_watched",media_id:m.id,watched:false});}
  $("category").value="serie";await loadPage();assert(catalogItems.length===1,"Series not grouped");assert(document.querySelector(".series-counts").textContent.includes("2 saisons · 2 épisodes · 2 non vus"),"Series counters missing");
  await openSeries("Shell Series");assert(document.querySelectorAll(".series-season").length===2,"Season drilldown missing");const season=document.querySelector(".series-season");season.open=true;season.querySelector(".member-title").click();await wait(()=>$("media-dialog").open);$("media-dialog").close();$("collection-dialog").close();
  $("category").value="film";await loadPage();
  step="create and populate saga via UI";await openCollections("saga");$("collection-create").elements.name.value="Shell saga "+Date.now();$("collection-create").requestSubmit();await wait(()=>!busy&&collectionKind==="saga"&&$("collection-dialog").open);
  await $("collection-add").onclick();const choices=[...$("picker-items").querySelectorAll("button")];assert(choices.length===2,"Saga picker must list only films");choices.forEach(b=>b.click());$("picker-confirm").click();await wait(()=>!busy&&collectionView.media_ids.length===2);$("collection-dialog").close();
  step="create playlist via UI";await openCollections("playlist");$("collection-create").elements.name.value="Shell playlist";$("collection-create").requestSubmit();await wait(()=>!busy&&collectionKind==="playlist"&&$("collection-dialog").open);await $("collection-add").onclick();assert($("picker-items").querySelectorAll("button").length===4,"Playlist missing episodes");$("picker-items").querySelectorAll("button")[0].click();$("picker-confirm").click();await wait(()=>!busy&&collectionView.media_ids.length===1);$("collection-dialog").close();
  step="TMDB config form";$("admin-toggle").click();await wait(()=>!busy&&!$("admin-panel").hidden);const form=$("tmdb-admin").querySelector("form"),input=form.querySelector("input[type=password]");input.value="00000000000000000000000000000000";form.querySelector("input[type=checkbox]").checked=false;form.requestSubmit();await wait(()=>!form.querySelector("button").disabled&&$("tmdb-admin").textContent.includes("TMDB configuré"));assert(input.value==="","Secret retained in form");const configured=await ui({action:"tmdb_status"});assert(configured.configured&&!JSON.stringify(configured).includes("00000000000000000000000000000000"),"Credential echo");await ui({action:"tmdb_clear"});
  step="advice read-only filters";await ai({action:"configure",provider:"codex",codex_path:config.cli,claude_path:"claude"});$("advice-toggle").click();await wait(()=>!$("advice-panel").hidden);$("advice-watched").value="false";$("advice-category").value="film";$("advice-request").value="Un film pour ce soir";$("advice-form").requestSubmit();await wait(()=>$("advice-results").querySelector(".advice-suggestion"));assert(!$("advice-results").textContent.includes("Confirmer cette proposition"),"Advice writes catalogue");$("advice-watched").value="true";$("advice-watched").dispatchEvent(new Event("change"));assert(!$("advice-results").children.length,"Stale suggestions after filter");
  await window.__TAURI__.core.invoke("qa_result",{success:true,detail:"native-Tauri-shell-sidebar-search-admin-logo-wide-detail-series-drilldown-saga-playlist-picker-TMDB-secret-advice-filters"});
 }catch(e){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:step+": "+(e.message||e.code||String(e))});}
};
