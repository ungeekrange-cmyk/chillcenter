"use strict";
let shellSection="catalog",shellSearchTimer=null;
const categoryNames={"":"Toute la collection",film:"Films",serie:"Séries",spectacle:"Spectacles",documentaire:"Documentaires"};
window.renderShell=()=>{
 const connected=state.stage==="connected",active=connected&&Boolean(state.profile_id);
 $("advice-toggle").hidden=!active;document.querySelector(".global-search").hidden=!active;
 document.querySelectorAll("[data-category],#show-sagas,#show-playlists").forEach(button=>button.disabled=!active);
 $("connected-address").textContent=state.address||"";
 if(!connected)shellSection="catalog";
 $("advice-panel").hidden=!active||shellSection!=="advice";
 $("catalog-panel").hidden=!active||shellSection!=="catalog";
 if(shellSection!=="admin")$("admin-panel").hidden=true;
 document.querySelectorAll("[data-category]").forEach(button=>button.classList.toggle("active",shellSection==="catalog"&&button.dataset.category===$("category").value));
 $("admin-toggle").classList.toggle("active",shellSection==="admin");$("advice-toggle").classList.toggle("active",shellSection==="advice");
 $("catalog-title").textContent=categoryNames[$("category").value]||"Toute la collection";
};
function shellShow(section){shellSection=section;window.renderShell();}
document.querySelectorAll("[data-category]").forEach(button=>button.onclick=async()=>{
 if(!state.profile_id)return;
 if(typeof playerState!=="undefined"&&playerState.stage==="player")await playerCall({action:"stop"});
 $("category").value=button.dataset.category;shellShow("catalog");scrollTo({top:0,behavior:"instant"});await loadPage();
});
$("advice-toggle").onclick=async()=>{if(typeof playerState!=="undefined"&&playerState.stage==="player")await playerCall({action:"stop"});shellShow("advice");};
$("admin-toggle").addEventListener("click",()=>{shellSection="admin";window.renderShell();});
$("admin-close").addEventListener("click",()=>shellShow("catalog"));
$("search").oninput=()=>{clearTimeout(shellSearchTimer);shellSearchTimer=setTimeout(()=>{if(state.profile_id){shellShow("catalog");scrollTo({top:0,behavior:"instant"});loadPage();}},250);};
$("category").onchange=()=>{shellShow("catalog");loadPage();};
for(const id of ["genre","year","watched","available","sort"])$(id).addEventListener("change",()=>{if(state.profile_id)loadPage();});
document.addEventListener("keydown",event=>{if((event.metaKey||event.ctrlKey)&&event.key.toLowerCase()==="f"&&state.profile_id){event.preventDefault();$("search").focus();}});
window.chillReady?.then(()=>window.renderShell());
