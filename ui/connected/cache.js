"use strict";
var uiText=globalThis.chillI18n?.t||((text)=>text);
(() => {
  const section=node("section"),form=node("form"),forward=node("input"),back=node("input"),status=node("p",undefined,"muted"),save=node("button",uiText("Enregistrer sur ce Mac"));
  section.id="playback-cache-settings";form.id="playback-cache-form";
  forward.id="cache-forward-mib";back.id="cache-back-mib";
  for(const [input,min,max] of [[forward,16,512],[back,0,256]]){
    input.type="number";input.min=String(min);input.max=String(max);input.step="1";input.required=true;
  }
  const label=(text,input)=>{const l=node("label",text);l.append(input);return l;};
  form.append(label(uiText("Tampon avant (Mio)"),forward),label(uiText("Tampon arrière (Mio)"),back),save);
  section.append(node("h2",uiText("Tampon vidéo de ce Mac")),form,node("p",uiText("Valeurs par défaut : 128 Mio avant, 64 Mio arrière. Cible : 60 secondes d’avance ; retour d’environ 30 secondes selon le débit. La mémoire disponible limite la durée réellement conservée. 0 Mio arrière désactive le tampon de retour. Application à la prochaine lecture, sans changer les autres appareils."),"muted"),status);
  $("admin-panel").querySelector(".admin-grid").append(section);
  let saving=false;
  function draw(cache){forward.value=String(cache.forward_mib);back.value=String(cache.back_mib);}
  window.refreshPlaybackCache=cache=>{
    if(saving)return;
    if(cache)draw(cache);
  };
  form.onsubmit=async event=>{
    event.preventDefault();if(saving||!form.reportValidity())return;
    saving=true;for(const input of [forward,back,save])input.disabled=true;status.textContent="";
    try{
      const result=await invoke({action:"save_cache_settings",cache:{forward_mib:Number(forward.value),back_mib:Number(back.value)}});
      state.playback_cache=result.cache;draw(result.cache);status.textContent=uiText("Tampon enregistré sur ce Mac. Il sera appliqué à la prochaine lecture.");
    }catch(error){status.textContent=error.message||String(error);}
    finally{saving=false;for(const input of [forward,back,save])input.disabled=false;}
  };
  window.chillReady.then(()=>window.refreshPlaybackCache(state.playback_cache));
})();
