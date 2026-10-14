"use strict";
// Catalogue and personal collections share the allowlisted Rust bridge. No URLs or
// filesystem paths from media metadata are evaluated by this interface.
let catalogProfile=null,collectionView=null,collectionKind="playlist",detailView=null,seriesView=null;
let collectionFilter="",collectionDetails=new Map(),collectionLoaded=0;
// Reordering a filtered playlist must still be a permutation of the canonical
// membership. Swap the two visible positions in that complete list so hidden
// members remain present and keep their relative order.
function playlistOrderForVisibleMove(mediaIds,visibleIds,mediaId,delta){
  const fromVisible=visibleIds.indexOf(mediaId),toVisible=fromVisible+delta;
  if(!Array.isArray(mediaIds)||!Array.isArray(visibleIds)||!Number.isInteger(delta)||toVisible<0||toVisible>=visibleIds.length)return null;
  const targetId=visibleIds[toVisible],from=mediaIds.indexOf(mediaId),to=mediaIds.indexOf(targetId);
  if(from<0||to<0||new Set(mediaIds).size!==mediaIds.length||new Set(visibleIds).size!==visibleIds.length||visibleIds.some(id=>!mediaIds.includes(id)))return null;
  const order=[...mediaIds];[order[from],order[to]]=[order[to],order[from]];return order;
}
function seriesEditCommand(series,expectedRevision,patch){return {action:"edit_series",series,expected_revision:expectedRevision,patch};}
function relatedMediaCommand(mediaId){return {action:"related_media",media_id:mediaId};}
function tmdbCommand(action,mediaId,revision,scope="media"){return action==="refresh"?{action:"tmdb_refresh",media_id:mediaId,expected_revision:revision,scope}:action==="restore"?{action:"tmdb_restore_poster",media_id:mediaId,expected_revision:revision}:{action:"tmdb_candidates",media_id:mediaId,query:""};}
if(typeof module!=="undefined")module.exports={playlistOrderForVisibleMove,seriesEditCommand,relatedMediaCommand,tmdbCommand};
function closeCatalogDialogs(){for(const id of ["media-dialog","collections-dialog","collection-dialog","picker-dialog"])if($(id).open)$(id).close();collectionView=null;detailView=null;seriesView=null;}
window.catalogProfileChanged=()=>{if(catalogProfile!==state.profile_id){closeCatalogDialogs();if(typeof hidePlayer==="function")hidePlayer();catalogProfile=state.profile_id;}};
function button(text,fn,cls="secondary"){const b=node("button",text,cls);b.type="button";b.onclick=fn;return b;}
function options(select,values){select.replaceChildren();for(const [value,label] of values){const o=node("option",label);o.value=value;select.append(o);}}
window.catalogEnhanceCard=(card,m)=>{
  const actions=card.querySelector(".actions");
  if(m.poster)observeArtwork(card.querySelector(".poster"),m.id);
  if(m.category==="serie"){actions?.append(button("Voir la série",()=>openSeries(m.series||m.title)));return;}
  actions.append(button("Fiche",()=>openMedia(m.id)),button("Ajouter à une playlist",()=>pickPlaylist(m.id)));
};
window.catalogOpenMedia=openMedia;window.catalogOpenSeries=openSeries;
async function importArtwork(target,reload){try{const result=await window.__TAURI__.core.invoke("artwork_command",{target});if(!result?.cancelled)await reload();}catch(error){message(error?.message||"Import de l'image non effectué.");}}
async function openMedia(id){
  const result=await run({action:"media_detail",media_id:id},false);if(!result)return;detailView=result;
  const m=result.media;$("detail-title").textContent=m.title;$("detail-overview").textContent=m.overview||"Informations à compléter";
  $("detail-metadata").textContent=[m.year,m.runtime_minutes?m.runtime_minutes+" min":null,...m.genres,...m.directors.map(n=>"Réalisation : "+n),...m.cast].filter(Boolean).join(" · ");
  $("detail-availability").textContent=m.available?"Disponible":"Vidéo indisponible";
  $("detail-play").disabled=!m.available;$("detail-play").onclick=()=>{$("media-dialog").close();window.chillPlay(m.id);};let artwork=$("detail-artwork");if(!artwork){artwork=button("Importer une image",()=>{});artwork.id="detail-artwork";$("detail-play").parentElement.append(artwork);}artwork.onclick=()=>importArtwork({kind:"media",media_id:m.id,expected_revision:result.revision},async()=>{await openMedia(m.id);await loadPage();});
  let tmdb=$("detail-tmdb");if(!tmdb){tmdb=document.createElement("section");tmdb.id="detail-tmdb";$("media-dialog").append(tmdb);}tmdb.replaceChildren(node("h3","TMDB"));const scope=document.createElement("select");scope.append(new Option("Cette vidéo","media"));if(m.category==="serie")scope.append(new Option("Informations communes de la série","series"));const candidates=document.createElement("div");tmdb.append(scope,button("Rechercher TMDB",async()=>{const r=await run(tmdbCommand("search",m.id,result.revision),false);if(!r)return;candidates.replaceChildren();for(const c of r.candidates||[]){candidates.append(button([c.title,c.year,c.kind].filter(Boolean).join(" · "),async()=>{if(await run({action:"tmdb_select",media_id:m.id,expected_revision:result.revision,tmdb_id:c.id,kind:c.kind,scope:scope.value},false)){await openMedia(m.id);await loadPage();}}));}}),button("Actualiser le fournisseur",async()=>{if(await run(tmdbCommand("refresh",m.id,result.revision,scope.value),false)){await openMedia(m.id);await loadPage();}}),button("Restaurer l’affiche fournisseur",async()=>{if(await run(tmdbCommand("restore",m.id,result.revision),false)){await openMedia(m.id);await loadPage();}}),candidates);
  $("detail-edit").hidden=false;$("metadata-form").hidden=true;
  options($("detail-rating-scope"),m.category==="serie"?[["video","Cet épisode"],["season","Cette saison"],["series","Toute la série"]]:[["video","Cette vidéo"]]);
  $("detail-rating").value=m.rating?String(m.rating):"";$("detail-rating-scope").value="video";$("media-dialog").showModal();
  loadRelatedMedia(m.id);
}
async function loadRelatedMedia(mediaId){
  let section=$("related-media");if(!section){section=document.createElement("section");section.id="related-media";section.append(node("h3","Vidéos en relation"),node("div",undefined,"actions"));$("media-dialog").append(section);}section.hidden=true;section.lastChild.replaceChildren();
  const result=await run(relatedMediaCommand(mediaId),false);if(!result||!detailView||detailView.media.id!==mediaId)return;
  for(const item of (result.items||[]).slice(0,12))section.lastChild.append(button(item.title,()=>openMedia(item.id)));section.hidden=!section.lastChild.childElementCount;
}
$("detail-rate").onclick=async()=>{if(!detailView)return;const id=detailView.media.id;
  if(await run({action:"group_rating",media_id:id,rating:$("detail-rating").value?Number($("detail-rating").value):null,scope:$("detail-rating-scope").value},false)){await openMedia(id);await loadPage();}};
$("detail-edit").onclick=()=>{
  if(!detailView)return;const m=detailView.media;
  for(const key of ["title","category","year","series","season","episode","overview","runtime_minutes","series_overview"])$("edit-"+key).value=m[key]??"";
  for(const key of ["genres","cast","directors"])$("edit-"+key).value=m[key].join(", ");
  $("metadata-form").hidden=false;
};
$("metadata-form").onsubmit=async e=>{
  e.preventDefault();if(!detailView)return;const patch={};
  for(const key of ["title","category","series","overview","series_overview"])patch[key]=$("edit-"+key).value;
  for(const key of ["year","runtime_minutes"])if($("edit-"+key).value!=="")patch[key]=Number($("edit-"+key).value);
  for(const key of ["season","episode"])patch[key]=$("edit-"+key).value===""?null:Number($("edit-"+key).value);
  for(const key of ["genres","cast","directors"])patch[key]=$("edit-"+key).value.split(",").map(s=>s.trim()).filter(Boolean);
  const result=await run({action:"edit_media",media_id:detailView.media.id,expected_revision:detailView.revision,patch},false);
  if(result){await openMedia(result.media.id);await loadPage();}else{$("detail-availability").textContent="Enregistrement non effectué. Rouvrez la fiche pour relire sa dernière révision ; vos champs restent visibles.";}
};
async function openSeries(name){
  const result=await run({action:"series",name},false);if(!result)return;
  seriesView=result;collectionKind="series";collectionView={id:result.name||name,name:result.name||name,media_ids:result.episodes.map(m=>m.id),episodes:result.episodes};
  collectionFilter="";collectionDetails=new Map();collectionLoaded=0;ensureCollectionFilter("series");
  $("collection-title").textContent=collectionView.name;$("collection-counts").textContent=result.counters.watched+" vus · "+result.counters.unwatched+" non vus";
  $("collection-admin").hidden=true;$("collection-add").hidden=true;$("collection-members").replaceChildren();
  renderSeriesCommon(result);
  const ordered=[...result.episodes].sort((a,b)=>(a.season===null?Infinity:a.season)-(b.season===null?Infinity:b.season)||(a.episode===null?Infinity:a.episode)-(b.episode===null?Infinity:b.episode)||a.title.localeCompare(b.title,"fr"));
  let season=undefined,section=null;for(const m of ordered){if(m.season!==season){season=m.season;section=document.createElement("details");section.className="series-season";const episodes=ordered.filter(e=>e.season===season),watched=episodes.filter(e=>e.watched).length;section.append(node("summary",(season===null?"Saison inconnue":"Saison "+season)+" · "+watched+" vus · "+(episodes.length-watched)+" non vus"));appendGroupRating(section,episodes,"season");$("collection-members").append(section);}appendCollectionMember(m,undefined,section);}
  $("collection-dialog").showModal();
}
function ratingSummary(items){const ratings=[...new Set(items.map(item=>item.rating??null))];return ratings.length===1?(ratings[0]===null?"Sans note":"Ma note : "+"★".repeat(ratings[0])):"Notes différentes";}
function appendGroupRating(target,items,scope){
  if(!items.length)return;const controls=node("div",undefined,"actions"),select=document.createElement("select");select.setAttribute("aria-label",scope==="series"?"Note de la série":"Note de la saison");
  for(const [value,label] of [["","Sans note"],["1","★"],["2","★★"],["3","★★★"],["4","★★★★"],["5","★★★★★"]]){const option=node("option",label);option.value=value;select.append(option);}const uniform=new Set(items.map(item=>item.rating??null));if(uniform.size===1)select.value=[...uniform][0]??"";
  controls.append(node("span",ratingSummary(items),"muted"),select,button("Appliquer",async()=>{if(await run({action:"group_rating",media_id:items[0].id,rating:select.value?Number(select.value):null,scope},false)){await openSeries(collectionView.name);await loadPage();}}));target.append(controls);
}
function renderSeriesCommon(result){
  let section=$("series-common");if(!section){section=document.createElement("section");section.id="series-common";$("collection-members").before(section);}section.replaceChildren();const m=result.metadata||{};
  section.append(node("p",[m.year===0?"Année inconnue":m.year,m.genres?.join(", "),m.overview||"Informations communes à compléter"].filter(Boolean).join(" · ")||"Informations communes à compléter"));appendGroupRating(section,result.episodes,"series");
  const form=document.createElement("form"),edit=button("Modifier les informations communes",()=>form.hidden=!form.hidden);form.className="series-form";form.hidden=true;
  const fields=[["name","Nom de la série",m.name??result.name],["year","Année",m.year??""],["genres","Genres séparés par des virgules",(m.genres||[]).join(", ")],["overview","Résumé commun",m.overview??""],["cast","Distribution",(m.cast||[]).join(", ")],["directors","Réalisation",(m.directors||[]).join(", ")]];
  for(const [key,label,value] of fields){const input=key==="overview"?document.createElement("textarea"):document.createElement("input");input.name=key;input.value=value;input.maxLength=key==="name"?500:20000;if(key==="year"){input.type="number";input.min="0";input.max="2200";}const wrapper=document.createElement("label");wrapper.textContent=label;wrapper.append(input);form.append(wrapper);}form.append(button("Enregistrer"));form.onsubmit=async event=>{event.preventDefault();const values=form.elements,split=key=>values[key].value.split(",").map(value=>value.trim()).filter(Boolean),patch={name:values.name.value,year:values.year.value===""?0:Number(values.year.value),genres:split("genres"),overview:values.overview.value,cast:split("cast"),directors:split("directors")};const updated=await run(seriesEditCommand(result.name,result.revision,patch),false);if(updated){await openSeries(updated.name||patch.name||result.name);await loadPage();}};section.append(edit,form);
}
async function openCollections(kind="playlist",addMedia=null){
  const result=await run({action:kind==="playlist"?"playlists":"sagas"},false);if(!result)return;
  collectionKind=kind;$("collections-title").textContent=kind==="playlist"?"Mes playlists":"Sagas communes";$("collections-list").replaceChildren();
  for(const c of result.items){const row=node("div",undefined,"collection-row");row.append(button(c.name+" · "+c.media_ids.length+" vidéos",async()=>{
    if(addMedia){const r=await run({action:"add_playlist",id:c.id,expected_revision:c.revision,media_ids:[addMedia]},false);if(r)$("collections-dialog").close();}
    else{$("collections-dialog").close();await openCollection(c.id,kind);}
  }));$("collections-list").append(row);}
  $("collection-create").hidden=false;$("collection-create").elements.name.placeholder=kind==="playlist"?"Nouvelle playlist":"Nouvelle saga commune";$("collection-create").onsubmit=async e=>{e.preventDefault();const name=e.currentTarget.elements.name.value;
    const r=await run(kind==="playlist"?{action:"create_playlist",name,media_id:addMedia}:{action:"create_saga",name},false);if(r){$("collection-create").reset();$("collections-dialog").close();await openCollection((r.playlist||r.saga).id,kind);}};
  $("collections-dialog").showModal();
}
async function pickPlaylist(media){await openCollections("playlist",media);}
async function openCollection(id,kind="playlist"){
  const r=await run({action:kind==="playlist"?"playlist":"saga",id},false);if(!r)return;collectionKind=kind;collectionView=r.playlist||r.saga;
  $("series-common")?.remove();seriesView=null;
  $("collection-title").textContent=collectionView.name;$("collection-counts").textContent=collectionView.media_ids.length+" vidéos";
  $("collection-name").value=collectionView.name;$("collection-admin").hidden=false;$("collection-add").hidden=false;$("collection-members").replaceChildren();
  let artwork=$("collection-artwork");if(kind==="saga"){if(!artwork){artwork=button("Importer une image",()=>{});artwork.id="collection-artwork";$("collection-admin").append(artwork);}artwork.hidden=false;artwork.onclick=()=>importArtwork({kind:"saga",id:collectionView.id,expected_revision:collectionView.revision},async()=>await openCollection(collectionView.id,"saga"));}else if(artwork)artwork.hidden=true;
  collectionFilter="";collectionDetails=new Map();collectionLoaded=0;ensureCollectionFilter(kind);
  // Fetch fixed pages of one item at a time, bounded by the collection limit; never
  // load media files or make the webview a direct server HTTP client.
  await collectionPage(id,0);
  $("collection-dialog").showModal();
}
function ensureCollectionFilter(kind){
  let form=$("collection-filter");
  if(kind!=="playlist"){if(form)form.remove();return;}
  if(!form){form=document.createElement("form");form.id="collection-filter";form.className="inline";
    const input=document.createElement("input");input.type="search";input.name="q";input.placeholder="Filtrer les titres";input.setAttribute("aria-label","Filtrer les titres de la playlist");input.maxLength=500;
    form.append(input);$("collection-members").before(form);
    form.onsubmit=e=>e.preventDefault();input.oninput=()=>{collectionFilter=input.value.trim().toLocaleLowerCase();renderCollectionMembers();};
  }else form.elements.q.value="";
}
async function collectionPage(id,start){
  const view=collectionView,kind=collectionKind;
  if(!view||view.id!==id)return;
  const ids=view.media_ids.slice(start,start+50);
  for(const media_id of ids){const d=await run({action:"media_detail",media_id},false);if(collectionView!==view||kind!==collectionKind)return;if(d)collectionDetails.set(media_id,d.media);}
  collectionLoaded=Math.max(collectionLoaded,start+ids.length);renderCollectionMembers();
}
function visibleCollectionIds(){return collectionView.media_ids.filter(id=>{const m=collectionDetails.get(id);return m&&(!collectionFilter||m.title.toLocaleLowerCase().includes(collectionFilter));});}
function renderCollectionMembers(){
  if(!collectionView)return;$("collection-members").replaceChildren();const visibleIds=visibleCollectionIds();
  for(const id of visibleIds)appendCollectionMember(collectionDetails.get(id),visibleIds);
  if(collectionLoaded<collectionView.media_ids.length){const more=button("Afficher les 50 vidéos suivantes",async()=>await collectionPage(collectionView.id,collectionLoaded));$("collection-members").append(more);}
}
function appendCollectionMember(m,visibleIds,target=$("collection-members")){
  const row=node("article",undefined,"collection-row");row.append(node("span",[m.season!==null?"S"+m.season:null,m.episode!==null?"E"+m.episode:null,m.title,m.watched?"Vu ✓":null,!m.available?"Indisponible":null].filter(Boolean).join(" · ")));
  const context={kind:collectionKind,id:collectionView.id};const play=button("Lecture",()=>{$("collection-dialog").close();window.chillPlay(m.id,context);});play.disabled=!m.available;row.append(play);
  if(collectionKind!=="series"){
    const ids=collectionView.media_ids,index=ids.indexOf(m.id);
    for(const [text,delta] of [["↑",-1],["↓",1]]){const order=collectionKind==="playlist"?playlistOrderForVisibleMove(ids,visibleIds,m.id,delta):index+delta<0||index+delta>=ids.length?null:(()=>{const next=[...ids];[next[index],next[index+delta]]=[next[index+delta],next[index]];return next;})();const b=button(text,async()=>{
      if(order&&await run({action:collectionKind==="playlist"?"reorder_playlist":"reorder_saga",id:collectionView.id,expected_revision:collectionView.revision,expected_order:ids,media_ids:order},false))await openCollection(collectionView.id,collectionKind);});b.disabled=!order;row.append(b);}
    row.append(button("Retirer",async()=>{if(await run({action:collectionKind==="playlist"?"remove_playlist_member":"remove_saga_member",id:collectionView.id,media_id:m.id,expected_revision:collectionView.revision},false))await openCollection(collectionView.id,collectionKind);}));
  }
  target.append(row);
}
$("collection-rename").onsubmit=async e=>{e.preventDefault();if(!collectionView)return;if(await run({action:collectionKind==="playlist"?"rename_playlist":"rename_saga",id:collectionView.id,expected_revision:collectionView.revision,name:$("collection-name").value},false))await openCollection(collectionView.id,collectionKind);};
$("collection-delete").onclick=()=>{$("collection-delete-confirm").hidden=false;};
$("collection-delete-yes").onclick=async()=>{const cmd=collectionView&&{action:collectionKind==="playlist"?"delete_playlist":"delete_saga",id:collectionView.id};if(cmd&&collectionKind==="saga")cmd.expected_revision=collectionView.revision;if(cmd&&await run(cmd,false)){$("collection-dialog").close();collectionView=null;$("collection-delete-confirm").hidden=true;await openCollections(collectionKind);}};
$("collection-delete-no").onclick=()=>{$("collection-delete-confirm").hidden=true;};
let pickerOffset=0,pickerSelection=new Set();
async function pickerPage(){const page=await run(catalogCommand({category:"",q:$("picker-search").value,genre:"",year:"",watched:"",sort:"",available:""},pickerOffset,false),false);if(!page)return;$("picker-items").replaceChildren();
  for(const m of page.items){if(collectionView.media_ids.includes(m.id))continue;const b=button((pickerSelection.has(m.id)?"✓ ":"")+m.title,()=>{if(pickerSelection.has(m.id))pickerSelection.delete(m.id);else if(pickerSelection.size<1000)pickerSelection.add(m.id);pickerPage();});$("picker-items").append(b);}
  $("picker-selected").textContent=pickerSelection.size+" sélectionnée(s)"+(pickerSelection.size>=1000?" — maximum atteint":"");
  $("picker-prev").disabled=!pickerOffset;$("picker-next").disabled=!page.has_more;
}
$("collection-add").onclick=async()=>{pickerOffset=0;pickerSelection=new Set();$("picker-search").value="";await pickerPage();$("picker-dialog").showModal();};
$("picker-confirm").onclick=async()=>{if(!collectionView||!pickerSelection.size)return;const ids=[...pickerSelection];const r=await run({action:collectionKind==="playlist"?"add_playlist":"add_saga",id:collectionView.id,expected_revision:collectionView.revision,media_ids:ids},false);if(r){pickerSelection.clear();$("picker-dialog").close();await openCollection(collectionView.id,collectionKind);}};
$("picker-cancel").onclick=()=>{pickerSelection.clear();$("picker-dialog").close();};
$("picker-form").onsubmit=e=>{e.preventDefault();pickerOffset=0;pickerPage();};$("picker-prev").onclick=()=>{pickerOffset=Math.max(0,pickerOffset-50);pickerPage();};$("picker-next").onclick=()=>{pickerOffset+=50;pickerPage();};
$("show-playlists").onclick=()=>openCollections();$("show-sagas").onclick=()=>openCollections("saga");
for(const id of ["media-dialog","collections-dialog","collection-dialog","picker-dialog"]){const m=node("p",undefined,"dialog-message");m.setAttribute("role","status");m.setAttribute("aria-live","polite");$(id).prepend(m);$(id).querySelector(".dialog-close").onclick=()=>$(id).close();}
let artworkQueue=[],artworkLoading=false;
window.resetArtwork=()=>{artworkObserver.disconnect();artworkQueue=[];};
const artworkObserver=new IntersectionObserver(entries=>{
  for(const e of entries){if(e.isIntersecting){if(!e.target.dataset.loaded){artworkQueue.push({target:e.target,id:e.target.dataset.mediaId,profile:state.profile_id});}}else{e.target.replaceChildren(node("span",e.target.dataset.initial||""));delete e.target.dataset.loaded;}}
  drainArtwork();
},{rootMargin:"100px"});
function observeArtwork(target,id){target.dataset.mediaId=id;target.dataset.initial=target.textContent;artworkObserver.observe(target);}
async function drainArtwork(){
  if(artworkLoading)return;artworkLoading=true;
  while(artworkQueue.length){const item=artworkQueue.shift();if(!item.target.isConnected||item.profile!==state.profile_id||item.target.dataset.loaded)continue;
    if(busy||playerBusy){artworkQueue.unshift(item);await new Promise(r=>setTimeout(r,300));continue;}
    try{const r=await invoke({action:"artwork",media_id:item.id});if(!item.target.isConnected||item.profile!==state.profile_id)continue;
      const img=node("img");img.src=r.src;img.alt="";img.decoding="async";item.target.replaceChildren(img);item.target.dataset.loaded="true";
    }catch(e){if(e.code==="CLIENT_BUSY"){artworkQueue.unshift(item);await new Promise(r=>setTimeout(r,300));}}
  }
  artworkLoading=false;
}
window.chillArtwork=observeArtwork;
