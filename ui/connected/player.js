"use strict";
let playerState={stage:"closed"},playerBusy=false,playerTimer=null,playerResize=null;
let nextKey=null,nextDeadline=null,nextTimer=null,nextCancelled=false;
const videoRect=()=>{const r=$("native-video").getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};};
async function playerCall(command,quiet=false){
  if(playerBusy)return null;
  playerBusy=true;
  try{const next=await window.__TAURI__.core.invoke("player_command",{command});playerState=next;renderPlayer();return next;}
  catch(e){if(!quiet&&e.code!=="CLIENT_BUSY")message(e.message||String(e));return null;}
  finally{playerBusy=false;}
}
window.chillPlay=async (media_id,context=null)=>{
  clearNext();
  $("player-panel").hidden=false;document.body.classList.add("player-open");$("player-note").textContent="Chargement de la vidéo…";
  const started=await playerCall({action:"start",media_id,rect:videoRect(),context});
  if(!started){hidePlayer();return;}
  clearInterval(playerTimer);playerTimer=setInterval(()=>playerCall({action:"status"},true),500);
};
function clearNext(){clearTimeout(nextTimer);nextTimer=null;nextKey=null;nextDeadline=null;nextCancelled=false;$("player-next").hidden=true;}
function hidePlayer(){clearNext();clearInterval(playerTimer);playerTimer=null;$("player-panel").hidden=true;document.body.classList.remove("player-open");playerState={stage:"closed"};}
function cancelNext(){clearTimeout(nextTimer);nextTimer=null;nextDeadline=null;nextCancelled=true;$("player-next-count").textContent="Lancement automatique annulé.";}
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
  const next=playerState.next,media=next?.media;
  if(!playerState.native?.ended||playerState.native.error||!media){clearNext();return;}
  const key=playerState.media_id+":"+media.id;
  if(nextKey!==key){clearNext();nextKey=key;$("player-next").hidden=false;
    $("player-next-title").textContent=media.title;$("player-next-season").textContent=next.new_season?"Nouvelle saison":"À suivre";
    $("player-next-thumb").textContent=media.title.slice(0,1).toUpperCase();
    if(media.poster&&window.chillArtwork)window.chillArtwork($("player-next-thumb"),media.id);
    $("player-next-now").disabled=!next.available||!media.available;
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
  if(playerState.stage!=="player"){hidePlayer();return;}
  const n=playerState.native;$("player-panel").hidden=false;document.body.classList.add("player-open");
  $("player-title").textContent=playerState.title;$("player-note").textContent=playerState.note||n.error||n.notice||(n.ended?"Lecture terminée.":!n.loaded?"Chargement de la vidéo…":n.buffering?"Mise en mémoire tampon…":"");
  $("player-pause").textContent=n.paused?"Lecture":"Pause";$("player-speed").value=String(n.speed);
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
$("player-next-now").onclick=launchNext;$("player-next-cancel").onclick=cancelNext;
document.addEventListener("visibilitychange",()=>{if(document.hidden&&nextDeadline!==null)cancelNext();});
$("player-pause").onclick=()=>playerCall({action:playerState.native?.paused?"resume":"pause"});
$("player-back").onclick=()=>playerCall({action:"jump",seconds:-10});$("player-forward").onclick=()=>playerCall({action:"jump",seconds:30});
$("player-speed").onchange=()=>playerCall({action:"speed",speed:Number($("player-speed").value)});
$("player-progress").onchange=()=>playerCall({action:"seek",position:Number($("player-progress").value)});
$("player-audio").onchange=()=>playerCall({action:"audio",track_id:Number($("player-audio").value)});
$("player-subtitle").onchange=()=>{const value=$("player-subtitle").value;playerCall(value.startsWith("sub-")?{action:"external_subtitle",id:value}:{action:"subtitle",track_id:value==="off"?null:Number(value)});};
$("player-fullscreen").onclick=()=>playerCall({action:"fullscreen"});
$("player-close").onclick=async()=>{if(await playerCall({action:"stop"})){hidePlayer();await loadPage();}};
const reposition=()=>{clearTimeout(playerResize);playerResize=setTimeout(()=>{if(!$("player-panel").hidden)playerCall({action:"mount",rect:videoRect()},true);},80);};
new ResizeObserver(reposition).observe($("native-video"));window.addEventListener("resize",reposition);window.addEventListener("scroll",reposition,{passive:true});
document.addEventListener("keydown",e=>{if($("player-panel").hidden||["INPUT","SELECT","TEXTAREA"].includes(e.target.tagName))return;
  if(e.code==="Space"){e.preventDefault();$("player-pause").click();}else if(e.code==="ArrowRight"){e.preventDefault();$("player-forward").click();}else if(e.code==="ArrowLeft"){e.preventDefault();$("player-back").click();}else if(e.key==="f")$("player-fullscreen").click();
});
