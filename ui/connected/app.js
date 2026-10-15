"use strict";
const $ = id => document.getElementById(id);
let state = {stage:"server"}, busy=false, offset=0, more=false, deleting=null, timer=null, draftAddress="";
const CATALOG_PAGE_SIZE=48;
function message(value="") { $("message").textContent=value;document.querySelectorAll(".dialog-message").forEach(n=>n.textContent=value); }
function node(tag,text,cls) { const n=document.createElement(tag); if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n; }
async function invoke(command) { return window.__TAURI__.core.invoke("ui_command",{command}); }
async function run(command, update=true, progress="") {
  if(busy)return null; busy=true;document.body.setAttribute("aria-busy","true");const disabledBefore=new Map();document.querySelectorAll("button,input,select").forEach(n=>{disabledBefore.set(n,n.disabled);n.disabled=true;});message(progress);
  let result;try { result=await invoke(command);if(update)state=result; }
  catch(error){$("profile-switch").value=state.profile_id||"";message(error?.message||String(error));if(error?.status===404)setTimeout(async()=>{if(await run({action:"refresh"})){if(!state.profile_id)$("media").replaceChildren();}},0);return null;}
  finally{busy=false;document.body.removeAttribute("aria-busy");for(const [n,disabled] of disabledBefore)if(n.isConnected)n.disabled=disabled;if(update&&result)render();pager();}
  if(result&&/^(create_playlist|rename_playlist|delete_playlist|add_playlist|remove_playlist_member|reorder_playlist|create_saga|rename_saga|delete_saga|add_saga|remove_saga_member|reorder_saga|saga_member_artwork|set_watched|set_rating|group_rating|edit_media|edit_series|forget_unavailable)$/.test(command.action))await syncAfterMutation();return result;
}
function render(){
  if(window.adminContextChanged)window.adminContextChanged();
  if(window.identityContextChanged)window.identityContextChanged();
  if(window.aiContextChanged)window.aiContextChanged();
  if(window.tmdbContextChanged)window.tmdbContextChanged();
  if(window.catalogProfileChanged)window.catalogProfileChanged();
  if(window.refreshAppearance)window.refreshAppearance();
  if(window.renderShell)window.renderShell();
  if(window.refreshPlaybackCache)window.refreshPlaybackCache(state.playback_cache);
  if(window.refreshPlayerPresentation)window.refreshPlayerPresentation(state.player_presentation);
  const cacheNotice=state.catalog_cache?.offline?(state.catalog_cache.notice||"Catalogue local disponible hors connexion."):"";
  const notice=[state.notice,cacheNotice].filter(Boolean).join(" · ");
  $("connection-notice").textContent=notice;$("connection-notice").hidden=!notice;
  for(const id of ["server-panel","trust-panel","profile-panel","catalog-panel"])$(id).hidden=true;
  const connected=state.stage==="connected";
  $("admin-toggle").hidden=!connected;$("server-switch").hidden=!connected;$("profile-menu").hidden=!connected||!(state.profiles?.length);
  document.documentElement.dataset.theme=state.theme||"system";$("theme").value=state.theme||"system";
  if(!connected){$("admin-panel").hidden=true;clearTimeout(timer);$("media").replaceChildren();$("profile-choices").replaceChildren();$("admin-profiles").replaceChildren();$("profile-switch").replaceChildren();$("profile-menu").open=false;}
  if(state.stage==="server"){
    $("server-panel").hidden=false;$("address").value=state.address||draftAddress;
    $("saved-servers").replaceChildren();for(const address of state.servers||[]){const b=node("button",address,"secondary");b.onclick=()=>connect(address);$("saved-servers").append(b);}return;
  }
  if(state.stage==="trust"){
    $("trust-panel").hidden=false;$("trust-address").textContent=state.address;$("fingerprint").textContent=state.fingerprint;
    $("trust-warning").textContent=state.replacement?"Attention : la clé du serveur a changé avant expiration. Vérifiez cette nouvelle empreinte avant de l'accepter.":"Vérifiez cette empreinte auprès du serveur du foyer avant votre première connexion.";
    $("certificate-expiry").textContent="Certificat valable jusqu'au "+new Date(state.expires_at*1000).toLocaleDateString("fr-FR");return;
  }
  $("profile-switch").replaceChildren(node("option","Choisir un profil…"));$("profile-switch").firstChild.value="";
  $("profile-choices").replaceChildren();$("admin-profiles").replaceChildren();
  for(const p of state.profiles){
    const option=node("option",p.name);option.value=p.id;$("profile-switch").append(option);
    const choice=node("button",p.name,"profile-choice");choice.onclick=()=>selectProfile(p.id);$("profile-choices").append(choice);
    const row=node("div",undefined,"profile-row"),name=node("input");name.value=p.name;name.maxLength=80;name.setAttribute("aria-label","Renommer "+p.name);
    const rename=node("button","Renommer","secondary");rename.onclick=async()=>{await run({action:"rename_profile",id:p.id,name:name.value,expected_revision:p.revision});};
    const remove=node("button","Supprimer","secondary danger");remove.onclick=()=>{deleting=p;$("delete-name").textContent=p.name;$("confirm-delete").showModal();};row.append(name,rename,remove);$("admin-profiles").append(row);
  }
  $("profile-switch").value=state.profile_id||"";
  const activeProfile=state.profiles.find(profile=>profile.id===state.profile_id);
  const activeName=activeProfile?.name||"Choisir un profil";
  $("profile-name").textContent=activeName;
  $("profile-avatar").textContent=activeName.trim().slice(0,1).toLocaleUpperCase()||"?";
  if(activeProfile)$("profile-menu").open=false;
  $("profile-empty").hidden=Boolean(state.profiles.length);
  $("profile-panel").hidden=Boolean(state.profile_id);$("catalog-panel").hidden=!state.profile_id;
  if(!state.profile_id)$("media").replaceChildren();
  $("video-root").value=state.library.video_root||"";$("personal-settings").hidden=!state.profile_id;
  if(!$("autoplay").querySelector('option[value="15"]'))$("autoplay").append(new Option("Après 15 secondes","15"));
  if(state.preferences){$("audio-language").value=state.preferences.audio_language;$("subtitle-language").value=state.preferences.subtitle_language;$("autoplay").value=state.preferences.autoplay_seconds===null?"off":String(state.preferences.autoplay_seconds);}
  else{$("audio-language").value="";$("subtitle-language").value="";}
  if(window.renderShell)window.renderShell();
}
async function connect(address){draftAddress=address;state={stage:"server",address,theme:state.theme,servers:state.servers||[]};render();const r=await run({action:"connect",address});if(r?.profile_id){await loadPage();void syncCatalog();}}
async function selectProfile(id){if(!id)return;clearCatalogContext();offset=0;await run({action:"select_profile",id});await loadPage();void syncCatalog();}
function pager(){} // API batches remain internal; no page-navigation UI.
function catalogCommand(filters,at,grouped){return {action:"catalog",offset:at,category:filters.category||null,q:filters.q||null,genre:filters.genre||null,year:filters.year===""?null:Number(filters.year),watched:filters.watched===""?null:filters.watched==="true",sort:filters.sort||null,grouped,available:filters.available===""?null:filters.available==="true"};}
function catalogFilters(){return {category:$("category").value,q:$("search").value,genre:$("genre").value,year:$("year").value,watched:$("watched").value,sort:$("sort").value,available:$("available").value};}
if(typeof module!=="undefined")module.exports={catalogCommand};
let catalogItems=[],catalogCounters={},catalogGeneration=0,catalogWindowKey="",catalogFrame=null,catalogContext="";
function clearCatalogContext(){catalogGeneration++;catalogItems=[];catalogCounters={};catalogWindowKey="";$("media").replaceChildren();$("media").style.height="";window.resetArtwork?.();}
function renderCatalogWindow(){
 const target=$("media");if(!target||!catalogItems.length||$("catalog-panel").hidden)return;
 const rect=target.getBoundingClientRect(),list=$("view-mode").value==="list",size=Number($("poster-size").value)||200;
 const windowInfo=catalogWindow(catalogItems.length,Math.max(1,rect.width),size,-rect.top,innerHeight,list);
 target.style.height=windowInfo.height+"px";
 const key=[catalogGeneration,windowInfo.start,windowInfo.end,windowInfo.columns,list,Math.round(rect.width)].join(":");
 if(key===catalogWindowKey)return;catalogWindowKey=key;
 const focused=document.activeElement.closest?.(".media-card")?.dataset.mediaId;
 if(window.resetArtwork)window.resetArtwork();
 const grid=node("div",undefined,"virtual-window"+(list?" list":""));grid.style.top=windowInfo.top+"px";grid.style.gridTemplateColumns="repeat("+windowInfo.columns+",minmax(0,1fr))";
 for(let index=windowInfo.start;index<windowInfo.end;index++){
  const m=catalogItems[index],card=node("article",undefined,"media-card catalog-open"),poster=node("div",m.title.slice(0,1).toUpperCase(),"poster"),info=node("div",undefined,"info");
  card.dataset.mediaId=m.id;card.tabIndex=0;const visual=node("div",m.title.slice(0,1).toUpperCase(),"poster-image");poster.replaceChildren(visual);
  info.append(node("h2",m.title),node("p",[m.year,(m.genres||[]).slice(0,2).join(", ")].filter(Boolean).join(" · ")));
  if(m.rating){const stars=node("p","★".repeat(m.rating),"poster-rating");stars.setAttribute("aria-label","Ma note : "+m.rating+" sur 5");info.append(stars);}
  const actions=node("div",undefined,"poster-actions"),watched=node("button",m.watched?"✓":"○","poster-watched");
  watched.setAttribute("aria-label",m.watched?"Marquer non vu":"Marquer vu");watched.setAttribute("aria-pressed",String(m.watched));
  if(m.category!=="serie"){
   watched.onclick=async()=>{if(await run({action:"set_watched",media_id:m.id,watched:!m.watched},false))await loadPage();};
   const play=node("button","▶","play-video");play.setAttribute("aria-label","Lire "+m.title);play.disabled=!m.available;play.onclick=()=>window.chillPlay(m.id);
   const add=node("button","+","poster-add");add.setAttribute("aria-label","Ajouter à une liste de lecture ou une saga");add.onclick=()=>window.catalogPickCollection?.(m.id,m.category);
   actions.append(play,watched,add);
  }else{
   const counters=catalogCounters[m.id];
   info.append(node("p",counters?counters.seasons+" saisons · "+counters.episodes+" épisodes · "+counters.unwatched+" non vus":m.watched?"Tous les épisodes sont vus":"Des épisodes restent à voir","series-counts"));
  }
  poster.append(actions);if(!m.available)info.append(node("p","Indisponible","unavailable-badge"));
  card.append(poster,info);const open=()=>m.category==="serie"?window.catalogOpenSeries?.(catalogCounters[m.id]?.series_key||(m.tmdb_id?"tmdb:tv:"+m.tmdb_id:"media:"+m.id)):window.catalogOpenMedia?.(m.id);
  card.onclick=event=>{if(!event.target.closest("button,select,summary,details"))open();};
  card.onkeydown=event=>{if((event.key==="Enter"||event.key===" ")&&!event.target.closest("button,select,summary,details")){event.preventDefault();open();}};
  window.catalogEnhanceCard?.(card,m);grid.append(card);
 }
 target.replaceChildren(grid);if(focused){const restore=[...grid.children].find(card=>card.dataset.mediaId===focused);restore?.focus({preventScroll:true});}
}
function scheduleCatalogWindow(){if(catalogFrame)return;catalogFrame=requestAnimationFrame(()=>{catalogFrame=null;renderCatalogWindow();});}
window.addEventListener("scroll",scheduleCatalogWindow,{passive:true});window.addEventListener("resize",()=>{catalogWindowKey="";scheduleCatalogWindow();});
async function catalogRequest(command,generation,context){
 for(let attempt=0;attempt<50;attempt++){
  if(generation!==catalogGeneration||context!==state.server_id+":"+state.profile_id)return null;
  try{return await invoke(command);}catch(error){
   if(error.code!=="CLIENT_BUSY"){message(error.message||"Catalogue indisponible.");return null;}
   await new Promise(resolve=>setTimeout(resolve,150));
  }
 }
 message("Le serveur est occupé. Rechargez le catalogue depuis Administration.");return null;
}
async function loadPage(){
 const context=state.server_id+":"+state.profile_id;if(context!==catalogContext){clearCatalogContext();catalogContext=context;}const generation=++catalogGeneration;
 if(!state.profile_id){catalogItems=[];$("media").replaceChildren();return false;}
 const filters=catalogFilters(),ids=new Set(),staged=[],counters={};let at=0;
 if(!catalogItems.length)$("catalog-count").textContent="Chargement du catalogue…";
 for(;;){
  const page=await catalogRequest(catalogCommand(filters,at,true),generation,context);
  if(!page||generation!==catalogGeneration||context!==state.server_id+":"+state.profile_id)return false;
  if(!Array.isArray(page.items)||page.items.length>CATALOG_PAGE_SIZE||page.has_more&&page.items.length===0){message("Réponse catalogue incohérente.");return false;}
  let added=0;for(const item of page.items){if(!ids.has(item.id)){ids.add(item.id);staged.push(item);added++;}}
  Object.assign(counters,page.series_counters||{});
  if(!page.has_more)break;
  if(!added||staged.length>=30000){message("Catalogue trop volumineux ou modifié pendant le chargement. Rechargez depuis Administration.");return false;}
  at+=page.items.length;
 }
 catalogItems=staged;catalogCounters=counters;catalogWindowKey="";$("catalog-count").textContent=catalogItems.length+" titres";
 if(catalogItems.length)renderCatalogWindow();else{$("media").style.height="";$("media").replaceChildren(node("p","Aucun titre ne correspond. Configurez puis scannez la bibliothèque dans Administration.","muted"));}
 return true;
}
$("connect-form").onsubmit=e=>{e.preventDefault();connect($("address").value);};
$("approve-trust").onclick=async()=>{const r=await run({action:"approve",token:state.token});if(r?.profile_id){await loadPage();void syncCatalog();}};
$("reject-trust").onclick=()=>run({action:"disconnect"});
$("server-switch").onclick=()=>run({action:"disconnect"});
$("profile-switch").onchange=()=>selectProfile($("profile-switch").value);
for(const id of ["create-first","create-admin","create-profile-menu"])$(id).onsubmit=async e=>{e.preventDefault();const name=e.currentTarget.elements.name.value;const r=await run({action:"create_profile",name});if(r){$(id).reset();$("profile-menu").open=false;}};
$("admin-toggle").onclick=async()=>{if(!await stopPlayerForNavigation())return;shellShow("admin");const context=state.server_id+":"+state.profile_id,current=()=>context===state.server_id+":"+state.profile_id&&shellSection==="admin";$("admin-panel").hidden=false;void syncCatalog();await pollScan();if(!current())return;await unavailableAdmin();if(current())await window.refreshTmdbStatus?.();};
$("admin-close").onclick=()=>{$("admin-panel").hidden=true;clearTimeout(timer);};
$("library-form").onsubmit=async e=>{e.preventDefault();if(await run({action:"save_library",video_root:$("video-root").value.trim()||null,expected_revision:state.library.revision}))message("Chemin enregistré sur le serveur.");};
$("preferences-form").onsubmit=async e=>{e.preventDefault();if(await run({action:"save_preferences",audio_language:$("audio-language").value,subtitle_language:$("subtitle-language").value,autoplay_seconds:$("autoplay").value==="off"?null:Number($("autoplay").value)}))message("Préférences du profil enregistrées.");};
$("theme").onchange=()=>run({action:"theme",theme:$("theme").value});
$("filter-form").onsubmit=e=>{e.preventDefault();offset=0;loadPage();};$("refresh-catalog").onclick=()=>syncCatalog();
$("view-mode").onchange=()=>{catalogWindowKey="";renderCatalogWindow();};
$("poster-size").oninput=()=>{document.documentElement.style.setProperty("--poster-size",$("poster-size").value+"px");catalogWindowKey="";renderCatalogWindow();};
$("scan").onclick=async()=>{if(await run({action:"start_scan"},false))await pollScan();};
async function unavailableAdmin(at=0){
  let panel=$("unavailable-admin");if(!panel){panel=document.createElement("section");panel.id="unavailable-admin";panel.append(node("h2","Vidéos indisponibles"),node("p","Retirer une entrée efface son historique de reprise, vu et notes. Aucun fichier NAS ne sera supprimé.","muted"));$("admin-panel").querySelector(".admin-grid").append(panel);}
  const context=state.server_id+":"+state.profile_id;
  const page=await libraryRead({action:"unavailable",offset:at,limit:48},()=>context===state.server_id+":"+state.profile_id);if(!page||context!==state.server_id+":"+state.profile_id)return;
  const selected=new Set(),list=node("div"),remove=node("button","Retirer la sélection","danger");
  remove.onclick=async()=>{if(!selected.size||!confirm("Retirer ces références du catalogue ? Reprises, notes, vus et associations seront perdus pour tous les profils. Aucun fichier NAS ne sera supprimé."))return;const r=await run({action:"forget_unavailable",media_ids:[...selected],expected_epoch:page.epoch,confirmed:true},false);if(r)await unavailableAdmin(at);};
  for(const item of page.items||[]){const label=node("label"),check=node("input");check.type="checkbox";check.onchange=()=>{check.checked?selected.add(item.id):selected.delete(item.id);remove.disabled=!selected.size;};label.append(check,document.createTextNode(" "+item.title+" — "+item.relative_path));list.append(label);}remove.disabled=true;
  const previous=node("button","Précédent","secondary"),next=node("button","Suivant","secondary");previous.disabled=at===0;next.disabled=!page.has_more;previous.onclick=()=>unavailableAdmin(Math.max(0,at-48));next.onclick=()=>unavailableAdmin(at+48);
  panel.replaceChildren(node("h2","Vidéos indisponibles"),node("p","Retirer une référence efface les historiques associés dans tous les profils. Les fichiers NAS ne sont jamais supprimés. Une vidéo redevenue disponible est protégée.","muted"),list,remove,previous,next);
}
async function pollScan(){clearTimeout(timer);const context=state.server_id+":"+state.profile_id,current=()=>state.stage==="connected"&&!$("admin-panel").hidden&&context===state.server_id+":"+state.profile_id;if(!current())return;const s=await libraryRead({action:"scan_status"},current);if(!current())return;if(!s){timer=setTimeout(pollScan,2000);return;}$("scan-status").textContent=s.running?"Scan en cours…":s.last_error||(s.count!==null?s.count+" vidéos découvertes.":"Aucun scan lancé dans cette session serveur.");if(s.running)timer=setTimeout(pollScan,2000);else if(s.count!==null&&state.profile_id)await syncAfterMutation();}
$("cancel-delete").onclick=()=>{$("confirm-delete").close();deleting=null;};
$("confirm-delete").addEventListener("cancel",()=>{deleting=null;});
$("confirm-delete-button").onclick=async()=>{if(!deleting)return;const p=deleting;$("confirm-delete").close();deleting=null;if(await run({action:"delete_profile",id:p.id,confirm_id:p.id})){$("media").replaceChildren();if(state.profile_id)await loadPage();}};
let lastFocusRefresh=0;
let syncPromise=null,syncContext=null;
async function syncCatalog(){
 if(state.stage!=="connected"||!state.profile_id)return;
 const requestedContext=state.server_id+":"+state.profile_id;if(syncPromise)return syncContext===requestedContext?syncPromise:syncPromise.then(()=>syncCatalog());syncContext=requestedContext;
 const context=state.server_id+":"+state.profile_id;let icon=$("sync-indicator");if(!icon){icon=node("span","⟳","sync-indicator");icon.id="sync-indicator";icon.setAttribute("role","status");$("profile-menu").before(icon);}icon.hidden=false;icon.classList.add("syncing");icon.title="Vérification des nouveautés";icon.setAttribute("aria-label",icon.title);
 syncPromise=(async()=>{try{
  const result=await invoke({action:"sync_catalog"});if(context!==state.server_id+":"+state.profile_id)return;
  if(result.changed){
   if(!await loadPage())throw new Error("Actualisation du catalogue incomplète");
   if(context!==state.server_id+":"+state.profile_id)return;
   // Only the web bridge issues a ticket; existing native clients need no new command.
   if(result.ack_ticket!==undefined){const ack=await invoke({action:"ack_catalog_sync",ticket:result.ack_ticket});if(ack.discarded){icon.title="Mise à jour différée après une modification locale";return;}}
  }
  icon.title=result.discarded?"Mise à jour différée après une modification locale":"Catalogue à jour";
 }catch(error){if(context===state.server_id+":"+state.profile_id){icon.title="Serveur indisponible — catalogue local conservé";}}finally{icon.classList.remove("syncing");icon.setAttribute("aria-label",icon.title);syncPromise=null;}})();return syncPromise;
}
async function syncAfterMutation(){if(syncPromise)await syncPromise;await syncCatalog();}
window.addEventListener("focus",()=>{if(state.stage!=="connected"||!state.profile_id||busy)return;const now=Date.now();if(now-lastFocusRefresh<5000)return;lastFocusRefresh=now;void syncCatalog();});
window.chillReady=(async()=>{if(!window.__TAURI__){message("Ouvrez cette interface depuis l'application Chill Center, pas directement dans un navigateur.");return;}const r=await run({action:"boot"});if(r?.profile_id){await loadPage();void syncCatalog();}})();
