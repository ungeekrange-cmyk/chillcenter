"use strict";
var uiText=globalThis.chillI18n?.t||((text)=>text);
let shellSection="catalog",shellSearchTimer=null;
const categoryNames={"":uiText("Toute la collection"),film:uiText("Films"),serie:uiText("Séries"),spectacle:uiText("Spectacles"),documentaire:uiText("Documentaires")};
window.renderShell=()=>{
 const connected=state.stage==="connected",active=connected&&Boolean(state.profile_id);
 $("advice-toggle").hidden=!active;document.querySelector(".global-search").hidden=!active;
 document.querySelectorAll("[data-category],#show-sagas,#show-playlists").forEach(button=>button.disabled=!active);
 $("connected-address").textContent=state.address||"";
 if(!connected)shellSection="catalog";
 $("advice-panel").hidden=!active||shellSection!=="advice";
 window.aiAdviceVisibilityChanged?.(!$("advice-panel").hidden);
 $("catalog-panel").hidden=!active||shellSection!=="catalog";
 for(const [id,section] of [["collections-panel","collections"],["collection-panel","collection"],["picker-panel","picker"]])$(id).hidden=!active||shellSection!==section;
 for(const [id,kind] of [["show-sagas","saga"],["show-playlists","playlist"]])$(id).classList.toggle("active",active&&["collections","collection","picker"].includes(shellSection)&&collectionKind===kind);
 if(shellSection!=="admin")$("admin-panel").hidden=true;
 document.querySelectorAll("[data-category]").forEach(button=>button.classList.toggle("active",shellSection==="catalog"&&button.dataset.category===$("category").value));
 $("admin-toggle").classList.toggle("active",shellSection==="admin");$("advice-toggle").classList.toggle("active",shellSection==="advice");
 $("catalog-title").textContent=categoryNames[$("category").value]||uiText("Toute la collection");
};
function shellShow(section){if(!["collections","collection","picker"].includes(section))window.leaveCollectionPages?.();shellSection=section;window.recoveryContextChanged?.();window.renderShell();}
document.querySelectorAll("[data-category]").forEach(button=>button.onclick=async()=>{
 if(!state.profile_id)return;
 if(!await stopPlayerForNavigation())return;
 $("category").value=button.dataset.category;shellShow("catalog");scrollTo({top:0,behavior:"instant"});await loadPage();
});
$("advice-toggle").onclick=async()=>{if(!await stopPlayerForNavigation())return;shellShow("advice");};
$("admin-close").addEventListener("click",()=>shellShow("catalog"));
$("search").oninput=()=>{clearTimeout(shellSearchTimer);shellSearchTimer=setTimeout(async()=>{if(state.profile_id&&await stopPlayerForNavigation()){if(shellSection!=="catalog")$("category").value="";shellShow("catalog");scrollTo({top:0,behavior:"instant"});loadPage();}},250);};
$("category").onchange=()=>{shellShow("catalog");loadPage();};
for(const id of ["genre","year","watched","available","sort"])$(id).addEventListener("change",()=>{if(state.profile_id)loadPage();});
document.addEventListener("keydown",event=>{if((event.metaKey||event.ctrlKey)&&event.key.toLowerCase()==="f"&&state.profile_id){event.preventDefault();$("search").focus();}});
window.chillReady?.then(()=>window.renderShell());
