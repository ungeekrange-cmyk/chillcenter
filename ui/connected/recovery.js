"use strict";
// A rejected CAS write is never retried automatically. Only explicit reads may
// refresh its revision; drafts stay editable and the user decides whether to save.
function isRevisionConflict(error){return error?.status===409&&["PROFILE_CONFLICT","CONFIG_CONFLICT"].includes(error.code);}
function captureRecoveryFields(root){return Array.from(root?.querySelectorAll("input,textarea,select")||[]).filter(n=>!["password","file"].includes(n.type)).map((n,index)=>({node:n,id:n.id,name:n.name,index,value:n.value,checked:n.checked}));}
function restoreRecoveryFields(fields,root){
 const controls=Array.from(root?.querySelectorAll("input,textarea,select")||[]);
 for(const item of fields||[]){const n=controls.includes(item.node)?item.node:item.id?controls.find(n=>n.id===item.id):item.name?controls.find(n=>n.name===item.name):null;if(!n||["password","file"].includes(n.type))continue;n.value=item.value;if(typeof item.checked==="boolean")n.checked=item.checked;}
}
if(typeof module!=="undefined")module.exports={isRevisionConflict,captureRecoveryFields,restoreRecoveryFields};
if(typeof window!=="undefined")(()=>{
 let pending=null;
 const scope=()=>state.server_id+":"+state.profile_id;
 function dismiss(){pending?.box.remove();pending=null;}
 async function refreshCatalogue(){const result=await syncAfterMutation();if(result?.error)throw result.error;if(result?.discarded)throw Error(uiText("Actualisation différée. Réessayez dans un instant."));}
 window.captureRecoveryFields=captureRecoveryFields;window.restoreRecoveryFields=restoreRecoveryFields;window.recoveryRefreshCatalogue=refreshCatalogue;
 window.dismissConflictRecovery=dismiss;
 window.recoveryContextChanged=()=>{if(pending&&!pending.target.current()&&(!pending.refreshing||pending.context!==scope()||!(pending.target.identityCurrent||pending.target.current)()))dismiss();};
 window.conflictTarget=command=>{
  if(command.expected_revision===undefined&&command.expected_epoch===undefined&&command.expected_catalogue_epoch===undefined)return null;
  const context=scope(),current=()=>context===scope()&&state.stage==="connected";
  const target=window.catalogConflictTarget?.(command);if(target)return target;
  if(command.action==="save_library"||command.action==="rename_profile")return {
   root:$("admin-panel"),label:command.action==="save_library"?"Actualiser les réglages de bibliothèque":"Actualiser les profils",
   current:()=>current()&&!$("admin-panel").hidden,
   async reload(){
    const fresh=await invoke({action:"refresh"});if(!this.current())return false;
    // Capture at completion, not before the await: typing during the read wins.
    const fields=captureRecoveryFields(document),libraryDraft=window.captureLibraryDraft?.();state=fresh;render();window.restoreLibraryDraft?.(libraryDraft);restoreRecoveryFields(fields,document);return true;
   }
  };
  if(command.action==="forget_unavailable")return {root:$("unavailable-admin"),label:"Actualiser les vidéos indisponibles",current:()=>current()&&!$("admin-panel").hidden,async reload(){await unavailableAdmin();return this.current();}};
  return {root:document.querySelector("dialog[open]")||$("catalog-panel"),label:"Actualiser le catalogue",current,async reload(){await refreshCatalogue();return this.current();}};
 };
 window.offerConflictRecovery=(error,target)=>{
  if(!isRevisionConflict(error)||!target?.current())return;
  dismiss();const box=node("section",undefined,"conflict-recovery"),text=node("p",uiText("Les données ont changé ailleurs. Votre saisie est conservée. Actualisez avant de vérifier puis enregistrer à nouveau.")),reload=node("button",uiText(target.label),"secondary"),close=node("button",uiText("Ignorer"),"secondary");
  box.setAttribute("role","status");box.setAttribute("aria-live","polite");reload.type=close.type="button";box.append(text,reload,close);target.root.prepend(box);const item=pending={target,box,context:scope(),refreshing:false};close.onclick=dismiss;
  reload.onclick=async()=>{
   if(pending!==item||reload.disabled)return;if(!target.current()){dismiss();return;}
   item.refreshing=true;reload.disabled=true;text.textContent=uiText("Actualisation en cours… Le reste de l’application reste disponible.");
   try{const applied=await target.reload();if(pending!==item)return;if(applied){dismiss();message(uiText("Données actualisées. Votre saisie est conservée : vérifiez-la puis enregistrez explicitement."));}else dismiss();}
   catch(error){if(pending===item&&target.current())text.textContent=uiText("Actualisation impossible. Votre saisie est conservée ; réessayez quand le serveur est disponible.");}
   finally{item.refreshing=false;reload.disabled=false;window.recoveryContextChanged();}
  };
 };
 document.addEventListener("close",window.recoveryContextChanged,true);
})();
