"use strict";
// Exact, profile-scoped resume observations. Never infer duration from TMDB.
(function(root){
 function percent(entry){
  if(!entry||!Number.isSafeInteger(entry.position_ms)||entry.position_ms<0||!Number.isSafeInteger(entry.duration_ms)||entry.duration_ms<=0)return null;
  return Math.min(100,entry.position_ms/entry.duration_ms*100);
 }
 function started(entry,watched=false){const value=percent(entry);return !watched&&!entry?.watched&&value!==null&&value>0;}
 function inProgress(entry,watched=false){return started(entry,watched)&&percent(entry)>=5;}
 function parse(payload){
  if(!payload||typeof payload.revision!=="string"||payload.revision.length>255)throw Error("Invalid progress response");
  if(payload.unchanged===true)return null;
  if(!Array.isArray(payload.items)||payload.items.length>30000)throw Error("Invalid progress items");
  const result=new Map();
  for(const entry of payload.items){
   if(!entry||typeof entry.media_id!=="string"||!/^[0-9a-f-]{36}$/i.test(entry.media_id)||result.has(entry.media_id)||typeof entry.watched!=="boolean"||percent(entry)===null)throw Error("Invalid progress item");
   result.set(entry.media_id,entry);
  }
  return result;
 }
 if(typeof module!=="undefined")module.exports={percent,started,inProgress,parse};
 if(typeof document==="undefined")return;
 let scope="",revision=null,entries=new Map(),pending=null,only=false,unsupported=false,lastCheck=0;
 const context=()=>typeof state!=="undefined"&&state.stage==="connected"&&state.server_id&&state.profile_id?state.server_id+":"+state.profile_id:"";
 const cacheKey=key=>"chillcenter-measured-progress:"+key;
 function activeContext(){
  const key=context();if(scope===key)return key;
  scope=key;revision=null;entries=new Map();pending=null;unsupported=false;lastCheck=0;
  if(key)try{const payload=JSON.parse(localStorage.getItem(cacheKey(key))||"null"),saved=parse(payload);if(saved){entries=saved;revision=payload.revision;}}catch{};
  return key;
 }
 function entryFor(media){activeContext();return entries.get(media.id);}
 root.isProgressView=()=>only;
 root.leaveProgressView=()=>{only=false;};
 root.dropProgressProfile=id=>{try{localStorage.removeItem(cacheKey(state.server_id+":"+id));}catch{}};
 root.filterProgressItems=items=>only?items.filter(media=>inProgress(entryFor(media),media.watched)):items;
 root.renderMediaProgress=(poster,media)=>{
  const previous=poster.querySelector?.(".media-progress");previous?.remove();
  poster.dataset.progressMediaId=media.id;poster.dataset.progressWatched=String(Boolean(media.watched));
  const entry=entryFor(media);if(!started(entry,media.watched))return;
  const value=percent(entry),bar=node("div",undefined,"media-progress"),fill=node("span");
  bar.setAttribute("role","progressbar");bar.setAttribute("aria-label",uiText("Progression de lecture"));
  bar.setAttribute("aria-valuemin","0");bar.setAttribute("aria-valuemax","100");bar.setAttribute("aria-valuenow",String(Math.floor(value)));
  bar.title=uiText("Lecture en cours · {0} %",Math.floor(value));fill.style.width=value+"%";bar.append(fill);poster.append(bar);
 };
 root.renderDetailProgress=media=>{
  let label=$("detail-progress");if(!label){label=node("p",undefined,"detail-progress");label.id="detail-progress";label.setAttribute("role","status");$("detail-overview").before(label);}
  label.dataset.progressMediaId=media.id;label.dataset.progressWatched=String(Boolean(media.watched));
  const entry=entryFor(media);label.hidden=!started(entry,media.watched);label.textContent=label.hidden?"":uiText("Lecture en cours · {0} %",Math.floor(percent(entry)));
 };
 function redraw(){
  for(const poster of document.querySelectorAll(".poster[data-progress-media-id]"))root.renderMediaProgress(poster,{id:poster.dataset.progressMediaId,watched:poster.dataset.progressWatched==="true"});
  const label=$("detail-progress");if(label)root.renderDetailProgress({id:label.dataset.progressMediaId,watched:label.dataset.progressWatched==="true"});
  if(typeof loadPage==="function"&&context()&&typeof shellSection!=="undefined"&&shellSection==="catalog")void loadPage(true);
 }
 root.refreshProgress=async(force=false)=>{
  const key=activeContext();if(!key||unsupported&&!force&&Date.now()-lastCheck<300000)return;
  if(pending?.key===key){
   if(force&&!pending.after)pending.after=pending.promise.then(()=>context()===key?root.refreshProgress(true):undefined);
   return pending.after||pending.promise;
  }
  if(!force&&Date.now()-lastCheck<30000)return;
  lastCheck=Date.now();
  const task={key,promise:null};pending=task;
  task.promise=(async()=>{
   try{
    const payload=await invoke({action:"playback_progress",known_revision:revision});
    if(context()!==key||pending!==task||payload?.discarded)return;
    const updated=parse(payload);unsupported=false;if(updated){entries=updated;revision=payload.revision;try{localStorage.setItem(cacheKey(key),JSON.stringify(payload));}catch{}redraw();}
   }catch(error){if(context()===key&&pending===task&&(error?.status===404||error?.status===405)){unsupported=true;if(only)message(uiText("La progression mesurée nécessite une mise à jour du serveur."));}}
   finally{if(pending===task)pending=null;}
  })();
  return task.promise;
 };
 const toggle=$("show-in-progress");
 toggle.onclick=async()=>{if(!state.profile_id||!await stopPlayerForNavigation())return;only=true;$("category").value="";shellShow("catalog");scrollTo({top:0,behavior:"instant"});await loadPage();void root.refreshProgress(true);};
 setInterval(()=>{if(document.visibilityState!=="hidden")void root.refreshProgress();},30000);
 document.addEventListener("visibilitychange",()=>{if(document.visibilityState!=="hidden")void root.refreshProgress(true);});
})(globalThis);
