"use strict";
const appearanceLabels={spring:"Printemps",summer:"Été",autumn:"Automne",winter:"Hiver",christmas:"Noël",halloween:"Halloween"};
const systemTone=window.matchMedia("(prefers-color-scheme: dark)");let appearanceTimer=null,appearanceRetry=null;
window.refreshAppearance=async()=>{
 clearTimeout(appearanceRetry);const date=new Date();
 try{const r=await invoke({action:"resolve_appearance",year:date.getFullYear(),month:date.getMonth()+1,day:date.getDate()});
  const tone=state.theme==="dark"||((state.theme||"system")==="system"&&systemTone.matches)?"dark":"light";
  document.documentElement.dataset.tone=tone;document.documentElement.dataset.ambiance=r.ambiance||"off";
  $("brand-logo").src=r.ambiance?"branding/"+r.ambiance+"-"+tone+".svg":"branding/default.svg";
  $("ambiance-mode").value=r.appearance.mode;$("ambiance-choice").value=r.appearance.manual;$("ambiance-choice-label").hidden=r.appearance.mode!=="manual";
  $("ambiance-status").textContent=r.ambiance?"Ambiance "+appearanceLabels[r.ambiance]+" · sur ce Mac uniquement":"Sans ambiance · thème conservé";
 }catch(e){if(e.code==="CLIENT_BUSY")appearanceRetry=setTimeout(refreshAppearance,500);}
 clearTimeout(appearanceTimer);const next=new Date(date.getFullYear(),date.getMonth(),date.getDate()+1);appearanceTimer=setTimeout(refreshAppearance,Math.max(1000,next-date+100));
};
const saveAppearance=async()=>{try{await invoke({action:"save_appearance",appearance:{mode:$("ambiance-mode").value,manual:$("ambiance-choice").value}});await refreshAppearance();}catch(e){message(e.message||String(e));}};
$("ambiance-mode").onchange=saveAppearance;$("ambiance-choice").onchange=saveAppearance;
systemTone.addEventListener("change",refreshAppearance);window.addEventListener("focus",refreshAppearance);window.addEventListener("pageshow",refreshAppearance);document.addEventListener("visibilitychange",()=>{if(!document.hidden)refreshAppearance();});
window.chillReady.then(refreshAppearance);
