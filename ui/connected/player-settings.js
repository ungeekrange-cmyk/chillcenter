"use strict";
var uiText=globalThis.chillI18n?.t||((text)=>text);
(() => {
  const section=node("section"),form=node("form"),label=node("label",undefined,"checkbox-setting"),check=node("input"),status=node("p",undefined,"muted");
  section.id="player-presentation-settings";form.id="player-presentation-form";
  check.id="player-fullscreen-default";check.type="checkbox";check.checked=true;
  label.append(check,node("span",uiText("Ouvrir les vidéos en plein écran")));
  form.append(label,node("button",uiText("Enregistrer sur ce Mac")));
  section.append(node("h2",uiText("Affichage du lecteur")),form,node("p",uiText("Activé par défaut. Sinon, la vidéo occupe tout le panneau droit, avec des bandes noires si nécessaire. Ce choix ne change ni les autres appareils ni les préférences du profil."),"muted"),status);
  $("admin-panel").querySelector(".admin-grid").append(section);
  let saving=false;
  window.refreshPlayerPresentation=presentation=>{if(!saving)check.checked=presentation?.fullscreen_on_start!==false;};
  form.onsubmit=async event=>{
    event.preventDefault();if(saving)return;saving=true;status.textContent="";
    try{
      const result=await invoke({action:"save_player_settings",fullscreen_on_start:check.checked});
      state.player_presentation=result.player_presentation;check.checked=result.player_presentation.fullscreen_on_start;
      status.textContent=uiText("Affichage enregistré sur ce Mac. Appliqué à la prochaine vidéo.");
    }catch(error){check.checked=state.player_presentation?.fullscreen_on_start!==false;status.textContent=error.message||String(error);}
    finally{saving=false;}
  };
  window.chillReady?.then(()=>window.refreshPlayerPresentation(state.player_presentation));
})();
