"use strict";
const $ = id => document.getElementById(id);
let state = {stage:"server"}, busy=false, offset=0, more=false, deleting=null, timer=null, draftAddress="";
function message(value="") { $("message").textContent=value;document.querySelectorAll(".dialog-message").forEach(n=>n.textContent=value); }
function node(tag,text,cls) { const n=document.createElement(tag); if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n; }
async function invoke(command) { return window.__TAURI__.core.invoke("ui_command",{command}); }
async function run(command, update=true) {
  if(busy)return null; busy=true;document.body.setAttribute("aria-busy","true");const disabledBefore=new Map();document.querySelectorAll("button,input,select").forEach(n=>{disabledBefore.set(n,n.disabled);n.disabled=true;});message();
  try { const result=await invoke(command);if(update){state=result;render();}return result; }
  catch(error){$("profile-switch").value=state.profile_id||"";message(error?.message||String(error));if(error?.status===404)setTimeout(async()=>{if(await run({action:"refresh"})){if(!state.profile_id)$("media").replaceChildren();}},0);return null;}
  finally{busy=false;document.body.removeAttribute("aria-busy");for(const [n,disabled] of disabledBefore)if(n.isConnected)n.disabled=disabled;pager();}
}
function render(){
  if(window.catalogProfileChanged)window.catalogProfileChanged();
  if(window.refreshAppearance)window.refreshAppearance();
  $("connection-notice").textContent=state.notice||"";$("connection-notice").hidden=!state.notice;
  for(const id of ["server-panel","trust-panel","profile-panel","catalog-panel"])$(id).hidden=true;
  const connected=state.stage==="connected";
  $("admin-toggle").hidden=!connected;$("server-switch").hidden=!connected;$("profile-switch").hidden=!connected||!(state.profiles?.length);
  document.documentElement.dataset.theme=state.theme||"system";$("theme").value=state.theme||"system";
  if(!connected){$("admin-panel").hidden=true;clearTimeout(timer);$("media").replaceChildren();$("profile-choices").replaceChildren();$("admin-profiles").replaceChildren();$("profile-switch").replaceChildren();}
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
  $("profile-switch").value=state.profile_id||"";$("profile-empty").hidden=Boolean(state.profiles.length);
  $("profile-panel").hidden=Boolean(state.profile_id);$("catalog-panel").hidden=!state.profile_id;
  if(!state.profile_id)$("media").replaceChildren();
  $("video-root").value=state.library.video_root||"";$("personal-settings").hidden=!state.profile_id;
  if(state.preferences){$("audio-language").value=state.preferences.audio_language;$("subtitle-language").value=state.preferences.subtitle_language;$("autoplay").value=state.preferences.autoplay_seconds===null?"off":String(state.preferences.autoplay_seconds);}
  else{$("audio-language").value="";$("subtitle-language").value="";}
}
async function connect(address){draftAddress=address;state={stage:"server",address,theme:state.theme,servers:state.servers||[]};render();const r=await run({action:"connect",address});if(r?.profile_id)await loadPage();}
async function selectProfile(id){if(!id)return;$("media").replaceChildren();offset=0;const r=await run({action:"select_profile",id});if(r)await loadPage();}
function pager(){ $("previous").disabled=busy||offset===0;$("next").disabled=busy||!more;$("page-label").textContent="Page "+(Math.floor(offset/50)+1); }
function catalogCommand(filters,at,grouped){return {action:"catalog",offset:at,category:filters.category||null,q:filters.q||null,genre:filters.genre||null,year:filters.year===""?null:Number(filters.year),watched:filters.watched===""?null:filters.watched==="true",sort:filters.sort||null,grouped,available:filters.available===""?null:filters.available==="true"};}
function catalogFilters(){return {category:$("category").value,q:$("search").value,genre:$("genre").value,year:$("year").value,watched:$("watched").value,sort:$("sort").value,available:$("available").value};}
if(typeof module!=="undefined")module.exports={catalogCommand};
async function loadPage(){
  if(window.resetArtwork)window.resetArtwork();
  if(!state.profile_id)return;$("media").replaceChildren();
  const page=await run(catalogCommand(catalogFilters(),offset,true),false);if(!page)return;
  more=page.has_more;pager();
  if(!page.items.length){$("media").append(node("p","Aucun élément sur cette page. Configurez puis scannez la bibliothèque dans Administration.","muted"));return;}
  for(const m of page.items){
    const card=node("article",undefined,"media-card"),poster=node("div",m.title.slice(0,1).toUpperCase(),"poster"),info=node("div",undefined,"info");
    info.append(node("h2",m.title),node("p",[m.category,m.year,m.season!==null?"Saison "+m.season:null,m.episode!==null?"Épisode "+m.episode:null].filter(Boolean).join(" · ")));
    const details=node("details"),summary=node("summary","Détails");details.append(summary,node("p",m.overview||"Informations à compléter"));info.append(details);
    const actions=node("div",undefined,"actions"),watched=node("button",m.watched?"Vu ✓":"Non vu","secondary"),rating=node("select");rating.setAttribute("aria-label","Note de "+m.title);
    for(const [value,text] of [["","Sans note"],["1","★"],["2","★★"],["3","★★★"],["4","★★★★"],["5","★★★★★"]]){const o=node("option",text);o.value=value;rating.append(o);}rating.value=m.rating?String(m.rating):"";
    if(m.category!=="serie"){
      watched.onclick=async()=>{if(await run({action:"set_watched",media_id:m.id,watched:!m.watched},false))await loadPage();};
      rating.onchange=async()=>{if(await run({action:"set_rating",media_id:m.id,rating:rating.value?Number(rating.value):null},false))await loadPage();};
      const play=node("button","Lecture","play-video");play.disabled=!m.available;play.onclick=()=>window.chillPlay(m.id);
      actions.append(play,watched,rating);watched.classList.add("watched-toggle");info.append(actions);
    }else{info.append(node("p",m.watched?"Tous les épisodes sont vus":"Des épisodes restent à voir","muted"),actions);}
    if(!m.available)info.append(node("p","Indisponible au dernier scan","muted"));card.append(poster,info);
    card.tabIndex=0;card.classList.add("catalog-open");const open=()=>m.category==="serie"?window.catalogOpenSeries?.(m.series||m.title):window.catalogOpenMedia?.(m.id);card.onclick=e=>{if(!e.target.closest("button,select,summary,details"))open();};card.onkeydown=e=>{if((e.key==="Enter"||e.key===" ")&&!e.target.closest("button,select,summary,details")){e.preventDefault();open();}};
    if(window.catalogEnhanceCard)window.catalogEnhanceCard(card,m);$("media").append(card);
  }
}
$("connect-form").onsubmit=e=>{e.preventDefault();connect($("address").value);};
$("approve-trust").onclick=async()=>{const r=await run({action:"approve",token:state.token});if(r?.profile_id)await loadPage();};
$("reject-trust").onclick=()=>run({action:"disconnect"});
$("server-switch").onclick=()=>run({action:"disconnect"});
$("profile-switch").onchange=()=>selectProfile($("profile-switch").value);
for(const id of ["create-first","create-admin"])$(id).onsubmit=async e=>{e.preventDefault();const name=e.currentTarget.elements.name.value;const r=await run({action:"create_profile",name});if(r)$(id).reset();};
$("admin-toggle").onclick=async()=>{if(typeof playerState!=="undefined"&&playerState.stage==="player")await playerCall({action:"stop"});if(await run({action:"refresh"})){$("admin-panel").hidden=false;await pollScan();}};
$("admin-close").onclick=()=>{$("admin-panel").hidden=true;clearTimeout(timer);};
$("library-form").onsubmit=async e=>{e.preventDefault();if(await run({action:"save_library",video_root:$("video-root").value.trim()||null,expected_revision:state.library.revision}))message("Chemin enregistré sur le serveur.");};
$("preferences-form").onsubmit=async e=>{e.preventDefault();if(await run({action:"save_preferences",audio_language:$("audio-language").value,subtitle_language:$("subtitle-language").value,autoplay_seconds:$("autoplay").value==="off"?null:Number($("autoplay").value)}))message("Préférences du profil enregistrées.");};
$("theme").onchange=()=>run({action:"theme",theme:$("theme").value});
$("filter-form").onsubmit=e=>{e.preventDefault();offset=0;loadPage();};$("refresh-catalog").onclick=()=>loadPage();
$("view-mode").onchange=()=>$("media").classList.toggle("list",$("view-mode").value==="list");
$("poster-size").oninput=()=>document.documentElement.style.setProperty("--poster-size",$("poster-size").value+"px");
$("previous").onclick=()=>{offset=Math.max(0,offset-50);loadPage();};$("next").onclick=()=>{offset+=50;loadPage();};
$("scan").onclick=async()=>{if(await run({action:"start_scan"},false))await pollScan();};
async function pollScan(){clearTimeout(timer);if(state.stage!=="connected"||$("admin-panel").hidden)return;const s=await run({action:"scan_status"},false);if(!s){timer=setTimeout(pollScan,2000);return;}$("scan-status").textContent=s.running?"Scan en cours…":s.last_error||(s.count!==null?s.count+" vidéos découvertes.":"Aucun scan lancé dans cette session serveur.");if(s.running)timer=setTimeout(pollScan,2000);else if(s.count!==null&&state.profile_id)await loadPage();}
$("cancel-delete").onclick=()=>{$("confirm-delete").close();deleting=null;};
$("confirm-delete").addEventListener("cancel",()=>{deleting=null;});
$("confirm-delete-button").onclick=async()=>{if(!deleting)return;const p=deleting;$("confirm-delete").close();deleting=null;if(await run({action:"delete_profile",id:p.id,confirm_id:p.id})){$("media").replaceChildren();if(state.profile_id)await loadPage();}};
window.chillReady=(async()=>{if(!window.__TAURI__){message("Ouvrez cette interface depuis l'application Chill Center, pas directement dans un navigateur.");return;}const r=await run({action:"boot"});if(r?.profile_id)await loadPage();})();
