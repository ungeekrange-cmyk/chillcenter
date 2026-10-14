"use strict";
// Catalogue and personal collections share the allowlisted Rust bridge. No URLs or
// filesystem paths from media metadata are evaluated by this interface.
let catalogProfile=typeof state==="undefined"?null:state.server_id+":"+state.profile_id,collectionView=null,collectionKind="playlist",detailView=null,seriesView=null;
let collectionFilter="",collectionDetails=new Map(),collectionLoaded=0;
let collectionEpoch=0,collectionContext="";
function collectionScope(){return state.server_id+":"+state.profile_id;}
function collectionCurrent(epoch,context){return epoch===collectionEpoch&&context===collectionScope();}
async function libraryRead(command,current){
  for(let attempt=0;attempt<50&&current();attempt++){
    try{const result=await invoke(command);return current()?result:null;}
    catch(error){if(error.code!=="CLIENT_BUSY"){if(current())message(error.message||"Lecture indisponible.");return null;}await new Promise(resolve=>setTimeout(resolve,100));}
  }
  if(current())message("Le client est occupé. Réessayez dans un instant.");return null;
}
function showCollectionPage(section){shellShow(section);scrollTo({top:0,behavior:"instant"});}
window.leaveCollectionPages=()=>{collectionEpoch++;pickerGeneration++;collectionView=null;seriesView=null;collectionDetails=new Map();};
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
function tmdbCommand(action,mediaId,revision,scope="media",query=""){return action==="refresh"?{action:"tmdb_refresh",media_id:mediaId,expected_revision:revision,scope}:action==="restore"?{action:"tmdb_restore_poster",media_id:mediaId,expected_revision:revision}:{action:"tmdb_candidates",media_id:mediaId,query};}
function sagaArtworkCommand(id){return {action:"saga_artwork",id};}
function playlistArtworkCommand(id){return {action:"playlist_artwork",id};}
function sagaMemberArtworkCommand(id,expectedRevision,mediaId){return {action:"saga_member_artwork",id,expected_revision:expectedRevision,media_id:mediaId};}
if(typeof module!=="undefined")module.exports={playlistOrderForVisibleMove,seriesEditCommand,relatedMediaCommand,tmdbCommand,sagaArtworkCommand,playlistArtworkCommand,sagaMemberArtworkCommand,collectionMosaic};
function closeCatalogDialogs(){for(const id of ["media-dialog","playlist-target-dialog"])if($(id).open)$(id).close();window.leaveCollectionPages();detailView=null;}
window.catalogProfileChanged=()=>{if(catalogProfile!==collectionScope()){closeCatalogDialogs();if(typeof hidePlayer==="function")hidePlayer();catalogProfile=collectionScope();if(typeof shellSection!=="undefined")shellSection="catalog";}};
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
  $("detail-availability").textContent=m.available?"":"Indisponible";$("detail-availability").hidden=m.available;
  $("detail-poster").replaceChildren(node("span",m.title.slice(0,1).toUpperCase()));if(m.poster)observeArtwork($("detail-poster"),m.id);
  $("detail-addlist").onclick=()=>pickPlaylist(m.id);
  $("detail-play").disabled=!m.available;$("detail-play").onclick=()=>{$("media-dialog").close();window.chillPlay(m.id);};let artwork=$("detail-artwork");if(!artwork){artwork=button("Importer une image",()=>{});artwork.id="detail-artwork";$("detail-play").parentElement.append(artwork);}artwork.onclick=()=>importArtwork({kind:"media",media_id:m.id,expected_revision:result.revision},async()=>{await openMedia(m.id);await loadPage();});
  let tmdb=$("detail-tmdb");if(!tmdb){tmdb=document.createElement("section");tmdb.id="detail-tmdb";$("media-dialog").querySelector(".detail-copy").append(tmdb);}tmdb.replaceChildren(node("h3","TMDB"));const scope=document.createElement("select"),query=document.createElement("input");scope.append(new Option("Cette vidéo","media"));if(m.category==="serie")scope.append(new Option("Informations communes de la série","series"));query.type="search";query.maxLength=500;query.value=m.title||"";query.placeholder="Titre à rechercher";query.setAttribute("aria-label","Recherche TMDB");const candidates=document.createElement("div");tmdb.append(scope,query,button("Rechercher TMDB",async()=>{const r=await run(tmdbCommand("search",m.id,result.revision,scope.value,query.value),false);if(!r)return;candidates.replaceChildren();for(const c of r.candidates||[]){candidates.append(button([c.title,c.year,c.kind].filter(Boolean).join(" · "),async()=>{if(await run({action:"tmdb_select",media_id:m.id,expected_revision:result.revision,tmdb_id:c.id,kind:c.kind,scope:scope.value},false)){await openMedia(m.id);await loadPage();}}));}}),button("Actualiser le fournisseur",async()=>{if(await run(tmdbCommand("refresh",m.id,result.revision,scope.value),false)){await openMedia(m.id);await loadPage();}}),button("Restaurer l’affiche fournisseur",async()=>{if(await run(tmdbCommand("restore",m.id,result.revision),false)){await openMedia(m.id);await loadPage();}}),candidates);
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
  if(!await stopPlayerForNavigation())return;
  const epoch=++collectionEpoch,context=collectionScope();
  const result=await libraryRead({action:"series",name},()=>collectionCurrent(epoch,context));if(!result)return;
  seriesView=result;collectionKind="series";collectionView={id:result.name||name,name:result.name||name,media_ids:result.episodes.map(m=>m.id),episodes:result.episodes};
  collectionFilter="";collectionDetails=new Map();collectionLoaded=0;ensureCollectionFilter("series");
  $("collection-title").textContent=collectionView.name;$("collection-counts").textContent=new Set(result.episodes.map(m=>m.season)).size+" saisons · "+result.episodes.length+" épisodes · "+result.counters.unwatched+" épisodes non vus";
  $("collection-admin").hidden=true;$("collection-add").hidden=true;$("collection-members").replaceChildren();
  renderSeriesCommon(result);
  const ordered=[...result.episodes].sort((a,b)=>(a.season===null?Infinity:a.season)-(b.season===null?Infinity:b.season)||(a.episode===null?Infinity:a.episode)-(b.episode===null?Infinity:b.episode)||a.title.localeCompare(b.title,"fr"));
  let season=undefined,section=null;for(const m of ordered){if(m.season!==season){season=m.season;section=document.createElement("details");section.className="series-season";const episodes=ordered.filter(e=>e.season===season),watched=episodes.filter(e=>e.watched).length;section.append(node("summary",(season===null?"Saison inconnue":"Saison "+season)+" · "+episodes.length+" épisodes · "+(episodes.length-watched)+" non vus"));appendGroupRating(section,episodes,"season");$("collection-members").append(section);}appendCollectionMember(m,undefined,section);}
  $("collection-members").classList.add("series-members");$("collection-members").style.height="";
  showCollectionPage("collection");
}
function ratingSummary(items){const ratings=[...new Set(items.map(item=>item.rating??null))];return ratings.length===1?(ratings[0]===null?"Sans note":"Ma note : "+"★".repeat(ratings[0])):"Notes différentes";}
function appendGroupRating(target,items,scope){
  if(!items.length)return;const controls=node("div",undefined,"actions"),select=document.createElement("select");select.setAttribute("aria-label",scope==="series"?"Note de la série":"Note de la saison");
  for(const [value,label] of [["","Sans note"],["1","★"],["2","★★"],["3","★★★"],["4","★★★★"],["5","★★★★★"]]){const option=node("option",label);option.value=value;select.append(option);}const uniform=new Set(items.map(item=>item.rating??null));if(uniform.size===1)select.value=[...uniform][0]??"";
  controls.append(node("span",ratingSummary(items),"muted"),select,button("Appliquer",async()=>{if(await run({action:"group_rating",media_id:items[0].id,rating:select.value?Number(select.value):null,scope},false)){await openSeries(collectionView.name);await loadPage();}}));target.append(controls);
}
function renderSeriesCommon(result){
  let section=$("series-common");if(!section){section=document.createElement("section");section.id="series-common";$("collection-members").before(section);}section.replaceChildren();const m=result.metadata||{};
  const illustrated=result.episodes.find(e=>e.poster&&!e.poster_custom);if(illustrated){const poster=node("div",undefined,"poster series-poster");poster.setAttribute("aria-label","Affiche fournisseur de la série");section.append(poster);observeArtwork(poster,illustrated.id);}
  section.append(node("p",[m.year===0?"Année inconnue":m.year,m.genres?.join(", "),m.overview||"Informations communes à compléter"].filter(Boolean).join(" · ")||"Informations communes à compléter"));appendGroupRating(section,result.episodes,"series");
  const form=document.createElement("form"),edit=button("Modifier les informations communes",()=>form.hidden=!form.hidden);form.className="series-form";form.hidden=true;
  const fields=[["name","Nom de la série",m.name??result.name],["year","Année",m.year??""],["genres","Genres séparés par des virgules",(m.genres||[]).join(", ")],["overview","Résumé commun",m.overview??""],["cast","Distribution",(m.cast||[]).join(", ")],["directors","Réalisation",(m.directors||[]).join(", ")]];
  for(const [key,label,value] of fields){const input=key==="overview"?document.createElement("textarea"):document.createElement("input");input.name=key;input.value=value;input.maxLength=key==="name"?500:20000;if(key==="year"){input.type="number";input.min="0";input.max="2200";}const wrapper=document.createElement("label");wrapper.textContent=label;wrapper.append(input);form.append(wrapper);}form.append(button("Enregistrer"));form.onsubmit=async event=>{event.preventDefault();const values=form.elements,split=key=>values[key].value.split(",").map(value=>value.trim()).filter(Boolean),patch={name:values.name.value,year:values.year.value===""?0:Number(values.year.value),genres:split("genres"),overview:values.overview.value,cast:split("cast"),directors:split("directors")};const updated=await run(seriesEditCommand(result.name,result.revision,patch),false);if(updated){await openSeries(updated.name||patch.name||result.name);await loadPage();}};section.append(edit,form);
}
function posterTile(title,subtitle,activate){
  const tile=button("",activate,"poster-tile"),poster=node("div",title.slice(0,1).toUpperCase(),"poster"),info=node("div",undefined,"info");
  info.append(node("h2",title),node("p",subtitle,"muted"));tile.append(poster,info);tile.setAttribute("aria-label",title);return {tile,poster};
}
function collectionArtworkCommand(kind,id){return kind==="saga"?sagaArtworkCommand(id):playlistArtworkCommand(id);}
function collectionMosaic(poster,collection,kind,epoch,context){
  if(collection.poster){observeArtwork(poster,collection.id,kind);return;}
  poster.classList.add("collection-mosaic");poster.replaceChildren();
  for(const mediaId of collection.media_ids.slice(0,9)){
    const cell=node("div","·","collection-mosaic-cell");poster.append(cell);
    void libraryRead({action:"media_detail",media_id:mediaId},()=>collectionCurrent(epoch,context)).then(result=>{
      if(!result||!cell.isConnected||!collectionCurrent(epoch,context))return;
      cell.textContent=result.media.title.slice(0,1).toUpperCase();
      if(result.media.poster)observeArtwork(cell,result.media.id);
    });
  }
}
async function openCollections(kind="playlist"){
  if(!await stopPlayerForNavigation())return;
  const epoch=++collectionEpoch,context=collectionScope(),result=await libraryRead({action:kind==="playlist"?"playlists":"sagas"},()=>collectionCurrent(epoch,context));if(!result)return;
  collectionView=null;seriesView=null;collectionKind=kind;collectionContext=context;
  const title=kind==="playlist"?"Listes de lecture":"Sagas";
  $("collections-title").textContent=title;$("collections-list").replaceChildren();$("collection-create").hidden=true;
  $("collection-create-toggle").setAttribute("aria-label",kind==="saga"?"Créer une saga":"Créer une liste de lecture");
  $("collection-create").elements.name.placeholder=kind==="saga"?"Nom de la saga":"Nom de la liste de lecture";
  for(const c of result.items){
    const {tile,poster}=posterTile(c.name,c.media_ids.length+" vidéos",()=>openCollection(c.id,kind));tile.dataset.collectionId=c.id;
    $("collections-list").append(tile);
    if(c.media_ids.length||c.poster)collectionMosaic(poster,c,kind,epoch,context);
  }
  if(!result.items.length)$("collections-list").append(node("p","Aucune collection. Utilisez + pour en créer une.","muted"));
  $("collection-create").onsubmit=async e=>{
    e.preventDefault();if(!collectionCurrent(epoch,context))return;
    const name=e.currentTarget.elements.name.value,r=await run(kind==="playlist"?{action:"create_playlist",name}:{action:"create_saga",name},false);
    if(r&&collectionCurrent(epoch,context)){$("collection-create").reset();await openCollection((r.playlist||r.saga).id,kind);}
  };
  showCollectionPage("collections");
}
async function pickPlaylist(media){
  const context=collectionScope(),result=await libraryRead({action:"playlists"},()=>context===collectionScope());if(!result)return;
  $("playlist-targets").replaceChildren();
  for(const c of result.items)$("playlist-targets").append(button(c.name,async()=>{
    if(context!==collectionScope())return;
    if(await run({action:"add_playlist",id:c.id,expected_revision:c.revision,media_ids:[media]},false))$("playlist-target-dialog").close();
  }));
  $("playlist-target-create").onsubmit=async e=>{
    e.preventDefault();if(context!==collectionScope())return;
    if(await run({action:"create_playlist",name:e.currentTarget.elements.name.value,media_id:media},false)){$("playlist-target-create").reset();$("playlist-target-dialog").close();}
  };
  $("playlist-target-dialog").showModal();
}
async function openCollection(id,kind="playlist"){
  if(!await stopPlayerForNavigation())return;
  const epoch=++collectionEpoch,context=collectionScope();
  const r=await libraryRead({action:kind==="playlist"?"playlist":"saga",id},()=>collectionCurrent(epoch,context));if(!r)return;collectionKind=kind;collectionView=r.playlist||r.saga;collectionContext=context;
  $("series-common")?.remove();seriesView=null;
  $("collection-title").textContent=collectionView.name;$("collection-counts").textContent=collectionView.media_ids.length+" vidéos";
  $("collection-name").value=collectionView.name;$("collection-admin").hidden=false;$("collection-admin").open=false;$("collection-add").hidden=false;$("collection-members").replaceChildren();$("collection-members").classList.remove("series-members");
  $("collection-admin").querySelector("summary").textContent=kind==="playlist"?"Gérer la liste":"Gérer la saga";
  $("collection-delete-confirm").hidden=true;
  let artwork=$("collection-artwork"),preview=$("saga-artwork-preview");if(!artwork){artwork=button("Importer une image",()=>{});artwork.id="collection-artwork";$("collection-admin").append(artwork);}if(!preview){preview=document.createElement("div");preview.id="saga-artwork-preview";$("collection-admin").append(preview);}artwork.hidden=false;preview.hidden=false;const view=collectionView;artwork.textContent=kind==="playlist"?"Importer une image de liste":"Importer une image de saga";artwork.onclick=()=>importArtwork({kind,id:view.id,expected_revision:view.revision},async()=>await openCollection(view.id,kind));void loadCollectionArtwork(view,kind);
  if(!collectionCurrent(epoch,context))return;
  collectionFilter="";collectionDetails=new Map();collectionLoaded=0;ensureCollectionFilter(kind);
  showCollectionPage("collection");
  await collectionPage(id,0);
}
$("collection-create-toggle").onclick=()=>{$("collection-create").hidden=!$("collection-create").hidden;if(!$("collection-create").hidden)$("collection-create").elements.name.focus();};
$("collection-create-cancel").onclick=()=>{$("collection-create").hidden=true;$("collection-create").reset();};
$("collection-back").onclick=()=>{if(collectionKind==="series"){shellShow("catalog");}else return openCollections(collectionKind);};
function ensureCollectionFilter(kind){
  let form=$("collection-filter");
  if(kind==="series"){if(form)form.remove();return;}
  if(!form){form=document.createElement("form");form.id="collection-filter";form.className="inline";
    const input=document.createElement("input");input.type="search";input.name="q";input.placeholder="Filtrer les titres";input.setAttribute("aria-label","Filtrer les titres de la collection");input.maxLength=500;
    form.append(input);$("collection-members").before(form);
    form.onsubmit=e=>e.preventDefault();input.oninput=()=>{collectionFilter=input.value.trim().toLocaleLowerCase();scrollTo({top:0,behavior:"instant"});renderCollectionMembers();};
  }else form.elements.q.value="";
}
async function collectionPage(id,start){
  const view=collectionView,kind=collectionKind;
  if(!view||view.id!==id)return;
  const ids=view.media_ids.slice(start,start+50);
  const context=collectionScope();
  for(const media_id of ids){const d=await libraryRead({action:"media_detail",media_id},()=>collectionView===view&&kind===collectionKind&&context===collectionScope());if(collectionView!==view||kind!==collectionKind||context!==collectionScope())return;if(d)collectionDetails.set(media_id,d.media);}
  collectionLoaded=Math.max(collectionLoaded,start+ids.length);renderCollectionMembers();if(collectionView===view&&collectionLoaded<view.media_ids.length)await collectionPage(id,collectionLoaded);
}
function visibleCollectionIds(){return collectionView.media_ids.filter(id=>{const m=collectionDetails.get(id);return m&&(!collectionFilter||m.title.toLocaleLowerCase().includes(collectionFilter));});}
function renderCollectionMembers(){
  if(!collectionView||collectionKind==="series"||$("collection-panel").hidden)return;
  const target=$("collection-members"),visibleIds=visibleCollectionIds(),rect=target.getBoundingClientRect();
  const view=catalogWindow(visibleIds.length,Math.max(1,rect.width),200,-rect.top,innerHeight);
  const focused=document.activeElement?.closest?.(".collection-media-card")?.dataset.mediaId;
  target.style.height=view.height+"px";
  const grid=node("div",undefined,"virtual-window");grid.style.top=view.top+"px";grid.style.gridTemplateColumns="repeat("+view.columns+",minmax(0,1fr))";
  for(const id of visibleIds.slice(view.start,view.end))appendCollectionMember(collectionDetails.get(id),visibleIds,grid);
  target.replaceChildren(grid);
  if(!visibleIds.length)target.append(node("p",collectionLoaded<collectionView.media_ids.length?"Chargement des vidéos…":"Aucune vidéo. Utilisez + pour en ajouter ou modifiez le filtre.","muted"));
  if(focused)[...grid.children].find(card=>card.dataset.mediaId===focused)?.focus({preventScroll:true});
}
function appendCollectionMember(m,visibleIds,target=$("collection-members")){
  const view=collectionView,kind=collectionKind,scope=collectionScope();
  const row=node("article",undefined,"media-card collection-media-card"),poster=node("div",m.title.slice(0,1).toUpperCase(),"poster"),copy=node("div",undefined,"info member-copy"),actions=node("div",undefined,"actions");row.dataset.mediaId=m.id;row.tabIndex=0;
  copy.append(button([m.season!=null?"S"+m.season:null,m.episode!=null?"E"+m.episode:null,m.title].filter(Boolean).join(" · "),()=>openMedia(m.id),"member-title"),node("p",[m.watched?"✓ Vu":"Non vu",!m.available?"Indisponible":null].filter(Boolean).join(" · "),"muted"));row.append(poster,copy);copy.append(actions);
  row.onclick=e=>{if(!e.target.closest("button,select"))openMedia(m.id);};row.onkeydown=e=>{if((e.key==="Enter"||e.key===" ")&&e.target===row){e.preventDefault();openMedia(m.id);}};
  const context={kind,id:view.id};const play=button("▶ Lire",()=>{if(collectionView===view&&scope===collectionScope())window.chillPlay(m.id,context);});play.disabled=!m.available;actions.append(play);
  if(collectionKind!=="series"){
    const ids=collectionView.media_ids,index=ids.indexOf(m.id);
    for(const [text,delta] of [["↑",-1],["↓",1]]){const order=collectionKind==="playlist"?playlistOrderForVisibleMove(ids,visibleIds,m.id,delta):index+delta<0||index+delta>=ids.length?null:(()=>{const next=[...ids];[next[index],next[index+delta]]=[next[index+delta],next[index]];return next;})();const b=button(text,async()=>{
      if(collectionView!==view||scope!==collectionScope())return;if(order&&await run({action:kind==="playlist"?"reorder_playlist":"reorder_saga",id:view.id,expected_revision:view.revision,expected_order:ids,media_ids:order},false))await openCollection(view.id,kind);});b.disabled=!order;b.setAttribute("aria-label",(delta<0?"Déplacer avant : ":"Déplacer après : ")+m.title);actions.append(b);}
    actions.append(button("Retirer",async()=>{if(collectionView!==view||scope!==collectionScope())return;if(await run({action:kind==="playlist"?"remove_playlist_member":"remove_saga_member",id:view.id,media_id:m.id,expected_revision:view.revision},false))await openCollection(view.id,kind);}));
    if(collectionKind==="saga"&&m.poster){const view=collectionView,server=state.server_id,profile=state.profile_id,label=m.poster_custom?"Utiliser l’affiche personnelle du film":m.tmdb_id?"Utiliser l’affiche TMDB du film":"Utiliser l’affiche du film";actions.append(button(label,async()=>{if(collectionView!==view||scope!==collectionScope())return;const updated=await run(sagaMemberArtworkCommand(view.id,view.revision,m.id),false);if(updated&&collectionView===view&&server===state.server_id&&profile===state.profile_id)await openCollection(view.id,"saga");}));}
  }
  target.append(row);if(m.poster)observeArtwork(poster,m.id);
}
async function loadCollectionArtwork(view,kind){
  const preview=$("saga-artwork-preview"),server=state.server_id,profile=state.profile_id;
  if(!preview)return;
  const label=kind==="playlist"?"liste":"saga";
  preview.replaceChildren(node("span",view.poster?"Chargement de l’affiche…":"Aucune image de "+label+". La couverture est composée des neuf premières vidéos."));
  if(!view.poster)return;
  const result=await libraryRead(collectionArtworkCommand(kind,view.id),()=>collectionView===view&&server===state.server_id&&profile===state.profile_id);
  if(collectionView!==view||server!==state.server_id||profile!==state.profile_id||!preview.isConnected)return;
  if(!result?.src)return;
  const image=node("img");image.src=result.src;image.alt="Image de la "+label;image.decoding="async";preview.replaceChildren(image);
}
$("collection-rename").onsubmit=async e=>{e.preventDefault();if(!collectionView)return;if(await run({action:collectionKind==="playlist"?"rename_playlist":"rename_saga",id:collectionView.id,expected_revision:collectionView.revision,name:$("collection-name").value},false))await openCollection(collectionView.id,collectionKind);};
$("collection-delete").onclick=()=>{$("collection-delete-confirm").hidden=false;};
$("collection-delete-yes").onclick=async()=>{const cmd=collectionView&&{action:collectionKind==="playlist"?"delete_playlist":"delete_saga",id:collectionView.id};if(cmd&&collectionKind==="saga")cmd.expected_revision=collectionView.revision;if(cmd&&await run(cmd,false)){collectionView=null;$("collection-delete-confirm").hidden=true;await openCollections(collectionKind);}};
$("collection-delete-no").onclick=()=>{$("collection-delete-confirm").hidden=true;};
let pickerSelection=new Set(),pickerItems=[],pickerGeneration=0,pickerSearchTimer=null;
async function pickerPage(){
 const view=collectionView,epoch=++pickerGeneration,context=collectionScope();
 if(!view)return;
 pickerItems=[];$("picker-items").replaceChildren(node("p","Chargement des vidéos…","muted"));$("picker-items").scrollTop=0;
 const filters={category:collectionKind==="saga"?"film":$("picker-category").value,q:$("picker-search").value,watched:$("picker-watched").value,genre:"",year:"",sort:"",available:""},ids=new Set();
 const current=()=>epoch===pickerGeneration&&view===collectionView&&context===collectionScope()&&shellSection==="picker";
 for(let at=0;current();){
  const page=await libraryRead(catalogCommand(filters,at,false),current);if(!page)return;
  if(!Array.isArray(page.items)||page.items.length>CATALOG_PAGE_SIZE||page.has_more&&!page.items.length){message("Sélecteur catalogue incohérent.");return;}
  let added=0;
  for(const media of page.items){if(!ids.has(media.id)){ids.add(media.id);added++;if(!view.media_ids.includes(media.id))pickerItems.push(media);}}
  renderPicker();
  if(!page.has_more)break;if(!added||ids.size>=30000){message("Catalogue trop volumineux ou modifié. Relancez la recherche.");return;}at+=page.items.length;
 }
}
function renderPicker(){
 const container=$("picker-items");if($("picker-panel").hidden)return;
 const rect=container.getBoundingClientRect(),view=catalogWindow(pickerItems.length,Math.max(1,rect.width),176,container.scrollTop,container.clientHeight);
 const list=node("div",undefined,"picker-virtual");list.style.height=view.height+"px";
 const grid=node("div",undefined,"picker-window");grid.style.top=view.top+"px";grid.style.gridTemplateColumns="repeat("+view.columns+",minmax(0,1fr))";
 for(let index=view.start;index<view.end;index++){
  const media=pickerItems[index],subtitle=[({film:"Film",serie:"Série",documentaire:"Documentaire",spectacle:"Spectacle"})[media.category],media.year,media.season!=null?"S"+media.season:null,media.episode!=null?"E"+media.episode:null].filter(Boolean).join(" · ");
  const {tile,poster}=posterTile(media.title,subtitle,()=>{
    if(pickerSelection.has(media.id))pickerSelection.delete(media.id);else if(pickerSelection.size<1000)pickerSelection.add(media.id);
    tile.classList.toggle("selected",pickerSelection.has(media.id));tile.setAttribute("aria-pressed",String(pickerSelection.has(media.id)));pickerCount();
  });
  tile.dataset.mediaId=media.id;tile.classList.toggle("selected",pickerSelection.has(media.id));tile.setAttribute("aria-pressed",String(pickerSelection.has(media.id)));grid.append(tile);
  if(media.poster)observeArtwork(poster,media.id);
 }
 list.append(grid);container.replaceChildren(list);
 if(!pickerItems.length)container.append(node("p","Aucune vidéo à ajouter pour ces filtres.","muted"));pickerCount();
}
function pickerCount(){$("picker-selected").textContent=pickerSelection.size+" sélectionnée(s)"+(pickerSelection.size>=1000?" — maximum atteint":"");$("picker-confirm").disabled=!pickerSelection.size;}
function closePicker(){pickerGeneration++;clearTimeout(pickerSearchTimer);pickerSelection.clear();showCollectionPage("collection");renderCollectionMembers();}
$("picker-items").addEventListener("scroll",renderPicker,{passive:true});
$("collection-add").onclick=async()=>{
 if(!collectionView||collectionKind==="series")return;
 pickerSelection=new Set();$("picker-search").value="";$("picker-category").value=collectionKind==="saga"?"film":"";$("picker-category").disabled=collectionKind==="saga";$("picker-watched").value="";
 $("picker-hint").textContent=collectionKind==="saga"?"Une saga regroupe des films. Sélectionnez leurs vignettes.":"Films, épisodes de séries, documentaires et spectacles : sélectionnez leurs vignettes.";
 showCollectionPage("picker");await pickerPage();
};
$("picker-confirm").onclick=async()=>{
 const view=collectionView,kind=collectionKind,context=collectionScope();if(!view||!pickerSelection.size)return;
 const ids=[...pickerSelection],r=await run({action:kind==="playlist"?"add_playlist":"add_saga",id:view.id,expected_revision:view.revision,media_ids:ids},false);
 if(r&&view===collectionView&&context===collectionScope()){pickerSelection.clear();pickerGeneration++;await openCollection(view.id,kind);}
};
$("picker-cancel").onclick=closePicker;$("picker-back").onclick=closePicker;
$("picker-form").onsubmit=e=>{e.preventDefault();clearTimeout(pickerSearchTimer);pickerPage();};
$("picker-search").oninput=()=>{clearTimeout(pickerSearchTimer);pickerSearchTimer=setTimeout(pickerPage,250);};
for(const id of ["picker-category","picker-watched"])$(id).onchange=()=>{clearTimeout(pickerSearchTimer);pickerPage();};
let collectionFrame=null;
function scheduleCollectionWindow(){if(collectionFrame)return;collectionFrame=requestAnimationFrame(()=>{collectionFrame=null;renderCollectionMembers();});}
window.addEventListener("scroll",scheduleCollectionWindow,{passive:true});
window.addEventListener("resize",()=>{scheduleCollectionWindow();if(shellSection==="picker")renderPicker();});
$("show-playlists").onclick=()=>openCollections();$("show-sagas").onclick=()=>openCollections("saga");
for(const id of ["media-dialog","playlist-target-dialog"]){const m=node("p",undefined,"dialog-message");m.setAttribute("role","status");m.setAttribute("aria-live","polite");$(id).prepend(m);$(id).querySelector(".dialog-close").onclick=()=>$(id).close();}
let artworkQueue=[],artworkLoading=false;
const artworkTargets=new Set();
function discardDetachedArtwork(){for(const target of artworkTargets){if(!target.isConnected){artworkObserver.unobserve(target);artworkTargets.delete(target);}}artworkQueue=artworkQueue.filter(item=>item.target.isConnected);}
window.resetArtwork=()=>{for(const target of artworkTargets){if(!target.isConnected||$("media").contains(target)){artworkObserver.unobserve(target);artworkTargets.delete(target);}}artworkQueue=artworkQueue.filter(item=>item.target.isConnected&&!$("media").contains(item.target));};
const artworkObserver=new IntersectionObserver(entries=>{
  // The callback runs after a whole virtual grid has been inserted. Pruning in
  // observeArtwork would unobserve earlier siblings still in its detached grid.
  discardDetachedArtwork();
  for(const e of entries){if(!e.target.isConnected)continue;if(e.isIntersecting){if(!e.target.dataset.loaded&&!e.target.dataset.queued){e.target.dataset.queued="true";artworkQueue.push({target:e.target,id:e.target.dataset.mediaId,kind:e.target.dataset.artworkKind,context:collectionScope()});}}else{e.target.replaceChildren(node("span",e.target.dataset.initial||""));delete e.target.dataset.loaded;}}
  drainArtwork();
},{rootMargin:"100px"});
function observeArtwork(target,id,kind="media"){target.dataset.mediaId=id;target.dataset.artworkKind=kind;target.dataset.initial=target.textContent;artworkTargets.add(target);artworkObserver.observe(target);}
async function drainArtwork(){
  if(artworkLoading)return;artworkLoading=true;
  while(artworkQueue.length){
    const item=artworkQueue.shift();
    if(!item.target.isConnected||item.context!==collectionScope()||item.target.dataset.loaded){delete item.target.dataset.queued;continue;}
    if(busy||playerBusy){artworkQueue.unshift(item);await new Promise(r=>setTimeout(r,300));continue;}
    try{
      const r=await invoke(item.kind==="media"?{action:"artwork",media_id:item.id}:collectionArtworkCommand(item.kind,item.id));
      if(!item.target.isConnected||item.context!==collectionScope())continue;
      const img=node("img");img.src=r.src;img.alt="";img.decoding="async";item.target.replaceChildren(img);item.target.dataset.loaded="true";
    }catch(e){if(e.code==="CLIENT_BUSY"){artworkQueue.unshift(item);await new Promise(r=>setTimeout(r,300));}}
    finally{if(!artworkQueue.includes(item))delete item.target.dataset.queued;}
  }
  artworkLoading=false;
}
window.chillArtwork=observeArtwork;
