"use strict";
var uiText=globalThis.chillI18n?.t||((text)=>text);
const $ = id => document.getElementById(id);
let state = {stage:"server"}, busy=false, offset=0, more=false, deleting=null, timer=null, draftAddress="";
const CATALOG_PAGE_SIZE=48;
function message(value="") { const text=uiText(value);$("message").textContent=text;document.querySelectorAll(".dialog-message").forEach(n=>n.textContent=text); }
function node(tag,text,cls) { const n=document.createElement(tag); if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n; }
async function invoke(command) {
  const recovery=window.conflictTarget?.(command);
  try{return await window.__TAURI__.core.invoke("ui_command",{command});}
  catch(error){window.offerConflictRecovery?.(error,recovery);throw error;}
}
async function run(command, update=true, progress="") {
  if(busy)return null; busy=true;document.body.setAttribute("aria-busy","true");const disabledBefore=new Map();document.querySelectorAll("button,input,select").forEach(n=>{disabledBefore.set(n,n.disabled);n.disabled=true;});message(progress);
  let result;try { result=await invoke(command);if(update)state=result; }
  catch(error){$("profile-switch").value=state.profile_id||"";message(error?.message||String(error));if(error?.status===404)setTimeout(async()=>{if(await run({action:"refresh"})){if(!state.profile_id)$("media").replaceChildren();}},0);return null;}
  finally{busy=false;document.body.removeAttribute("aria-busy");for(const [n,disabled] of disabledBefore)if(n.isConnected)n.disabled=disabled;if(update&&result)render();pager();}
  if(result&&/^(create_playlist|rename_playlist|delete_playlist|add_playlist|remove_playlist_member|reorder_playlist|create_saga|rename_saga|delete_saga|add_saga|remove_saga_member|reorder_saga|saga_member_artwork|set_watched|set_rating|group_rating|edit_media|edit_series|forget_unavailable)$/.test(command.action)){invalidateFacetBase();await syncAfterMutation();}return result;
}
function render(){
  window.recoveryContextChanged?.();
  if(state.language!==undefined&&window.chillI18n?.hydrate(state.language))return;
  if(window.adminContextChanged)window.adminContextChanged();
  if(window.identityContextChanged)window.identityContextChanged();
  if(window.aiContextChanged)window.aiContextChanged();
  if(window.tmdbContextChanged)window.tmdbContextChanged();
  if(window.catalogProfileChanged)window.catalogProfileChanged();
  if(window.refreshAppearance)window.refreshAppearance();
  if(window.renderShell)window.renderShell();
  void window.refreshProgress?.();
  if(window.refreshPlaybackCache)window.refreshPlaybackCache(state.playback_cache);
  if(window.refreshPlayerPresentation)window.refreshPlayerPresentation(state.player_presentation);
  const cacheNotice=state.catalog_cache?.offline?(state.catalog_cache.notice||uiText("Catalogue local disponible hors connexion.")):"";
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
    $("trust-warning").textContent=state.replacement?uiText("Attention : la clé du serveur a changé avant expiration. Vérifiez cette nouvelle empreinte avant de l'accepter."):uiText("Vérifiez cette empreinte auprès du serveur du foyer avant votre première connexion.");
    $("certificate-expiry").textContent=uiText("Certificat valable jusqu'au ")+new Date(state.expires_at*1000).toLocaleDateString(window.chillI18n?.language||"en");return;
  }
  $("profile-switch").replaceChildren(node("option",uiText("Choisir un profil…")));$("profile-switch").firstChild.value="";
  $("profile-choices").replaceChildren();$("admin-profiles").replaceChildren();
  for(const p of state.profiles){
    const option=node("option",p.name);option.value=p.id;$("profile-switch").append(option);
    const choice=node("button",p.name,"profile-choice");choice.onclick=()=>selectProfile(p.id);$("profile-choices").append(choice);
    const row=node("div",undefined,"profile-row"),name=node("input");name.id="profile-name-"+p.id;name.value=p.name;name.maxLength=80;name.setAttribute("aria-label",uiText("Renommer ")+p.name);
    const rename=node("button",uiText("Renommer"),"secondary");rename.onclick=async()=>{await run({action:"rename_profile",id:p.id,name:name.value,expected_revision:p.revision});};
    const remove=node("button",uiText("Supprimer"),"secondary danger");remove.onclick=()=>{deleting=p;$("delete-name").textContent=p.name;$("confirm-delete").showModal();};row.append(name,rename,remove);$("admin-profiles").append(row);
  }
  $("profile-switch").value=state.profile_id||"";
  const activeProfile=state.profiles.find(profile=>profile.id===state.profile_id);
  const activeName=activeProfile?.name||uiText("Choisir un profil");
  $("profile-name").textContent=activeName;
  $("profile-avatar").textContent=activeName.trim().slice(0,1).toLocaleUpperCase()||"?";
  if(activeProfile)$("profile-menu").open=false;
  $("profile-empty").hidden=Boolean(state.profiles.length);
  $("profile-panel").hidden=Boolean(state.profile_id);$("catalog-panel").hidden=!state.profile_id;
  if(!state.profile_id)$("media").replaceChildren();
  if(window.refreshLibraryLocations)window.refreshLibraryLocations(state.library);else $("video-root").value=state.library.video_root||"";$("personal-settings").hidden=!state.profile_id;
  if(!$("autoplay").querySelector('option[value="15"]'))$("autoplay").append(new Option(uiText("Après 15 secondes"),"15"));
  if(state.preferences){$("audio-language").value=state.preferences.audio_language;$("subtitle-language").value=state.preferences.subtitle_language;$("autoplay").value=state.preferences.autoplay_seconds===null?"off":String(state.preferences.autoplay_seconds);}
  else{$("audio-language").value="";$("subtitle-language").value="";}
  if(window.renderShell)window.renderShell();window.refreshConfiguration?.();
}
async function connect(address){draftAddress=address;state={stage:"server",address,theme:state.theme,servers:state.servers||[]};render();const r=await run({action:"connect",address});if(r?.profile_id){await loadPage();void syncCatalog();}}
async function selectProfile(id){if(!id)return;clearCatalogContext();offset=0;await run({action:"select_profile",id});await loadPage();void syncCatalog();}
function pager(){} // API batches remain internal; no page-navigation UI.
function catalogCommand(filters,at,grouped){return {action:"catalog",offset:at,category:filters.category||null,q:filters.q||null,genre:filters.genre||null,year:filters.year===""?null:Number(filters.year),watched:filters.watched===""?null:filters.watched==="true",sort:filters.sort||null,grouped,available:filters.available===""?null:filters.available==="true"};}
function catalogFilters(){return {category:$("category").value,q:$("search").value,genre:$("genre").value,year:$("year").value,watched:$("watched").value,sort:$("sort").value,available:$("available").value};}
if(typeof module!=="undefined")module.exports={catalogCommand};
let catalogItems=[],catalogCounters={},catalogGeneration=0,catalogWindowKey="",catalogFrame=null,catalogContext="",facetBase=null,catalogCards=new Map();
// Facets belong to the current server/profile and non-facet filters, never a global cache.
function invalidateFacetBase(){facetBase=null;}
window.invalidateFacetBase=invalidateFacetBase;
function discardCatalogCards(){for(const card of catalogCards.values())window.retireArtwork?.(card.querySelector(".poster-image"));catalogCards.clear();}
function clearCatalogContext(){catalogGeneration++;catalogItems=[];catalogCounters={};catalogWindowKey="";invalidateFacetBase();window.catalogFacets?.clear();discardCatalogCards();$("media").replaceChildren();$("media").style.height="";window.resetArtwork?.();}
function catalogCard(m){
 const groupedSeries=m.category==="serie"&&!window.isProgressView?.();
 const card=node("article",undefined,"media-card catalog-open"),poster=node("div",m.title.slice(0,1).toUpperCase(),"poster"),info=node("div",undefined,"info");
 card.dataset.mediaId=m.id;card.tabIndex=0;const visual=node("div",m.title.slice(0,1).toUpperCase(),"poster-image");poster.replaceChildren(visual);
 info.append(node("h2",m.title),node("p",[m.year,(m.genres||[]).slice(0,2).join(", ")].filter(Boolean).join(" · ")));
 if(m.rating){const stars=node("p","★".repeat(m.rating),"poster-rating");stars.setAttribute("aria-label",uiText("Ma note : ")+m.rating+uiText(" sur 5"));info.append(stars);}
 const actions=node("div",undefined,"poster-actions"),watched=node("button",m.watched?"✓":"○","poster-watched");
 watched.setAttribute("aria-label",m.watched?uiText("Marquer non vu"):uiText("Marquer vu"));watched.setAttribute("aria-pressed",String(m.watched));
 if(!groupedSeries){
  watched.onclick=async()=>{if(await run({action:"set_watched",media_id:m.id,watched:!m.watched},false))await loadPage();};
  const play=node("button","▶","play-video");play.setAttribute("aria-label",uiText("Lire ")+m.title);play.disabled=!m.available;play.onclick=()=>window.chillPlay(m.id,m.category==="serie"?{kind:"series",id:m.tmdb_id?"tmdb:tv:"+m.tmdb_id:"media:"+m.id}:null);
  const add=node("button","+","poster-add");add.setAttribute("aria-label",uiText("Ajouter à une liste de lecture ou une saga"));add.onclick=()=>window.catalogPickCollection?.(m.id,m.category);
  actions.append(play,watched,add);
 }else{
  const counters=catalogCounters[m.id];
  info.append(node("p",counters?counters.seasons+uiText(" saisons · ")+counters.episodes+uiText(" épisodes · ")+counters.unwatched+uiText(" non vus"):m.watched?uiText("Tous les épisodes sont vus"):uiText("Des épisodes restent à voir"),"series-counts"));
 }
 poster.append(actions);if(!m.available)info.append(node("p",uiText("Indisponible"),"unavailable-badge"));
 card.append(poster,info);if(!groupedSeries)window.renderMediaProgress?.(poster,m);const open=()=>groupedSeries?window.catalogOpenSeries?.(catalogCounters[m.id]?.series_key||(m.tmdb_id?"tmdb:tv:"+m.tmdb_id:"media:"+m.id)):window.catalogOpenMedia?.(m.id);
 card.onclick=event=>{if(!event.target.closest("button,select,summary,details"))open();};
 card.onkeydown=event=>{if((event.key==="Enter"||event.key===" ")&&!event.target.closest("button,select,summary,details")){event.preventDefault();open();}};
 window.catalogEnhanceCard?.(card,m);return card;
}
function renderCatalogWindow(){
 const target=$("media");if(!target||!catalogItems.length||$("catalog-panel").hidden)return;
 const rect=target.getBoundingClientRect(),list=$("view-mode").value==="list",size=Number($("poster-size").value)||200;
 // Native playback removes this panel from layout without changing `hidden`.
 // Never cache a virtual grid measured in that zero-width state.
 if(document.body.classList.contains("player-open")||rect.width<=0)return;
 const windowInfo=catalogWindow(catalogItems.length,Math.max(1,rect.width),size,-rect.top,innerHeight,list);
 target.style.height=windowInfo.height+"px";
 const key=[catalogGeneration,windowInfo.start,windowInfo.end,windowInfo.columns,list,Math.round(rect.width)].join(":");
 if(key===catalogWindowKey)return;catalogWindowKey=key;
 const focused=document.activeElement.closest?.(".media-card")?.dataset.mediaId;
 const grid=node("div",undefined,"virtual-window"+(list?" list":""));grid.style.top=windowInfo.top+"px";grid.style.gridTemplateColumns="repeat("+windowInfo.columns+",minmax(0,1fr))";grid.style.gridAutoRows=windowInfo.gridRowHeight+"px";
 const wanted=new Set();
 for(let index=windowInfo.start;index<windowInfo.end;index++){
  const m=catalogItems[index];wanted.add(m.id);let card=catalogCards.get(m.id);if(!card){card=catalogCard(m);catalogCards.set(m.id,card);}grid.append(card);
 }
 for(const [id,card] of catalogCards)if(!wanted.has(id)){window.retireArtwork?.(card.querySelector(".poster-image"));catalogCards.delete(id);}
 target.replaceChildren(grid);if(focused){const restore=[...grid.children].find(card=>card.dataset.mediaId===focused);restore?.focus({preventScroll:true});}
}
function scheduleCatalogWindow(){if(catalogFrame)return;catalogFrame=requestAnimationFrame(()=>{catalogFrame=null;renderCatalogWindow();});}
window.restoreCatalogLayout=()=>{catalogWindowKey="";scheduleCatalogWindow();requestAnimationFrame(()=>window.restoreCollectionLayout?.());};
window.catalogMediaUnavailable=mediaId=>{
 const update=media=>media.id===mediaId?{...media,available:false}:media;
 // A grouped show's availability is not that of its representative episode.
 // Update its individual detail, not the whole show's poster/card blindly.
 catalogItems=catalogItems.map(media=>media.category==="serie"?media:update(media));
 if(facetBase)facetBase={...facetBase,items:facetBase.items.map(media=>media.category==="serie"?media:update(media))};
 const old=catalogCards.get(mediaId);if(old){window.retireArtwork?.(old.querySelector(".poster-image"));catalogCards.delete(mediaId);}
 catalogWindowKey="";renderCatalogWindow();window.catalogDetailUnavailable?.(mediaId);
 // The server persisted this one-file observation. Fetch its normal delta in
 // background, never force a full scan or discard unrelated cached titles.
 void syncCatalog();
};
window.addEventListener("scroll",scheduleCatalogWindow,{passive:true});window.addEventListener("resize",()=>{catalogWindowKey="";scheduleCatalogWindow();});
// A busy native bridge is retried only while this profile and request generation remain current.
async function catalogRequest(command,generation,context){
 for(let attempt=0;attempt<50;attempt++){
  if(generation!==catalogGeneration||context!==state.server_id+":"+state.profile_id)return null;
  try{return await invoke(command);}catch(error){
   if(error.code!=="CLIENT_BUSY"){message(error.message||uiText("Catalogue indisponible."));return null;}
   await new Promise(resolve=>setTimeout(resolve,150));
  }
 }
 message(uiText("Le serveur est occupé. Rechargez le catalogue depuis Configuration."));return null;
}
// Stage the complete replacement before rendering: background refresh must not blank the grid.
// Generation/context guards also discard replies from a previous filter, server or profile.
async function loadPage(reuseFacetBase=false){
 const context=state.server_id+":"+state.profile_id;if(context!==catalogContext){clearCatalogContext();catalogContext=context;}const generation=++catalogGeneration;
 if(!state.profile_id){catalogItems=[];$("media").replaceChildren();return false;}
 const filters=catalogFilters(),baseFilters={...filters,genre:"",year:""},grouped=!window.isProgressView?.(),baseKey=context+"\u0000"+JSON.stringify({...baseFilters,grouped});
 let staged,counters;
 if(reuseFacetBase&&facetBase?.key===baseKey){staged=facetBase.items;counters=facetBase.counters;}
 else{
  const ids=new Set();staged=[];counters={};let at=0;
 if(!catalogItems.length)$("catalog-count").textContent=uiText("Chargement du catalogue…");
 for(;;){
  const page=await catalogRequest(catalogCommand(baseFilters,at,grouped),generation,context);
  if(!page||generation!==catalogGeneration||context!==state.server_id+":"+state.profile_id)return false;
  if(!Array.isArray(page.items)||page.items.length>CATALOG_PAGE_SIZE||page.has_more&&page.items.length===0){message(uiText("Réponse catalogue incohérente."));return false;}
  let added=0;for(const item of page.items){if(!ids.has(item.id)){ids.add(item.id);staged.push(item);added++;}}
  Object.assign(counters,page.series_counters||{});
  if(!page.has_more)break;
  if(!added||staged.length>=30000){message(uiText("Catalogue trop volumineux ou modifié pendant le chargement. Rechargez depuis Configuration."));return false;}
  at+=page.items.length;
 }
  facetBase={key:baseKey,items:staged,counters};
 }
 if(generation!==catalogGeneration||context!==state.server_id+":"+state.profile_id)return false;
 const eligible=window.filterProgressItems?.(staged)||staged;
 discardCatalogCards();window.catalogFacets?.draw(eligible);catalogItems=window.catalogFacets?.filter(eligible)||eligible;catalogCounters=counters;catalogWindowKey="";$("catalog-count").textContent=catalogItems.length+uiText(" titres");
 if(catalogItems.length)renderCatalogWindow();else{$("media").style.height="";$("media").replaceChildren(node("p",uiText("Aucun titre ne correspond à ces filtres. Les fiches sans année ne sont pas incluses dans un filtre d’année."),"muted"));}
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
$("library-form").onsubmit=async e=>{e.preventDefault();let command;try{command=window.configurationLibraryCommand?.()||{action:"save_library",video_root:$("video-root").value.trim()||null,expected_revision:state.library.revision};}catch(error){message(error.message);return;}if(await run(command))message(uiText("Emplacements enregistrés sur le serveur."));};
$("preferences-form").onsubmit=async e=>{e.preventDefault();if(await run({action:"save_preferences",audio_language:$("audio-language").value,subtitle_language:$("subtitle-language").value,autoplay_seconds:$("autoplay").value==="off"?null:Number($("autoplay").value)}))message(uiText("Préférences du profil enregistrées."));};
$("theme").onchange=()=>run({action:"theme",theme:$("theme").value});
$("filter-form").onsubmit=e=>{e.preventDefault();offset=0;loadPage();};$("refresh-catalog").onclick=()=>syncCatalog();
$("view-mode").onchange=()=>{catalogWindowKey="";renderCatalogWindow();};
$("poster-size").oninput=()=>{document.documentElement.style.setProperty("--poster-size",$("poster-size").value+"px");catalogWindowKey="";renderCatalogWindow();};
window.catalogFacets?.init({onChange:()=>{if(state.profile_id)loadPage(true);}});
$("scan").onclick=async()=>{if(await run({action:"start_scan"},false))await pollScan();};
async function unavailableAdmin(at=0){
  let panel=$("unavailable-admin");if(!panel){panel=document.createElement("section");panel.id="unavailable-admin";panel.append(node("h2",uiText("Vidéos indisponibles")),node("p",uiText("Retirer une entrée efface son historique de reprise, vu et notes. Aucun fichier NAS ne sera supprimé."),"muted"));$("admin-panel").querySelector(".admin-grid").append(panel);}
  const context=state.server_id+":"+state.profile_id;
  const page=await libraryRead({action:"unavailable",offset:at,limit:48},()=>context===state.server_id+":"+state.profile_id);if(!page||context!==state.server_id+":"+state.profile_id)return;
  const selected=new Set(),list=node("div"),remove=node("button",uiText("Retirer la sélection"),"danger");
  remove.onclick=async()=>{if(!selected.size||!confirm(uiText("Retirer ces références du catalogue ? Reprises, notes, vus et associations seront perdus pour tous les profils. Aucun fichier NAS ne sera supprimé.")))return;const r=await run({action:"forget_unavailable",media_ids:[...selected],expected_epoch:page.epoch,confirmed:true},false);if(r)await unavailableAdmin(at);};
  for(const item of page.items||[]){const label=node("label"),check=node("input");check.type="checkbox";check.onchange=()=>{check.checked?selected.add(item.id):selected.delete(item.id);remove.disabled=!selected.size;};label.append(check,document.createTextNode(" "+item.title+" — "+item.relative_path));list.append(label);}remove.disabled=true;
  const previous=node("button",uiText("Précédent"),"secondary"),next=node("button",uiText("Suivant"),"secondary");previous.disabled=at===0;next.disabled=!page.has_more;previous.onclick=()=>unavailableAdmin(Math.max(0,at-48));next.onclick=()=>unavailableAdmin(at+48);
  panel.replaceChildren(node("h2",uiText("Vidéos indisponibles")),node("p",uiText("Retirer une référence efface les historiques associés dans tous les profils. Les fichiers NAS ne sont jamais supprimés. Une vidéo redevenue disponible est protégée."),"muted"),list,remove,previous,next);
}
async function pollScan(){clearTimeout(timer);const context=state.server_id+":"+state.profile_id,current=()=>state.stage==="connected"&&!$("admin-panel").hidden&&context===state.server_id+":"+state.profile_id;if(!current())return;const s=await libraryRead({action:"scan_status"},current);if(!current())return;if(!s){timer=setTimeout(pollScan,2000);return;}$("scan-status").textContent=s.running?uiText("Scan en cours…"):s.last_error||(s.count!==null?s.count+uiText(" vidéos découvertes."):uiText("Aucun scan lancé dans cette session serveur."));if(s.running)timer=setTimeout(pollScan,2000);else if(s.count!==null&&state.profile_id)await syncAfterMutation();}
$("cancel-delete").onclick=()=>{$("confirm-delete").close();deleting=null;};
$("confirm-delete").addEventListener("cancel",()=>{deleting=null;});
$("confirm-delete-button").onclick=async()=>{if(!deleting)return;const p=deleting;$("confirm-delete").close();deleting=null;if(await run({action:"delete_profile",id:p.id,confirm_id:p.id})){window.dropProgressProfile?.(p.id);$("media").replaceChildren();if(state.profile_id)await loadPage();}};
let lastFocusRefresh=0;
function makeSyncIndicator(){
 const icon=node("span","","sync-indicator");icon.id="sync-indicator";icon.setAttribute("role","status");
 const svg=document.createElementNS("http://www.w3.org/2000/svg","svg"),path=document.createElementNS(svg.namespaceURI,"path");
 svg.setAttribute("viewBox","0 0 24 24");svg.setAttribute("class","sync-glyph");svg.setAttribute("aria-hidden","true");svg.setAttribute("focusable","false");path.setAttribute("d","M21 12a9 9 0 1 1-2.64-6.36 M21 3v6h-6");svg.append(path);icon.append(svg);return icon;
}
let syncPromise=null,syncContext=null;
// Detached delta sync owns only its indicator, not the global UI busy state.
// Keep native and web bridge contracts distinct; only web replies may require an ack ticket.
async function syncCatalog(){
 if(state.stage!=="connected"||!state.profile_id)return;
 const requestedContext=state.server_id+":"+state.profile_id;if(syncPromise)return syncContext===requestedContext?syncPromise:syncPromise.then(()=>syncCatalog());syncContext=requestedContext;
 const context=state.server_id+":"+state.profile_id;let icon=$("sync-indicator");if(!icon){icon=makeSyncIndicator();$("profile-menu").before(icon);}icon.hidden=false;icon.classList.add("syncing");icon.title=uiText("Vérification des nouveautés");icon.setAttribute("aria-label",icon.title);
 syncPromise=(async()=>{try{
  const result=await invoke({action:"sync_catalog"});if(context!==state.server_id+":"+state.profile_id)return;
  if(result.changed){
   if(typeof window!=="undefined")window.invalidateFacetBase?.();
   if(!await loadPage())throw new Error(uiText("Actualisation du catalogue incomplète"));
   if(context!==state.server_id+":"+state.profile_id)return;
   // Only the web bridge issues a ticket; existing native clients need no new command.
   if(result.ack_ticket!==undefined){const ack=await invoke({action:"ack_catalog_sync",ticket:result.ack_ticket});if(ack.discarded){icon.title=uiText("Mise à jour différée après une modification locale");return;}}
  }
  icon.title=result.discarded?uiText("Mise à jour différée après une modification locale"):uiText("Catalogue à jour");return result;
 }catch(error){if(context===state.server_id+":"+state.profile_id){icon.title=uiText("Serveur indisponible — catalogue local conservé");}return {error};}finally{icon.classList.remove("syncing");icon.setAttribute("aria-label",icon.title);syncPromise=null;}})();return syncPromise;
}
async function syncAfterMutation(){if(syncPromise)await syncPromise;const result=await syncCatalog();if(typeof window!=="undefined")void window.refreshProgress?.(true);return result;}
window.addEventListener("focus",()=>{if(state.stage!=="connected"||!state.profile_id||busy)return;const now=Date.now();if(now-lastFocusRefresh<5000)return;lastFocusRefresh=now;void syncCatalog();});
window.chillReady=(async()=>{
 // All deferred modules (artwork, shell, appearance...) must be installed before
 // a fast local-mirror boot can render cards. Otherwise the optional artwork
 // hook is absent and the initial images are never observed until a later redraw.
 if(document.readyState==="loading"||document.readyState==="interactive")await new Promise(resolve=>document.addEventListener("DOMContentLoaded",resolve,{once:true}));
 if(!window.__TAURI__){message(uiText("Ouvrez cette interface depuis l'application Chill Center, pas directement dans un navigateur."));return;}
 const r=await run({action:"boot"});if(r?.profile_id){await loadPage();void syncCatalog();}
 if(typeof sessionStorage!=="undefined"&&sessionStorage.getItem("chillcenter-language-return")==="settings"){
  sessionStorage.removeItem("chillcenter-language-return");
  if(r?.stage==="connected")await $("admin-toggle").onclick();
 }
})();
