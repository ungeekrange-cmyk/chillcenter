"use strict";
let playerState={stage:"closed"},playerBusy=false,playerTimer=null,playerResize=null;
const videoRect=()=>{const r=$("native-video").getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};};
async function playerCall(command,quiet=false){
  if(playerBusy)return null;
  playerBusy=true;
  try{const next=await window.__TAURI__.core.invoke("player_command",{command});playerState=next;renderPlayer();return next;}
  catch(e){if(!quiet&&e.code!=="CLIENT_BUSY")message(e.message||String(e));return null;}
  finally{playerBusy=false;}
}
window.chillPlay=async media_id=>{
  $("player-panel").hidden=false;document.body.classList.add("player-open");$("player-note").textContent="Chargement de la vidéo…";
  const started=await playerCall({action:"start",media_id,rect:videoRect()});
  if(!started){hidePlayer();return;}
  clearInterval(playerTimer);playerTimer=setInterval(()=>playerCall({action:"status"},true),500);
};
function hidePlayer(){clearInterval(playerTimer);playerTimer=null;$("player-panel").hidden=true;document.body.classList.remove("player-open");playerState={stage:"closed"};}
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
  $("player-decoder").textContent=n.hardware_decoder&&n.hardware_decoder!=="no"?"Décodage matériel · "+n.hardware_decoder:"Décodage logiciel · "+(n.decoder||"initialisation");
}
$("player-pause").onclick=()=>playerCall({action:playerState.native?.paused?"resume":"pause"});
$("player-back").onclick=()=>playerCall({action:"jump",seconds:-10});$("player-forward").onclick=()=>playerCall({action:"jump",seconds:30});
$("player-speed").onchange=()=>playerCall({action:"speed",speed:Number($("player-speed").value)});
$("player-progress").onchange=()=>playerCall({action:"seek",position:Number($("player-progress").value)});
$("player-audio").onchange=()=>playerCall({action:"audio",track_id:Number($("player-audio").value)});
$("player-subtitle").onchange=()=>playerCall({action:"subtitle",track_id:$("player-subtitle").value==="off"?null:Number($("player-subtitle").value)});
$("player-fullscreen").onclick=()=>playerCall({action:"fullscreen"});
$("player-close").onclick=async()=>{if(await playerCall({action:"stop"})){hidePlayer();await loadPage();}};
const reposition=()=>{clearTimeout(playerResize);playerResize=setTimeout(()=>{if(!$("player-panel").hidden)playerCall({action:"mount",rect:videoRect()},true);},80);};
new ResizeObserver(reposition).observe($("native-video"));window.addEventListener("resize",reposition);window.addEventListener("scroll",reposition,{passive:true});
document.addEventListener("keydown",e=>{if($("player-panel").hidden||["INPUT","SELECT","TEXTAREA"].includes(e.target.tagName))return;
  if(e.code==="Space"){e.preventDefault();$("player-pause").click();}else if(e.code==="ArrowRight"){e.preventDefault();$("player-forward").click();}else if(e.code==="ArrowLeft"){e.preventDefault();$("player-back").click();}else if(e.key==="f")$("player-fullscreen").click();
});
