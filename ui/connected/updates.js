"use strict";
// Update work deliberately bypasses run(), the global busy flag and catalogue.
(() => {
 const WEB_VERSION="__WEB_VERSION__";
 const visibleVersion=value=>typeof value?.version==="string"?value.version:null;
 function commandForPolicy(component,policy,hour,minute,mode){
  if(!["mac","server","web","themes"].includes(component)||!["manual","check","automatic"].includes(mode)||!Number.isInteger(hour)||hour<0||hour>23||!Number.isInteger(minute)||minute<0||minute>59)throw Error("Invalid update policy");
  if(component==="mac"&&mode==="automatic")throw Error("Mac installation requires confirmation");
  return {component,mode,hour,minute,expected_revision:policy.revision};
 }
 if(typeof module!=="undefined")module.exports={visibleVersion,commandForPolicy};
 if(typeof document==="undefined")return;
 const t=text=>globalThis.chillI18n?.t(text)||text,section=node("section"),summary=node("div"),rows=node("div"),note=node("p",undefined,"muted");
 section.id="update-settings";section.dataset.settingsCategory="general";section.append(node("h2",t("Versions et mises à jour")),summary,rows,note);$("admin-panel").querySelector(".admin-grid").append(section);
 const native=!window.ChillBridge;let refreshing=false,local=null,server=null,lastRefresh=0;
 const call=command=>window.__TAURI__.core.invoke("update_command",{command});
 const names={mac:"Client Mac",server:"Serveur",web:"Client web",themes:"Ambiances"};
 const resultLabels={AVAILABLE:"Mise à jour disponible",NO_UPDATE:"Vous utilisez la dernière version",CHECK_FAILED:"Vérification impossible. Réessayez plus tard.",DEFERRED_BUSY:"Installation différée pendant l’utilisation",INSTALLED:"Mise à jour installée",pending:"En attente",running:"En cours",succeeded:"Terminé",failed:"Échec",deferred_busy:"Installation différée pendant l’utilisation"};
 function render(force=false){
  // Polling must never destroy an unsaved settings edit or steal focus.
  if(!force&&rows.contains(document.activeElement))return;
  summary.replaceChildren();rows.replaceChildren();
  const installed={...(server?.installed||{}),...(local?.installed||{})};if(!native)installed.web={version:WEB_VERSION};
  for(const key of [native?"mac":"web","server","themes"])summary.append(node("p",`${t(names[key])} : ${visibleVersion(installed[key])||t("Indisponible")}`));
  summary.append(node("p",`API : ${server?.installed?.api||local?.installed?.api||"2026-10-14"}`,"muted"));
  if(server?.unsupported)summary.append(node("p",t("Ce serveur ne gère pas encore les mises à jour. Le reste de l’application reste disponible."),"muted"));
  for(const [scope,view,components]of [["local",local,native?["mac","themes"]:[]],["server",server,["server","web","themes"]]]){
   if(!view||view.unsupported)continue;
   for(const component of components){
    const policy=view.policies?.[component];if(!policy)continue;
    const card=node("div"),form=node("form",undefined,"inline"),select=node("select"),time=node("input"),save=node("button",t("Enregistrer")),check=node("button",t("Vérifier maintenant"),"secondary"),install=node("button",t("Installer maintenant")),status=node("p",undefined,"muted");
    card.append(node("h3",t(names[component])+` · ${t(scope==="local"?"Cet appareil":"Serveur")} · ${visibleVersion(view.installed?.[component])||t("Indisponible")}`));
    for(const [value,label]of [["manual","Manuel"],["check","Vérification automatique"],...(component==="mac"?[]:[["automatic","Installation automatique"]])]){const option=node("option",t(label));option.value=value;select.append(option);}
    select.value=policy.mode;select.setAttribute("aria-label",t("Mode de mise à jour"));time.type="time";time.value=String(policy.hour).padStart(2,"0")+":"+String(policy.minute).padStart(2,"0");time.setAttribute("aria-label",t("Heure locale de vérification"));
    form.append(select,time,save);check.type=install.type="button";
    const supported=scope==="local"||view.helper_available;for(const button of [save,check,install])button.disabled=!supported;
    const available=view.available?.[component],op=(view.operations||[]).filter(op=>op.component===component).at(-1);
    const result=op?.result_code||op?.status||"";
    status.textContent=!supported?t("Installez le service de mise à jour sur le serveur pour activer cette commande."):available?.version?`${t("Version disponible")} : ${available.version}`:t(resultLabels[result]||result);
    async function action(action,extra={}){check.disabled=install.disabled=true;status.textContent=t("Vérification en arrière-plan…");try{await call({action:(scope==="server"?"server_":"")+action,component,...extra});await refresh(true);}catch(error){note.textContent=error.message||String(error);if(error.code?.includes("CHANGED"))await refresh(true);}finally{if(card.isConnected)check.disabled=install.disabled=!supported;}}
    check.onclick=()=>action("check");install.onclick=()=>{void action("install",scope==="server"?{confirmed:true}:{});};
    form.onsubmit=event=>{event.preventDefault();try{const [hour,minute]=time.value.split(":").map(Number);void action("policy",commandForPolicy(component,policy,hour,minute,select.value));}catch(error){note.textContent=error.message;}};
    card.append(form,node("p",`${t("Prochain contrôle à")} ${time.value} · ${view.timezone||t("Heure locale")}`,"muted"),check,install,status);rows.append(card);
   }
  }
  if(native)summary.append(node("p",t("Sur Mac, l’installation demande toujours votre validation. La vérification automatique fonctionne lorsque l’application est ouverte."),"muted"));
 }
 async function refresh(force=false){if(refreshing)return;refreshing=true;const before=visibleVersion((native?local:server)?.installed?.themes);try{local=await call({action:"status"});try{server=native?await call({action:"server_status"}):local;}catch(_){server={unsupported:true,installed:server?.installed};}const after=visibleVersion((native?local:server)?.installed?.themes);if(before&&after&&before!==after)void window.reloadThemePack?.();render(force);}catch(error){note.textContent=error.message||String(error);}finally{refreshing=false;lastRefresh=Date.now();}}
 new MutationObserver(()=>{if(!$("admin-panel").hidden)void refresh();}).observe($("admin-panel"),{attributes:true,attributeFilter:["hidden"]});
 window.chillUpdates={refresh};window.chillReady?.then(()=>{void refresh();setInterval(()=>{if(!document.hidden&&(!$("admin-panel").hidden||Date.now()-lastRefresh>=60000))void refresh();},10000);});
})();
