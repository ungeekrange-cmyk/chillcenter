"use strict";
let playerState={stage:"closed"},playerBusy=false,playerTimer=null,playerResize=null;
let nextKey=null,nextDeadline=null,nextTimer=null,nextCancelled=false;
let playerNavigationEpoch=0,completedPlayback=null;
const playbackScope=()=>typeof state==="undefined"?null:state.server_id+":"+state.profile_id;
let playerIdleTimer=null,playerIdleAt=Date.now(),playerHover=false,playerFullscreenRequest=null,playerAutoFullscreen=false;
const playerIdleDelay=2500;
function playerControlsPinned(){
  const n=playerState.native;
  return playerState.stage!=="player"||!n?.loaded||n.paused||n.buffering||n.ended||Boolean(n.error)||playerHover||Boolean(document.activeElement?.closest?.("#player-overlay, .player-heading"));
}
function updatePlayerControls(){
  clearTimeout(playerIdleTimer);playerIdleTimer=null;
  const panel=$("player-panel"),pinned=playerControlsPinned(),remaining=playerIdleDelay-(Date.now()-playerIdleAt);
  panel.classList.toggle("controls-hidden",!pinned&&remaining<=0);
  if(!panel.hidden&&!pinned&&remaining>0)playerIdleTimer=setTimeout(updatePlayerControls,remaining);
}
function showPlayerControls(){playerIdleAt=Date.now();updatePlayerControls();}
function syncPlayerFullscreen(){
  let fullscreen=Boolean(playerState.native?.fullscreen);
  if(playerFullscreenRequest){
    if(fullscreen===playerFullscreenRequest.enabled||Date.now()>=playerFullscreenRequest.until)playerFullscreenRequest=null;
    else fullscreen=playerFullscreenRequest.enabled;
  }
  document.body.classList.toggle("player-fullscreen",fullscreen);
  $("player-fullscreen").setAttribute("aria-label",fullscreen?"Quitter le plein écran":"Plein écran");
  $("player-fullscreen").setAttribute("aria-pressed",String(fullscreen));
}
async function togglePlayerFullscreen(enabled){
  const desired=enabled??!document.body.classList.contains("player-fullscreen");
  playerFullscreenRequest={enabled:desired,until:Date.now()+2000};syncPlayerFullscreen();showPlayerControls();reposition();
  const result=await playerCall({action:"fullscreen",enabled:desired});
  if(!result){playerFullscreenRequest=null;syncPlayerFullscreen();}
  reposition();
}
function playerIcon(id,kind,label){
  const paths={play:"M8 5v14l11-7z",pause:"M8 5v14M16 5v14",back:"M4 9a8 8 0 1 1 1 9M4 4v5h5",return:"M15 5l-7 7 7 7M8 12h13",forward:"M20 9a8 8 0 1 0-1 9M20 4v5h-5",fullscreen:"M8 3H3v5M16 3h5v5M3 16v5h5M21 16v5h-5"};
  const button=$(id);if(button.dataset.icon===kind)return;button.dataset.icon=kind;
  button.setAttribute("aria-label",label);button.title=label;
  const svg=document.createElementNS("http://www.w3.org/2000/svg","svg");svg.setAttribute("viewBox","0 0 24 24");svg.setAttribute("aria-hidden","true");
  const path=document.createElementNS("http://www.w3.org/2000/svg","path");path.setAttribute("d",paths[kind]);path.setAttribute("fill",kind==="play"?"currentColor":"none");path.setAttribute("stroke","currentColor");path.setAttribute("stroke-width","2");path.setAttribute("stroke-linecap","round");path.setAttribute("stroke-linejoin","round");svg.append(path);
  button.replaceChildren(svg);
  if(kind==="back"||kind==="forward"){const text=node("span",kind==="back"?"10":"30");text.className="player-jump-value";button.append(text);}
}
const videoRect=()=>{const r=$("native-video").getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};};
async function playerCall(command,quiet=false){
  if(playerBusy&&quiet)return null;
  while(playerBusy)await new Promise(resolve=>setTimeout(resolve,50));
  playerBusy=true;
  try{
    let next;
    for(let attempt=0;;attempt++){
      try{next=await window.__TAURI__.core.invoke("player_command",{command});break;}
      catch(e){if(e.code!=="CLIENT_BUSY"||quiet||attempt>=19)throw e;await new Promise(resolve=>setTimeout(resolve,80));}
    }
    playerState=next;renderPlayer();return next;
  }
  catch(e){if(!quiet&&e.code!=="CLIENT_BUSY")message(e.message||String(e));return null;}
  finally{playerBusy=false;}
}
window.chillPlay=async (media_id,context=null)=>{
  ++playerNavigationEpoch;completedPlayback=null;
  clearNext();
  // Snapshot this Mac's preference for a new manual launch, not each status tick
  // or automatic episode. A confirmed transfer uses the same pending intent.
  playerAutoFullscreen=typeof state==="undefined"||state.player_presentation?.fullscreen_on_start!==false;
  document.querySelectorAll("dialog[open]").forEach(dialog=>dialog.close());
  $("player-transfer").hidden=true;$("native-video").hidden=false;document.querySelector(".player-controls").hidden=false;document.querySelector(".player-seek").hidden=false;
  $("player-panel").hidden=false;document.body.classList.add("player-open");$("player-note").textContent="Chargement de la vidéo…";
  showPlayerControls();
  const started=await playerCall({action:"start",media_id,rect:videoRect(),context});
  if(!started){await playerCall({action:"stop"},true);hidePlayer();return;}
  clearInterval(playerTimer);playerTimer=setInterval(()=>playerCall({action:"status"},true),500);
};
function clearNext(){clearTimeout(nextTimer);nextTimer=null;nextKey=null;nextDeadline=null;nextCancelled=false;$("player-next").hidden=true;}
function hidePlayer(){clearNext();clearInterval(playerTimer);clearTimeout(playerIdleTimer);playerIdleTimer=null;playerTimer=null;playerFullscreenRequest=null;playerAutoFullscreen=false;playerHover=false;$("player-panel").hidden=true;$("player-panel").classList.remove("controls-hidden");document.body.classList.remove("player-open","player-fullscreen");playerState={stage:"closed"};}
function applyPlayerPresentation(){
  if(!playerAutoFullscreen||playerState.stage!=="player")return;
  playerAutoFullscreen=false;
  requestAnimationFrame(()=>{if(playerState.stage==="player"&&!playerState.native?.fullscreen)togglePlayerFullscreen(true);});
}
function cancelNext(){clearTimeout(nextTimer);nextTimer=null;nextDeadline=null;nextCancelled=true;$("player-next-count").textContent="Lancement automatique annulé.";}
async function cancelNextAndClose(){
  cancelNext();
  await returnFromPlayback();
}
async function returnFromPlayback(completion=null){
  const playback={media_id:playerState.media_id,context:playerState.context},scope=playbackScope();
  const epoch=++playerNavigationEpoch,current=()=>epoch===playerNavigationEpoch&&scope===playbackScope();
  if(!await stopPlayerForNavigation(current)||!current())return;
  // The originating panel remains visible while its local mirror catches up.
  // A newer launch/navigation/profile must never be replaced by this delayed return.
  if(completion)await completion;
  else if(typeof syncAfterMutation==="function")await syncAfterMutation();
  if(!current())return;
  if(window.catalogReturnFromPlayback)await window.catalogReturnFromPlayback(playback,current);
  else if(typeof loadPage==="function")await loadPage();
}
function observePlaybackCompletion(){
  const next=playerState.next;
  // A resolved next object (even with media:null) confirms EOF reporting and lease
  // release succeeded. A missing object or decoder error is not a terminal list.
  if(!playerState.native?.ended||playerState.native.error||!next)return;
  const key=playerNavigationEpoch+":"+playerState.media_id;
  if(completedPlayback?.key===key)return;
  const scope=playbackScope(),epoch=playerNavigationEpoch;
  const completion={key,promise:null};completedPlayback=completion;
  // renderPlayer runs inside playerCall's lock; defer sync/stop until it is released.
  setTimeout(()=>{
    if(epoch!==playerNavigationEpoch||scope!==playbackScope()||completedPlayback!==completion)return;
    completion.promise=typeof syncAfterMutation==="function"?Promise.resolve(syncAfterMutation()).catch(()=>{}):Promise.resolve();
    if(!next.media)void returnFromPlayback(completion.promise);
  },0);
}
async function launchNext(){
  if(playerBusy){nextTimer=setTimeout(launchNext,100);return;}
  const id=playerState.next?.media?.id;if(!id||!playerState.next.available)return;
  cancelNext();$("player-next-now").disabled=true;
  const started=await playerCall({action:"next",expected_id:id});
  if(!started)$("player-next-count").textContent="Pas de lancement. Vérifiez la disponibilité ou le lecteur déjà actif sur un autre appareil.";
  $("player-next-now").disabled=false;
}
function countdownNext(){
  if(nextCancelled||nextDeadline===null)return;
  const remaining=Math.max(0,Math.ceil((nextDeadline-Date.now())/1000));
  $("player-next-count").textContent="Lancement dans "+remaining+" s";
  if(remaining===0){launchNext();return;}
  nextTimer=setTimeout(countdownNext,250);
}
function renderNext(){
  observePlaybackCompletion();
  const next=playerState.next,media=next?.media;
  if(!playerState.native?.ended||playerState.native.error||!media){clearNext();return;}
  const key=playerState.media_id+":"+media.id;
  if(nextKey!==key){clearNext();nextKey=key;$("player-next").hidden=false;
    $("player-next-title").textContent=media.title;$("player-next-season").textContent=next.new_season?"Nouvelle saison":"À suivre";
    $("player-next-thumb").textContent=media.title.slice(0,1).toUpperCase();
    if(media.poster&&window.chillArtwork)window.chillArtwork($("player-next-thumb"),media.id);
    $("player-next-now").disabled=!next.available||!media.available;
    const thumb=$("player-next-thumb"),playable=Boolean(next.available&&media.available);thumb.tabIndex=playable?0:-1;thumb.setAttribute("role","button");thumb.setAttribute("aria-label",playable?"Lancer "+media.title:"Vidéo suivante indisponible");thumb.setAttribute("aria-disabled",String(!playable));
    if(!next.available||!media.available){$("player-next-count").textContent="Vidéo suivante indisponible.";nextCancelled=true;}
    else if(playerState.autoplay_seconds===null){$("player-next-count").textContent="Lancement automatique désactivé.";}
    else {requestAnimationFrame(()=>{if(nextKey===key&&!nextCancelled){nextDeadline=Date.now()+playerState.autoplay_seconds*1000;countdownNext();}});}
  }
}
function timeLabel(value){const seconds=Math.max(0,Math.floor(value||0));return [Math.floor(seconds/3600),Math.floor(seconds/60)%60,seconds%60].map(n=>String(n).padStart(2,"0")).join(":");}
function tracksSelect(id,tracks,selected,off){
  const select=$(id),signature=JSON.stringify(tracks.map(t=>[t.id,t.language,t.title,t.selected]));
  if(select.dataset.signature===signature)return;select.dataset.signature=signature;select.replaceChildren();
  if(off){const o=node("option","Désactivés");o.value="off";select.append(o);}
  for(const t of tracks){const o=node("option",[t.title,t.language].filter(Boolean).join(" · ")||"Piste "+t.id);o.value=String(t.id);select.append(o);}
  select.value=selected===undefined?(off?"off":select.firstChild?.value||""):String(selected);
}
function renderPlayer(){
  if(playerState.stage==="transfer"){
    $("player-panel").hidden=false;document.body.classList.add("player-open");clearNext();
    $("player-title").textContent=playerState.title;$("player-note").textContent=playerState.note;
    $("native-video").hidden=true;document.querySelector(".player-controls").hidden=true;document.querySelector(".player-seek").hidden=true;$("player-decoder").textContent="";
    $("player-transfer").hidden=false;$("transfer-confirm").hidden=playerState.confirmed;$("transfer-refuse").hidden=playerState.confirmed;
    $("transfer-wait").textContent=playerState.confirmed?"Attente de l'arrêt de l'autre appareil (au plus "+Math.ceil(playerState.remaining_ms/1000)+" s)…":"Sans confirmation, la lecture continue sur l'autre appareil.";
    showPlayerControls();syncPlayerFullscreen();
    return;
  }
  if(playerState.stage!=="player"){const error=playerState.error;hidePlayer();if(error)message(error);return;}
  $("player-transfer").hidden=true;$("native-video").hidden=false;document.querySelector(".player-controls").hidden=false;document.querySelector(".player-seek").hidden=false;
  const n=playerState.native;$("player-panel").hidden=false;document.body.classList.add("player-open");
  applyPlayerPresentation();
  $("player-title").textContent=playerState.title;$("player-note").textContent=playerState.note||n.error||n.notice||(n.ended?"Lecture terminée.":!n.loaded?"Chargement de la vidéo…":n.buffering?"Mise en mémoire tampon…":"");
  playerIcon("player-pause",n.paused?"play":"pause",n.paused?"Reprendre la lecture":"Mettre en pause");$("player-speed").value=String(n.speed);
  if($("player-volume")&&document.activeElement!==$("player-volume"))$("player-volume").value=String(n.volume??100);
  syncPlayerFullscreen();updatePlayerControls();
  const position=playerState.position||0,duration=playerState.duration||0;
  $("player-progress").max=String(duration);if(document.activeElement!==$("player-progress"))$("player-progress").value=String(position);
  $("player-progress").disabled=!duration;$("player-time").textContent=timeLabel(position)+" / "+timeLabel(duration);
  const audio=n.tracks.filter(t=>t.kind==="Audio"),subtitles=n.tracks.filter(t=>t.kind==="Subtitle");
  tracksSelect("player-audio",audio,audio.find(t=>t.selected)?.id,false);tracksSelect("player-subtitle",subtitles,subtitles.find(t=>t.selected)?.id,true);
  const external=playerState.external_subtitles||[],select=$("player-subtitle");
  for(const t of external){if(!select.querySelector('option[value="'+t.id+'"]')){const o=node("option",["Fichier externe",t.title,t.language,t.ambiguous?"choix manuel":null].filter(Boolean).join(" · "));o.value=t.id;o.disabled=t.unavailable;select.append(o);}}
  $("player-decoder").textContent=n.hardware_decoder&&n.hardware_decoder!=="no"?"Décodage matériel · "+n.hardware_decoder:"Décodage logiciel · "+(n.decoder||"initialisation");
  renderNext();
}
$("player-next-now").onclick=launchNext;$("player-next-cancel").onclick=cancelNextAndClose;
$("player-next-thumb").onclick=()=>{if($("player-next-thumb").getAttribute("aria-disabled")!=="true")launchNext();};$("player-next-thumb").onkeydown=event=>{if((event.key==="Enter"||event.key===" ")&&$("player-next-thumb").getAttribute("aria-disabled")!=="true"){event.preventDefault();launchNext();}};
$("transfer-confirm").onclick=()=>playerCall({action:"confirm_transfer"});$("transfer-refuse").onclick=()=>playerCall({action:"refuse_transfer"});
document.addEventListener("visibilitychange",()=>{if(document.hidden&&nextDeadline!==null)cancelNext();});
$("player-pause").onclick=()=>playerCall({action:playerState.native?.paused?"resume":"pause"});
$("player-back").onclick=()=>playerCall({action:"jump",seconds:-10});$("player-forward").onclick=()=>playerCall({action:"jump",seconds:30});
$("player-speed").onchange=()=>playerCall({action:"speed",speed:Number($("player-speed").value)});
$("player-progress").onchange=()=>playerCall({action:"seek",position:Number($("player-progress").value)});
$("player-audio").onchange=()=>playerCall({action:"audio",track_id:Number($("player-audio").value)});
$("player-subtitle").onchange=()=>{const value=$("player-subtitle").value;playerCall(value.startsWith("sub-")?{action:"external_subtitle",id:value}:{action:"subtitle",track_id:value==="off"?null:Number(value)});};
$("player-fullscreen").onclick=()=>togglePlayerFullscreen();
if($("player-volume"))$("player-volume").onchange=()=>playerCall({action:"volume",volume:Number($("player-volume").value)});
$("player-close").onclick=()=>returnFromPlayback();
async function stopPlayerForNavigation(current=null){
  if(!current){const epoch=++playerNavigationEpoch,scope=playbackScope();current=()=>epoch===playerNavigationEpoch&&scope===playbackScope();}
  playerAutoFullscreen=false;
  while(playerBusy&&current())await new Promise(resolve=>setTimeout(resolve,50));
  if(!current())return false;
  if(playerState.stage==="closed")return true;
  if(document.body.classList.contains("player-fullscreen")){await togglePlayerFullscreen(false);if(!current()||document.body.classList.contains("player-fullscreen"))return false;}
  while(playerBusy&&current())await new Promise(resolve=>setTimeout(resolve,50));
  if(!current())return false;
  if(!await playerCall({action:"stop"}))return false;
  if(!current())return false;
  hidePlayer();return true;
}
const reposition=()=>{clearTimeout(playerResize);playerResize=setTimeout(()=>{if(!$("player-panel").hidden&&playerState.stage==="player"){if(playerBusy){reposition();return;}playerCall({action:"mount",rect:videoRect()},true);}},80);};
new ResizeObserver(reposition).observe($("native-video"));window.addEventListener("resize",reposition);window.addEventListener("scroll",reposition,{passive:true});
$("player-panel").addEventListener("pointermove",showPlayerControls);
$("player-panel").addEventListener("pointerdown",showPlayerControls);
$("player-panel").addEventListener("focusin",showPlayerControls);
$("player-panel").addEventListener("focusout",()=>setTimeout(updatePlayerControls,0));
if($("player-overlay")){
  $("player-overlay").addEventListener("pointerenter",()=>{playerHover=true;showPlayerControls();});
  $("player-overlay").addEventListener("pointerleave",()=>{playerHover=false;showPlayerControls();});
}
playerIcon("player-back","back","Revenir de dix secondes");playerIcon("player-forward","forward","Avancer de trente secondes");playerIcon("player-fullscreen","fullscreen","Plein écran");playerIcon("player-pause","pause","Mettre en pause");
playerIcon("player-close","return","Revenir à la bibliothèque");
document.addEventListener("keydown",e=>{if($("player-panel").hidden)return;showPlayerControls();
  if(e.key==="Escape"&&document.body.classList.contains("player-fullscreen")){e.preventDefault();togglePlayerFullscreen(false);return;}
  if(["INPUT","SELECT","TEXTAREA","BUTTON"].includes(e.target.tagName)||e.target.isContentEditable)return;
  if(e.code==="Space"){e.preventDefault();$("player-pause").click();}else if(e.code==="ArrowRight"){e.preventDefault();$("player-forward").click();}else if(e.code==="ArrowLeft"){e.preventDefault();$("player-back").click();}else if(e.key.toLowerCase()==="f")$("player-fullscreen").click();
});
