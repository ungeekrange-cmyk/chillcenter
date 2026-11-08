"use strict";
// Update work deliberately bypasses run(), the global busy flag and catalogue.
(() => {
 const WEB_VERSION="__WEB_VERSION__";
 const visibleVersion=value=>typeof value?.version==="string"?value.version:null;
 // Display only: the signed installer remains the authority for eligibility.
 function hasNewerVersion(installed,available){
  const parse=value=>{const version=visibleVersion(value);if(!version||version.length>128)return null;const match=/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:\+[0-9A-Za-z.-]+)?$/.exec(version);return match?match.slice(1,4).map(BigInt):null;};
  const current=parse(installed),next=parse(available);if(!current||!next)return false;
  for(let index=0;index<3;index++){if(next[index]!==current[index])return next[index]>current[index];}return false;
 }
 const isUnsupportedServerError=error=>error?.code==="UPDATE_SERVER_UNSUPPORTED"||error?.code==="ROUTE_NOT_FOUND"||error?.status===404;
 function commandForPolicy(component,policy,hour,minute,mode){
  if(!["server","web","themes"].includes(component)||!["manual","check","automatic"].includes(mode)||!Number.isInteger(hour)||hour<0||hour>23||!Number.isInteger(minute)||minute<0||minute>59)throw Error("Invalid update policy");
  return {component,mode,hour,minute,expected_revision:policy.revision};
 }
 if(typeof module!=="undefined")module.exports={visibleVersion,hasNewerVersion,commandForPolicy,isUnsupportedServerError};
 if(typeof document==="undefined")return;
 const t=text=>globalThis.chillI18n?.t(text)||text,section=node("section"),summary=node("div"),rows=node("div"),note=node("p",undefined,"muted");
 section.id="update-settings";section.dataset.settingsCategory="general";section.append(node("h2",t("Versions et mises à jour")),summary,rows,note);$("admin-panel").querySelector(".admin-grid").append(section);
 const native=!window.ChillBridge;let localRefreshing=false,serverRefreshing=false,local=null,server=null,lastRefresh=0,macInstallBusy=false;
 const call=command=>window.__TAURI__.core.invoke("update_command",{command});
 const names={mac:"Client Mac",server:"Serveur",web:"Client web",themes:"Ambiances"};
 const resultLabels={NO_UPDATE:"Vous utilisez la dernière version",CHECK_FAILED:"Vérification impossible. Réessayez maintenant ou plus tard.",CHECK_INCOMPATIBLE:"Cette mise à jour nécessite une version compatible des autres composants.",CHECK_INTERRUPTED:"Vérification interrompue. Réessayez dans quelques minutes.",DEFERRED_BUSY_AUTO:"Installation différée : une lecture ou une tâche est active.",DEFERRED_BUSY_MANUAL:"Installation différée : une lecture ou une tâche est active. Elle reprendra automatiquement.",BUSY_RETRY_EXHAUSTED:"Installation non effectuée après plusieurs attentes. Vérifiez qu’aucune lecture ou tâche n’est active, puis réessayez.",RECHECK_REQUIRED:"Une nouvelle vérification est nécessaire avant l’installation.",LOCAL_RECOVERY_REQUIRED:"Une intervention locale est nécessaire avant de reprendre cette mise à jour.",APPLY_FAILED:"Installation non effectuée. La version actuelle est conservée.",AUTOMATIC_POLICY_CHANGED:"Installation annulée car la politique de mise à jour a changé.",AVAILABLE:"Mise à jour disponible",INSTALLED:"Mise à jour installée",PENDING:"En attente du service de mise à jour (moins d’une minute)",RUNNING:"Vérification en cours",SUCCEEDED:"Terminé",FAILED:"Échec"};
 const cards=new Map(),cardKey=(scope,component)=>scope+":"+component;
 const currentView=scope=>scope==="local"?local:server;
 function operationFor(view,component){return (view?.operations||[]).filter(op=>op.component===component).at(-1);}
 function statusText(op,detected,available){
  if(op?.status==="pending")return t(resultLabels.PENDING);
  if(op?.status==="running")return t(resultLabels.RUNNING);
  if(op?.status==="deferred_busy")return t(resultLabels[(op.result_code||"DEFERRED_BUSY_MANUAL").toUpperCase()]||resultLabels.DEFERRED_BUSY_MANUAL);
  if(detected)return `${t("Version disponible")} : ${available.version}`;
  const result=(op?.result_code||op?.status||"").toUpperCase();if(result==="AVAILABLE"&&!available)return t("Mise à jour détectée ; détails indisponibles. Réactualisez.");return t(resultLabels[result]||result);
 }
 function createCard(scope,component){
  const card=node("div"),form=node("form",undefined,"inline"),select=node("select"),time=node("input"),save=node("button",t("Enregistrer")),check=node("button",t("Vérifier maintenant"),"secondary"),install=node("button",t("Installer maintenant")),status=node("p",undefined,"muted"),heading=node("h3"),next=node("p",undefined,"muted"),hint=node("p",undefined,"muted"),record={scope,component,card,form,select,time,save,check,install,status,heading,next,hint,inFlight:false,error:"",policy:null,canCheck:false,supported:false,busy:false};
  card.dataset.updateScope=scope;card.dataset.updateComponent=component;time.type="time";check.type=install.type="button";select.setAttribute("aria-label",t("Mode de mise à jour"));time.setAttribute("aria-label",t("Heure locale de vérification"));
  for(const [value,label]of [["manual","Manuel"],["check","Vérification automatique"],...(component==="mac"?[]:[["automatic","Installation automatique"]])]){const option=node("option",t(label));option.value=value;select.append(option);}
  async function action(action,extra={}){
   if(record.inFlight||record.busy||component==="mac"&&(macInstallBusy||local?.busy))return;record.inFlight=true;record.error="";const installing=component==="mac"&&action==="install";if(installing){macInstallBusy=true;renderMacBadge();}record.check.disabled=record.install.disabled=true;record.status.textContent=t(component==="mac"?(installing?"Téléchargement et installation en cours…":"Vérification en cours"):"Demande enregistrée. Le service de mise à jour la traitera en moins d’une minute.");
   try{note.textContent="";await call(component==="mac"&&action==="install"?{action:"install",component:"mac"}:{action:(scope==="server"?"server_":"")+action,component,...extra});await refresh();}
   catch(error){const message=error.message||String(error);note.textContent=message;record.error=message;record.status.textContent=message;if(error.code?.includes("CHANGED"))await refresh();}
   finally{record.inFlight=false;if(installing)macInstallBusy=false;if(record.card.isConnected)renderCard(record,currentView(scope));renderMacBadge();}
  }
  check.onclick=()=>{void action("check");};install.onclick=()=>{void action("install",scope==="server"?{confirmed:true}:{});};
  form.onsubmit=event=>{event.preventDefault();try{const [hour,minute]=time.value.split(":").map(Number);void action("policy",commandForPolicy(component,record.policy||{},hour,minute,select.value));}catch(error){note.textContent=error.message;}};
  form.append(select,time,save);
  if(component==="mac"){form.remove();check.setAttribute("aria-label",t("Vérifier maintenant"));install.setAttribute("aria-label",t("Installer maintenant"));card.append(heading,check,install,status,hint);}else card.append(heading,form,next,check,install,status,hint);
  rows.append(card);cards.set(cardKey(scope,component),record);return record;
 }
 function renderMacBadge(){
  if(!native)return;let badge=$("mac-update-badge");if(!badge){badge=node("button","↑","update-badge");badge.id="mac-update-badge";badge.type="button";const toggle=$("admin-toggle"),wrap=node("div",undefined,"mac-update-control");wrap.id="mac-update-control";toggle.replaceWith(wrap);wrap.append(toggle,badge);}
  const available=local?.available?.mac,op=operationFor(local,"mac"),installBusy=macInstallBusy||(local?.busy&&hasNewerVersion(local?.installed?.mac,available)&&!["pending","running"].includes(op?.status))||op?.kind==="install"&&["pending","running","deferred_busy"].includes(op?.status),detected=hasNewerVersion(local?.installed?.mac,available)&&!(op?.kind==="check"&&["pending","running","failed"].includes(op.status)),record=cards.get(cardKey("local","mac"));
  badge.hidden=!detected&&!installBusy;badge.disabled=!detected||installBusy||local?.busy;const label=installBusy?t(macInstallBusy?"Installation en cours…":"Mise à jour en cours…"):record?.error?t("Échec de la mise à jour Mac. Réessayer."):t("Installer la mise à jour Mac maintenant");badge.setAttribute("aria-label",label);badge.title=label;badge.textContent=installBusy?"…":"↑";
  badge.onclick=()=>{const record=cards.get(cardKey("local","mac"));if(record)record.install.onclick();};
 }
 function renderCard(record,view){
  const {scope,component,select,time,heading,next,hint,save,check,install,status,form}=record,policy=view?.policies?.[component];if(component!=="mac"&&!policy)return;
  record.policy=policy;heading.textContent=t(names[component])+` · ${t(scope==="local"?"Cet appareil":"Serveur")} · ${visibleVersion(view.installed?.[component])||t("Indisponible")}`;
  // Never overwrite an in-progress policy edit. Status and action controls remain live.
  if(component!=="mac"&&!form.contains(document.activeElement)){select.value=policy.mode;time.value=String(policy.hour).padStart(2,"0")+":"+String(policy.minute).padStart(2,"0");}
  const supported=scope==="local"||view.helper_available,canCheck=supported||view.check_available===true,available=view.available?.[component],op=operationFor(view,component),busy=(scope==="local"&&view?.busy===true)||["pending","running","deferred_busy"].includes(op?.status),detected=hasNewerVersion(view.installed?.[component],available)&&!(op?.kind==="check"&&op.status==="failed")&&(component==="mac"?!(op?.kind==="check"&&["pending","running","deferred_busy"].includes(op?.status)):!busy);
  record.supported=supported;record.canCheck=canCheck;record.busy=busy;if(component!=="mac")save.disabled=record.inFlight||!supported;check.disabled=record.inFlight||!canCheck||busy||(component==="mac"&&macInstallBusy);install.hidden=!detected;install.disabled=record.inFlight||!supported||busy||(component==="mac"&&macInstallBusy);status.textContent=record.error||(scope==="local"&&view?.busy&&!["pending","running"].includes(op?.status)?t("Mise \u00e0 jour en cours\u2026"):statusText(op,detected,available));if(component!=="mac")next.textContent=`${t("Prochain contrôle à")} ${time.value} · ${view.timezone||t("Heure locale")}`;
  hint.textContent=!supported?t(canCheck?"La recherche est disponible sans service d’installation. Installez ce service sur le serveur pour autoriser les installations et leur planification.":"Mettez à jour le serveur pour rechercher les versions sans service d’installation."):"";
 }
 function render(force=false){
  summary.replaceChildren();
  const installed={...(server?.installed||{}),...(local?.installed||{})};if(!native)installed.web={version:WEB_VERSION};
  for(const key of [native?"mac":"web","server","themes"])summary.append(node("p",`${t(names[key])} : ${visibleVersion(installed[key])||t("Indisponible")}`));
  summary.append(node("p",`API : ${server?.installed?.api||local?.installed?.api||"2026-10-14"}`,"muted"));
  if(server?.unsupported)summary.append(node("p",t("Ce serveur ne gère pas encore les mises à jour. Le reste de l’application reste disponible."),"muted"));
  const expectedCards=new Set();
  for(const [scope,view,components]of [["local",local,native?["mac","themes"]:[]],["server",server,["server","web","themes"]]]){
   if(!view||view.unsupported)continue;
   for(const component of components){if(component!=="mac"&&!view.policies?.[component])continue;const key=cardKey(scope,component);expectedCards.add(key);renderCard(cards.get(key)||createCard(scope,component),view);}
  }
  for(const [key,record]of cards)if(!expectedCards.has(key)){record.card.remove();cards.delete(key);}
  renderMacBadge();
 }
 async function refreshServer(){if(serverRefreshing)return;serverRefreshing=true;try{server=await call({action:"server_status"});render();}catch(error){if(isUnsupportedServerError(error)){server={unsupported:true,installed:server?.installed};}else{note.textContent=error?.message||"État du serveur temporairement indisponible. Les dernières informations connues sont conservées.";}render();}finally{serverRefreshing=false;}}
 async function refresh(force=false){if(localRefreshing)return;localRefreshing=true;const before=visibleVersion((native?local:server)?.installed?.themes);try{local=await call({action:"status"});if(!native)server=local;const after=visibleVersion((native?local:server)?.installed?.themes);if(before&&after&&before!==after)void window.reloadThemePack?.();render(force);lastRefresh=Date.now();if(native)void refreshServer();}catch(error){note.textContent=error.message||String(error);}finally{localRefreshing=false;}}
 new MutationObserver(()=>{if(!$("admin-panel").hidden)void refresh();}).observe($("admin-panel"),{attributes:true,attributeFilter:["hidden"]});
 function schedulePoll(){if(typeof setTimeout!=="function")return;const op=operationFor(local,"mac"),fast=!$("admin-panel").hidden||macInstallBusy||["pending","running","deferred_busy"].includes(op?.status);setTimeout(()=>{if(document.hidden)schedulePoll();else void refresh().finally(schedulePoll);},fast?10000:60000);}
 window.chillUpdates={refresh};const start=()=>{void refresh().finally(schedulePoll);};if(native)start();else window.chillReady?.then(start);
})();
