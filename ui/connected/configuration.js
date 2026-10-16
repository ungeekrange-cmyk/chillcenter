"use strict";
const configurationCategories=[
 ["all","Tout"],["general","Général"],["server","Serveur"],["library","Bibliothèque"],
 ["profiles","Profils"],["playback","Lecture"],["appearance","Apparence"],
 ["metadata","Fiches et matching"],["ai","Intelligence artificielle"],["archives","Archives"]
];
function configurationText(value){return String(value||"").normalize("NFD").replace(/[\u0300-\u036f]/g,"").toLocaleLowerCase().replace(/[^\p{L}\p{N}]+/gu," ").trim();}
function configurationMatches(category,query,cardCategory,text){return (category==="all"||category===cardCategory)&&configurationText(query).split(" ").every(term=>configurationText(text).includes(term));}
function buildLibraryConfigurationCommand(primary,additional,previous){
 const roots=additional.map(path=>path.trim()).filter(Boolean),first=primary.trim()||null;
 if(roots.length+(first?1:0)>16)throw Error("La bibliothèque est limitée à 16 emplacements.");
 if(new Set([first,...roots].filter(Boolean)).size!==roots.length+(first?1:0))throw Error("Chaque emplacement doit être unique.");
 const command={action:"save_library",video_root:first,expected_revision:previous.revision};
 // An old server still accepts the historical one-root request. Never silently
 // drop extra locations when the user is explicitly adding/removing them.
 if(roots.length||Array.isArray(previous.additional_video_roots))command.additional_video_roots=roots;
 return command;
}
if(typeof module!=="undefined")module.exports={configurationCategories,configurationText,configurationMatches,configurationLibraryCommand:buildLibraryConfigurationCommand};
if(typeof window!=="undefined"&&typeof document!=="undefined")(()=>{
 const panel=$("admin-panel"),grid=panel.querySelector(".admin-grid"),navigation=$("configuration-categories"),search=$("configuration-search");
 const groups={"tmdb-admin":"metadata","matching-admin":"metadata","archive-admin":"archives","ai-settings":"ai","playback-cache-settings":"playback","player-presentation-settings":"playback","unavailable-admin":"library","identity-settings":"library"};
 let selected="all",locationsSignature=null;
 for(const [key,label]of configurationCategories){const option=node("button",label,"configuration-category");option.type="button";option.dataset.category=key;option.onclick=()=>{selected=key;refresh();};navigation.append(option);}
 function refresh(){
  let count=0;
  for(const card of grid.children){
   if(card.tagName!=="SECTION")continue;
   const category=card.dataset.settingsCategory||groups[card.id]||"general";card.dataset.settingsCategory=category;
   const label=configurationCategories.find(([key])=>key===category)?.[1]||"Général";
   // Values of inputs (notably provider secrets) are never search material.
   const text=[label,...[...card.querySelectorAll("h2,h3,label,button,summary")].map(element=>element.textContent)].join(" ");
   card.hidden=!configurationMatches(selected,search.value,category,text)||(card.id==="personal-settings"&&!state.profile_id);
   if(!card.hidden)count++;
  }
  for(const option of navigation.children){option.setAttribute("aria-current",option.dataset.category===selected?"page":"false");option.classList.toggle("active",option.dataset.category===selected);}
  $("configuration-empty").hidden=count!==0;$("configuration-count").textContent=count+" rubrique"+(count===1?"":"s");
 }
 search.oninput=()=>{selected="all";refresh();};
 // Modules append their settings cards lazily. Observe only the grid's direct
 // children: status text and filtering must not trigger a mutation/render loop.
 new MutationObserver(refresh).observe(grid,{childList:true});
 function addLocation(path=""){
  const row=node("div",undefined,"library-location-row"),label=node("label","Localisation supplémentaire"),input=node("input"),remove=node("button","Retirer","secondary");
  input.value=path;input.autocomplete="off";input.maxLength=4096;input.placeholder="/autre/bibliotheque";label.append(input);remove.type="button";
  remove.onclick=()=>{row.remove();updateAddButton();};row.append(label,remove);$("additional-library-locations").append(row);updateAddButton();return input;
 }
 function updateAddButton(){$("library-add-location").disabled=$("additional-library-locations").children.length>=15;}
 $("library-add-location").onclick=()=>{if($("additional-library-locations").children.length<15)addLocation().focus();};
 window.refreshLibraryLocations=settings=>{
  const signature=JSON.stringify([settings.video_root,settings.additional_video_roots||[]]);if(signature===locationsSignature)return;locationsSignature=signature;
  $("video-root").value=settings.video_root||"";$("additional-library-locations").replaceChildren();for(const path of settings.additional_video_roots||[])addLocation(path);updateAddButton();
 };
 window.configurationLibraryCommand=()=>buildLibraryConfigurationCommand($("video-root").value,[...$("additional-library-locations").querySelectorAll("input")].map(input=>input.value),state.library);
 window.refreshConfiguration=refresh;
 window.configurationShow=category=>{selected=configurationCategories.some(([key])=>key===category)?category:"all";search.value="";refresh();};
 if(state.library)window.refreshLibraryLocations(state.library);refresh();
})();
