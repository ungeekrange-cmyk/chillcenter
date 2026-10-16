"use strict";

const appearanceLabels={spring:"Printemps",summer:"Été",autumn:"Automne",winter:"Hiver",christmas:"Noël",halloween:"Halloween"};
const systemTone=window.matchMedia("(prefers-color-scheme: dark)");
let appearanceTimer=null,appearanceRetry=null;

// Decoration is local, non-interactive and bounded. No animation loop on the JS thread.
(() => {
 const sidebar=document.querySelector(".sidebar");
 if(!sidebar)return;
 const weather=node("div",undefined,"ambiance-weather");
 weather.setAttribute("aria-hidden","true");
 for(const [kind,count]of [["snow",14],["bat",5]]){
  for(let i=0;i<count;i++){
   const particle=node("span",undefined,"ambiance-"+kind);
   particle.style.setProperty("--particle-x",((i*37+8)%90)+"%");
   particle.style.setProperty("--particle-size",(2+i%3)+"px");
   particle.style.setProperty("--particle-duration",(kind==="snow"?14+i%7:17+i*2)+"s");
   particle.style.setProperty("--particle-delay",(-i*3.7-2)+"s");
   if(kind==="bat")particle.append(node("i"));
   weather.append(particle);
  }
 }
 sidebar.prepend(weather);
 const visibility=()=>document.documentElement.dataset.ambiancePaused=String(document.hidden);
 document.addEventListener("visibilitychange",visibility);
 visibility();
})();

window.refreshAppearance=async()=>{
 clearTimeout(appearanceRetry);
 const date=new Date();
 try{
  const r=await invoke({action:"resolve_appearance",year:date.getFullYear(),month:date.getMonth()+1,day:date.getDate()});
  const tone=state.theme==="dark"||((state.theme||"system")==="system"&&systemTone.matches)?"dark":"light";
  document.documentElement.dataset.tone=tone;
  document.documentElement.dataset.ambiance=r.ambiance||"off";
  $("brand-logo").src=r.ambiance?"branding/"+r.ambiance+"-"+tone+".svg":"branding/default.svg";
  // The native bridge owns device-local preferences; no browser storage is introduced here.
  $("ambiance-mode").value=r.appearance.mode;
  $("ambiance-choice").value=r.appearance.manual;
  $("ambiance-choice-label").hidden=r.appearance.mode!=="manual";
  $("ambiance-status").textContent=r.ambiance?"Ambiance "+appearanceLabels[r.ambiance]+" · sur ce Mac uniquement":"Sans ambiance · thème conservé";
 }catch(e){
  if(e.code==="CLIENT_BUSY")appearanceRetry=setTimeout(refreshAppearance,500);
 }
 clearTimeout(appearanceTimer);
 const next=new Date(date.getFullYear(),date.getMonth(),date.getDate()+1);
 appearanceTimer=setTimeout(refreshAppearance,Math.max(1000,next-date+100));
};

const saveAppearance=async()=>{
 try{
  await invoke({action:"save_appearance",appearance:{mode:$("ambiance-mode").value,manual:$("ambiance-choice").value}});
  await refreshAppearance();
 }catch(e){
  message(e.message||String(e));
 }
};

$("ambiance-mode").onchange=saveAppearance;
$("ambiance-choice").onchange=saveAppearance;
systemTone.addEventListener("change",refreshAppearance);
window.addEventListener("focus",refreshAppearance);
window.addEventListener("pageshow",refreshAppearance);
document.addEventListener("visibilitychange",()=>{if(!document.hidden)refreshAppearance();});
window.chillReady.then(refreshAppearance);
