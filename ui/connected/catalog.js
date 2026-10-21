"use strict";
var uiText=globalThis.chillI18n?.t||((text)=>text);
// Catalogue and personal collections share the allowlisted Rust bridge. No URLs or
// filesystem paths from media metadata are evaluated by this interface.
let catalogProfile=typeof state==="undefined"?null:state.server_id+":"+state.profile_id,collectionView=null,collectionKind="playlist",detailView=null,seriesView=null;
let collectionFilter="",collectionDetails=new Map(),collectionLoaded=0,collectionCards=new Map(),collectionCardsOrder=[];
let collectionEpoch=0,collectionContext="",detailGeneration=0,detailId=null;
function detailCurrent(generation,context,id){return generation===detailGeneration&&context===collectionScope()&&detailId===id&&$("media-dialog").open;}
function invalidateDetail(){detailGeneration++;detailId=null;detailView=null;window.recoveryContextChanged?.();}
$("media-dialog").addEventListener("close",()=>{if(!$("media-dialog").open){retireArtwork($("detail-poster"));retireRecommendationRail($("related-media"));invalidateDetail();window.restoreCatalogLayout?.();}});
$("media-dialog").addEventListener("click",event=>{if(event.target!==$("media-dialog"))return;const r=event.target.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)event.target.close();});
function collectionScope(){return state.server_id+":"+state.profile_id;}
function collectionCurrent(epoch,context){return epoch===collectionEpoch&&context===collectionScope();}
async function libraryRead(command,current){
  for(let attempt=0;attempt<50&&current();attempt++){
    try{const result=await invoke(command);return current()?result:null;}
    catch(error){if(error.code!=="CLIENT_BUSY"){if(current())message(error.message||uiText("Lecture indisponible."));return null;}await new Promise(resolve=>setTimeout(resolve,100));}
  }
  if(current())message(uiText("Le client est occupé. Réessayez dans un instant."));return null;
}
function showCollectionPage(section){shellShow(section);scrollTo({top:0,behavior:"instant"});}
window.leaveCollectionPages=()=>{collectionEpoch++;pickerGeneration++;collectionView=null;seriesView=null;collectionDetails=new Map();discardCollectionCards();};
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
  if(m.poster)observeArtwork(card.querySelector(".poster-image"),m.id);
};
window.catalogOpenMedia=openMedia;window.catalogOpenSeries=openSeries;
function detailPlaybackContext(mediaId){if(window.isProgressView?.()&&detailView?.media.id===mediaId&&detailView.media.category==="serie"){const m=detailView.media;return {kind:"series",id:m.tmdb_id?"tmdb:tv:"+m.tmdb_id:"media:"+m.id};}return shellSection==="collection"&&collectionView?.media_ids.includes(mediaId)?{kind:collectionKind,id:collectionView.id}:null;}
window.catalogReturnFromPlayback=async(playback,current)=>{
  if(!current())return;
  const context=playback.context;
  if(context?.kind==="series"&&context.id)await openSeries(context.id);
  else if((context?.kind==="saga"||context?.kind==="playlist")&&context.id)await openCollection(context.id,context.kind);
  else if(playback.media_id)await openMedia(playback.media_id);
};
async function importArtwork(target,reload){const recovery=window.conflictTarget?.({...target,action:"artwork"});try{const result=await window.__TAURI__.core.invoke("artwork_command",{target});if(!result?.cancelled)await reload();}catch(error){window.offerConflictRecovery?.(error,recovery);message(error?.message||uiText("Import de l'image non effectué."));}}
async function openMedia(id){
  const same=detailId===id&&$("media-dialog").open,scroll=same?$("media-dialog").scrollTop:0,generation=++detailGeneration,context=collectionScope();detailId=id;detailView=null;
  const current=()=>detailCurrent(generation,context,id);window.recoveryContextChanged?.();
  // Even a same-media refresh may remove its poster. Retire the old observer
  // identity before awaiting metadata, not just the current image child/token.
  retireArtwork($("detail-poster"));$("detail-poster").replaceChildren();
  if(!same&&$("detail-progress")){$("detail-progress").hidden=true;$("detail-progress").textContent="";}
  if(!same){$("detail-title").textContent=uiText("Chargement…");$("detail-overview").textContent="";$("detail-metadata").textContent="";$("detail-availability").textContent="";$("detail-path")?.replaceChildren();$("detail-tmdb")?.replaceChildren();$("detail-stars")?.replaceChildren();retireRecommendationRail($("related-media"));$("related-media")?.replaceChildren();$("detail-play").disabled=true;$("detail-addlist").disabled=true;if($("detail-artwork"))$("detail-artwork").disabled=true;$("metadata-form").hidden=true;}
  if(!$("media-dialog").open)$("media-dialog").showModal();
  const result=await libraryRead({action:"media_detail",media_id:id},current);if(!result||!current())return;detailView=result;
  const m=result.media;$("detail-title").textContent=m.title;$("detail-overview").textContent=m.overview||uiText("Informations à compléter");
  window.renderDetailProgress?.(m);void window.refreshProgress?.();
  $("detail-metadata").textContent=[m.year,m.runtime_minutes?m.runtime_minutes+" min":null,...m.genres,...m.directors.map(n=>uiText("Réalisation : ")+n),...m.cast].filter(Boolean).join(" · ");
  let sourcePath=$("detail-path");if(!sourcePath){sourcePath=node("code",undefined,"detail-path muted");sourcePath.id="detail-path";sourcePath.setAttribute("aria-label",uiText("Chemin relatif de la vidéo sur le serveur"));$("detail-metadata").after(sourcePath);}sourcePath.textContent=m.path||"";sourcePath.hidden=!m.path;
  renderMediaAvailability(m);
  $("detail-poster").replaceChildren(node("span",m.title.slice(0,1).toUpperCase()));if(m.poster)observeArtwork($("detail-poster"),m.id);
  $("detail-addlist").disabled=false;$("detail-addlist").onclick=()=>window.catalogPickCollection(m.id,m.category);
  const reload=async()=>{if(context===collectionScope())await syncAfterMutation();if(current())await openMedia(m.id);};
  $("detail-play").disabled=!m.available;$("detail-play").onclick=()=>{const context=detailPlaybackContext(m.id);$("media-dialog").close();window.chillPlay(m.id,context);};let artwork=$("detail-artwork");if(!artwork){artwork=button(uiText("Importer une image"),()=>{});artwork.id="detail-artwork";$("detail-play").parentElement.append(artwork);}artwork.disabled=false;artwork.onclick=()=>importArtwork({kind:"media",media_id:m.id,expected_revision:result.revision},reload);
  let tmdb=$("detail-tmdb");if(!tmdb){tmdb=document.createElement("section");tmdb.id="detail-tmdb";$("media-dialog").querySelector(".detail-copy").append(tmdb);}renderTmdbPanel(tmdb,m,"media",current,reload);
  $("detail-edit").hidden=false;$("metadata-form").hidden=true;
  let stars=$("detail-stars");if(!stars){stars=node("div",undefined,"rating-stars");stars.id="detail-stars";$("detail-rate").parentElement.replaceChildren(stars);}renderRatingStars(stars,m.rating,async rating=>{if(!current())return;if(await libraryRead({action:"group_rating",media_id:id,rating,scope:"video"},current))await reload();});
  $("media-dialog").scrollTop=scroll;
  loadRelatedMedia(m.id);
}
function renderRatingStars(target,rating,save){target.replaceChildren();target.setAttribute("aria-label",uiText("Ma note sur cinq"));for(let value=1;value<=5;value++){const star=button(value<=(rating||0)?"★":"☆",async()=>{if(target.dataset.saving)return;target.dataset.saving="true";try{await save(value);}finally{delete target.dataset.saving;}},"rating-star");star.setAttribute("aria-label",value+uiText(" sur 5"));star.setAttribute("aria-pressed",String(value<=(rating||0)));target.append(star);}}
function renderMediaAvailability(media){
 $("detail-availability").textContent=media.available?"":uiText("Indisponible — le fichier n’est pas accessible. Retrouvez son emplacement ou gérez sa référence dans Configuration › Bibliothèque.");$("detail-availability").hidden=media.available;
 let locate=$("detail-locate");if(!locate){locate=button(uiText("Retrouver ou gérer le fichier"),async()=>{});locate.id="detail-locate";$("detail-play").parentElement.append(locate);}locate.hidden=media.available;
 locate.onclick=async()=>{$("media-dialog").close();await $("admin-toggle").onclick();window.configurationShow?.("library");};
}
window.catalogDetailUnavailable=mediaId=>{
 const media=collectionDetails.get(mediaId);if(media)collectionDetails.set(mediaId,{...media,available:false});
 if(seriesView?.episodes)for(const episode of seriesView.episodes)if(episode.id===mediaId)episode.available=false;
 const card=collectionCards.get(mediaId);if(card){retireArtwork(card.querySelector(".poster"));collectionCards.delete(mediaId);}
 if(detailView?.media.id===mediaId){detailView.media.available=false;$("detail-play").disabled=true;renderMediaAvailability(detailView.media);}
 if(collectionView&&collectionKind!=="series")renderCollectionMembers();
};
function renderTmdbPanel(target,media,scope,current,reload){
 target.replaceChildren(node("h3","TMDB"));const query=document.createElement("input"),results=node("div",undefined,"tmdb-candidates"),association=node("p",media.tmdb_id?uiText("Référence associée : ")+(media.series||media.title)+uiText(" (TMDB ")+media.tmdb_id+").":"","tmdb-association-status");association.setAttribute("role","status");association.setAttribute("aria-live","polite");query.type="search";query.maxLength=500;query.value=scope==="media"?media.title:media.series||media.title;query.setAttribute("aria-label",uiText("Titre à rechercher sur TMDB"));let searchGeneration=0,mutating=false;
 async function mutate(candidate,action){if(mutating||!current())return;mutating=true;searchGeneration++;const label=candidate?[candidate.title,candidate.year].filter(Boolean).join(" · "):media.series||media.title;results.replaceChildren();association.textContent=(candidate?uiText("Association en cours avec "):uiText("Actualisation en cours de "))+label+"…";association.className="tmdb-association-status is-working";association.setAttribute("aria-busy","true");query.disabled=search.disabled=refresh.disabled=true;let applied=false;
  try{const fresh=await libraryRead({action:"media_detail",media_id:media.id},current);if(!fresh)return;let revision=fresh.revision;
  // A season is selected inside an identified parent show, never merged by title.
  if(scope==="season"&&candidate&&fresh.media.tmdb_id!==candidate.id){const parent=await libraryRead({action:"tmdb_select",media_id:media.id,expected_revision:revision,tmdb_id:candidate.id,kind:"tv",scope:"series"},current);if(!parent)return;await syncAfterMutation();const updated=await libraryRead({action:"media_detail",media_id:media.id},current);if(!updated)return;revision=updated.revision;}
  const command=candidate?{action:"tmdb_select",media_id:media.id,expected_revision:revision,tmdb_id:candidate.id,kind:candidate.kind,scope}:tmdbCommand(action,media.id,revision,scope);
  const result=await libraryRead(command,current);if(result&&!result.discarded&&current()){applied=true;association.textContent=uiText("Association confirmée : ")+label+".";association.className="tmdb-association-status";await syncAfterMutation();if(current())await reload();}
  }finally{mutating=false;if(current()){association.className="tmdb-association-status";association.setAttribute("aria-busy","false");query.disabled=search.disabled=refresh.disabled=false;if(!applied)association.textContent=uiText("Association non confirmée avec ")+label+uiText(". Consultez le message d’erreur puis relancez la recherche pour réessayer.");}}
 }
 const search=button(uiText("Rechercher TMDB"),async()=>{if(mutating)return;const generation=++searchGeneration;results.replaceChildren(node("p","Recherche…","muted"));const automatic=query.value.trim()===(scope==="media"?media.title:media.series||media.title).trim();const response=await libraryRead({action:"tmdb_candidates",media_id:media.id,query:automatic?"":query.value},()=>current()&&generation===searchGeneration);if(!response)return;results.replaceChildren();for(const candidate of response.candidates||[]){if(scope!=="media"&&candidate.kind!=="tv")continue;results.append(button([candidate.title,candidate.year].filter(Boolean).join(" · "),()=>mutate(candidate)));}if(!results.childElementCount)results.append(node("p",uiText("Aucune correspondance."),"muted"));}),refresh=button(uiText("Actualiser les informations TMDB"),()=>mutate(null,"refresh"));target.append(query,search,refresh,association,results);
 if(scope!=="media")target.append(button(uiText("Importer une image"),async()=>{const fresh=await libraryRead({action:"media_detail",media_id:media.id},current);if(fresh)await importArtwork({kind:scope,media_id:media.id,expected_revision:fresh.revision},async()=>{await syncAfterMutation();if(current())await reload();});}));
}
// A recommendation is a title-level object. Rust groups shows and sends
// one representative artwork id; the client still rejects raw episodes so an
// older server response can never leak them back into either recommendation UI.
function recommendationItems(items){
 const seen=new Set(),valid=[];
 for(const item of Array.isArray(items)?items:[]){
  if(!item||typeof item.id!=="string"||!item.id)continue;
  const series=item.category==="serie";
  if(series&&(!item.series_key||item.season!=null||item.episode!=null))continue;
  if(!series&&(item.season!=null||item.episode!=null))continue;
  const key=series?"series:"+item.series_key:"media:"+item.id;
  if(seen.has(key))continue;seen.add(key);valid.push(item);
 }
 return valid;
}
function recommendationScroll(viewport,direction){
 const reduce=globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
 viewport.scrollBy({left:direction*viewport.clientWidth,behavior:reduce?"auto":"smooth"});
}
function catalogRecommendationRail(items,{label="",beforeOpen}={}){
 const recommended=recommendationItems(items),context=collectionScope();if(!recommended.length)return null;
 const rail=node("section",undefined,"recommendation-rail-wrap"),head=node("div",undefined,"recommendation-controls"),viewport=node("div",undefined,"recommendation-rail");
 viewport.tabIndex=0;viewport.setAttribute("role","region");viewport.setAttribute("aria-label",label||uiText("Sélection de vidéos recommandées"));
 if(label)rail.append(node("h3",label));
 const previous=button("←",()=>recommendationScroll(viewport,-1),"secondary"),next=button("→",()=>recommendationScroll(viewport,1),"secondary");
 previous.setAttribute("aria-label",uiText("Voir les recommandations précédentes"));next.setAttribute("aria-label",uiText("Voir les recommandations suivantes"));head.append(previous,next);rail.append(head,viewport);
 for(const item of recommended){
  const displayTitle=typeof item.title==="string"&&item.title?item.title:uiText("Sans titre"),card=document.createElement("button"),poster=node("div",displayTitle.slice(0,1).toUpperCase(),"poster recommendation-poster"),title=node("span",displayTitle,"recommendation-title"),reasons=Array.isArray(item.matches)&&item.matches.length?node("span",item.matches.join(" · "),"recommendation-reasons"):null;
  card.type="button";card.className="recommendation-card";card.append(poster,title,node("span",[({film:uiText("Film"),serie:uiText("Série"),documentaire:uiText("Documentaire"),spectacle:uiText("Spectacle")})[item.category],item.year].filter(Boolean).join(" · "),"recommendation-meta"));if(reasons)card.append(reasons);
  card.onclick=async()=>{if(!card.isConnected||context!==collectionScope()||beforeOpen&&beforeOpen(item)===false)return;if(item.category==="serie"){if($("media-dialog").open)$("media-dialog").close();await openSeries(item.series_key);}else await openMedia(item.id);};
  viewport.append(card);if(item.category!=="serie")window.renderMediaProgress?.(poster,item);if(item.poster){poster.dataset.recommendationMediaId=item.id;observeArtwork(poster,item.id);}
 }
 return rail;
}
function retireRecommendationRail(target){
 if(!target)return;
 for(const poster of target.querySelectorAll?.(".recommendation-poster")||[])retireArtwork(poster);
}
window.catalogRecommendationRail=catalogRecommendationRail;
window.catalogRetireRecommendationRail=retireRecommendationRail;
window.catalogSetRecommendationRailVisible=(target,visible)=>{
 if(!visible){retireRecommendationRail(target);return;}
 // Keep the selection and decoded posters when returning; only resume pending
 // artwork. Retiring a hidden rail invalidates any response already in flight.
 for(const poster of target?.querySelectorAll(".recommendation-poster")||[]){
  if(poster.dataset.recommendationMediaId&&!poster.dataset.mediaId&&!poster.querySelector("img"))observeArtwork(poster,poster.dataset.recommendationMediaId);
 }
};
async function loadRelatedMedia(mediaId){
  let section=$("related-media");if(!section){section=document.createElement("section");section.id="related-media";$("media-dialog").append(section);}retireRecommendationRail(section);section.replaceChildren(node("h3",uiText("Vidéos en relation")));section.hidden=true;
  const generation=detailGeneration,context=collectionScope();const result=await libraryRead(relatedMediaCommand(mediaId),()=>detailCurrent(generation,context,mediaId));if(!result||!detailView||detailView.media.id!==mediaId)return;
  const rail=catalogRecommendationRail(result.items||[],{beforeOpen:()=>detailCurrent(generation,context,mediaId)});if(rail)section.append(rail);section.hidden=!rail;
}
$("detail-edit").onclick=()=>{
  if(!detailView)return;const m=detailView.media;
  for(const key of ["title","category","year","series","season","episode","overview","runtime_minutes","series_overview"])$("edit-"+key).value=m[key]??"";
  for(const key of ["genres","cast","directors"])$("edit-"+key).value=m[key].join(", ");
  $("metadata-form").hidden=false;
};
$("metadata-form").onsubmit=async e=>{
  e.preventDefault();if(!detailView)return;const patch={},generation=detailGeneration,context=collectionScope(),id=detailId,current=()=>detailCurrent(generation,context,id);
  for(const key of ["title","category","series","overview","series_overview"])patch[key]=$("edit-"+key).value;
  for(const key of ["year","runtime_minutes"])if($("edit-"+key).value!=="")patch[key]=Number($("edit-"+key).value);
  for(const key of ["season","episode"])patch[key]=$("edit-"+key).value===""?null:Number($("edit-"+key).value);
  for(const key of ["genres","cast","directors"])patch[key]=$("edit-"+key).value.split(",").map(s=>s.trim()).filter(Boolean);
  const result=await libraryRead({action:"edit_media",media_id:id,expected_revision:detailView.revision,patch},current);
  if(result){await syncAfterMutation();if(current())await openMedia(id);}else if(current()){$("detail-availability").textContent=uiText("Enregistrement non effectué. Votre saisie est conservée ; utilisez l’action d’actualisation si un conflit est signalé.");$("detail-availability").hidden=false;}
};
window.catalogConflictTarget=command=>{
 const context=collectionScope();
 if(command.media_id&&detailView?.media.id===command.media_id&&$("media-dialog").open){
  const id=detailId,generation=detailGeneration,current=()=>detailCurrent(generation,context,id);
  return {root:$("media-dialog"),label:"Actualiser cette fiche",current,identityCurrent:()=>context===collectionScope()&&detailId===id&&$("media-dialog").open,async reload(){
   await window.recoveryRefreshCatalogue();if(!current())return false;
   const form=$("metadata-form"),draft=window.captureRecoveryFields(form),editing=!form.hidden;
   await openMedia(id);if(context!==collectionScope()||detailId!==id||!detailView)return false;
   window.restoreRecoveryFields(draft,$("metadata-form"));$("metadata-form").hidden=!editing;return true;
  }};
 }
 if(collectionView&&(command.id===collectionView.id||command.series===collectionView.id||command.media_id&&collectionView.media_ids.includes(command.media_id))){
  const view=collectionView,kind=collectionKind,epoch=collectionEpoch,current=()=>collectionCurrent(epoch,context)&&collectionView===view;
  return {root:$(shellSection==="picker"?"picker-panel":"collection-panel"),label:kind==="series"?"Actualiser cette série":kind==="saga"?"Actualiser cette saga":"Actualiser cette liste",current,identityCurrent:()=>context===collectionScope()&&collectionView?.id===view.id&&collectionKind===kind&&["collection","picker"].includes(shellSection),async reload(){
   await window.recoveryRefreshCatalogue();if(!current())return false;
   if(kind==="series"){
    const form=$("series-tools")?.querySelector(".series-form"),draft=window.captureRecoveryFields(form),editing=form&&!form.hidden;
    await openSeries(view.id);if(context!==collectionScope()||collectionView?.id!==view.id)return false;
    const next=$("series-tools")?.querySelector(".series-form");if(next){window.restoreRecoveryFields(draft,next);next.hidden=!editing;}return true;
   }
   const result=await invoke({action:kind==="saga"?"saga":"playlist",id:view.id});if(!current())return false;
   // Keep the object captured by move/picker handlers and the user's rename
   // field/selection. Refresh its CAS revision and membership, never its draft.
   Object.assign(view,result.saga||result.playlist);$("collection-title").textContent=view.name;$("collection-counts").textContent=view.media_ids.length+uiText(" vidéos");
   discardCollectionCards();await collectionPage(view.id,0);return current();
  }};
 }
 return null;
};
async function openSeries(name){
  if(!await stopPlayerForNavigation())return;
  const previous=shellSection==="collection"&&collectionKind==="series"?seriesView:null,previousScroll=globalThis.scrollY||0,openSeasons=new Set(Array.from($("collection-members").querySelectorAll?.(".series-season[open]")||[],section=>section.dataset.season));
  const epoch=++collectionEpoch,context=collectionScope();
  const result=await libraryRead({action:"series",name},()=>collectionCurrent(epoch,context));if(!result)return;
  seriesView=result;collectionKind="series";collectionView={id:name,name:result.metadata?.name||result.episodes[0]?.series||result.name||name,media_ids:result.episodes.map(m=>m.id),episodes:result.episodes};
  collectionFilter="";collectionDetails=new Map();collectionLoaded=0;ensureCollectionFilter("series");
  $("collection-title").textContent=collectionView.name;$("collection-counts").textContent=new Set(result.episodes.map(m=>m.season)).size+uiText(" saisons · ")+result.episodes.length+uiText(" épisodes · ")+result.counters.unwatched+uiText(" épisodes non vus");$("collection-counts").hidden=true;
  $("collection-admin").hidden=true;$("collection-add").hidden=true;$("collection-members").replaceChildren();
  renderSeriesCommon(result);
  const ordered=[...result.episodes].sort((a,b)=>(a.season===null?Infinity:a.season)-(b.season===null?Infinity:b.season)||(a.episode===null?Infinity:a.episode)-(b.episode===null?Infinity:b.episode)||a.title.localeCompare(b.title,"fr"));
  const same=previous?.episodes?.some(old=>result.episodes.some(item=>item.id===old.id));
  // Choose from the sorted per-profile episodes only on entry; refreshes preserve manual folds.
  const firstSeason=(ordered.find(episode=>!episode.watched)||ordered[0])?.season;
  let season=undefined,section=null,episodeGrid=null;
  for(const m of ordered){
   if(m.season!==season){
    season=m.season;section=document.createElement("details");section.className="series-season";section.dataset.season=String(season);section.open=same?openSeasons.has(String(season)):season===firstSeason;
    const episodes=ordered.filter(e=>e.season===season),watched=episodes.filter(e=>e.watched).length;
    section.append(node("summary",(season===null?uiText("Saison inconnue"):uiText("Saison ")+season)+" · "+episodes.length+uiText(" épisodes · ")+(episodes.length-watched)+uiText(" non vus")));
    const provider=node("section",undefined,"season-provider"),metadata=result.season_metadata?.[season];if(metadata)section.append(node("p",metadata.overview||"","muted"));
    renderTmdbPanel(provider,m,"season",()=>collectionCurrent(epoch,context),async()=>{if(collectionCurrent(epoch,context)){const detail=await libraryRead({action:"media_detail",media_id:m.id},()=>collectionCurrent(epoch,context));if(detail)await openSeries(detail.media.tmdb_id?"tmdb:tv:"+detail.media.tmdb_id:name);await loadPage();}});
    section.append(provider);appendGroupRating(section,episodes,"season");
    // Native details contents need a real grid wrapper, not flex on the details element itself.
    episodeGrid=node("div",undefined,"collection-grid series-episodes");section.append(episodeGrid);
    $("collection-members").append(section);
   }
   appendCollectionMember(m,undefined,episodeGrid);
  }
  $("collection-members").classList.add("series-members");$("collection-members").style.height="";
  if(result.tmdb_id){const sections=$("collection-members").querySelectorAll(".series-season");[...new Set(ordered.map(m=>m.season))].forEach((season,index)=>{if(!result.season_metadata?.[season]?.poster)return;const poster=node("div","", "poster series-poster");sections[index].append(poster);observeArtwork(poster,result.tmdb_id+":"+season,"season");});}
  if(same){shellShow("collection");scrollTo({top:previousScroll,behavior:"instant"});}else showCollectionPage("collection");
}
function ratingSummary(items){const ratings=[...new Set(items.map(item=>item.rating??null))];return ratings.length===1?(ratings[0]===null?uiText("Sans note"):uiText("Ma note : ")+"★".repeat(ratings[0])):uiText("Notes différentes");}
function appendGroupRating(target,items,scope){
  if(!items.length)return;const controls=node("div",undefined,"rating-stars"),epoch=collectionEpoch,context=collectionScope(),key=collectionView.id,uniform=new Set(items.map(item=>item.rating??null));renderRatingStars(controls,uniform.size===1?items[0].rating:null,async rating=>{if(await libraryRead({action:"group_rating",media_id:items[0].id,rating,scope},()=>collectionCurrent(epoch,context))){await syncAfterMutation();if(collectionCurrent(epoch,context))await openSeries(key);}});target.append(controls);
}
function renderSeriesCommon(result){
  let section=$("series-common");if(!section){section=document.createElement("section");section.id="series-common";$("collection-members").before(section);}section.className="series-overview";section.replaceChildren();const m=result.metadata||{};
  let tools=$("series-tools");if(!tools){tools=document.createElement("section");tools.id="series-tools";$("collection-members").after(tools);}tools.className="series-tools";tools.replaceChildren(node("h2",uiText("Images et informations communes")));
  const illustrated=result.episodes.find(e=>e.poster),poster=node("div",collectionView.name.slice(0,1).toUpperCase(),"poster series-poster"),copy=node("div",undefined,"series-overview-copy");poster.setAttribute("aria-label",uiText("Affiche de la série"));section.append(poster,copy);if(illustrated)observeArtwork(poster,illustrated.id);
  copy.append(node("p",[m.year===0?uiText("Année inconnue"):m.year,m.genres?.join(", ")].filter(Boolean).join(" · "),"muted"),node("p",m.overview||uiText("Informations communes à compléter")),node("p",$("collection-counts").textContent,"series-summary-counts"));appendGroupRating(copy,result.episodes,"series");
  // Keep metadata tools after the seasons, not between the synopsis and episodes.
  if(result.episodes.length){const provider=node("section"),epoch=collectionEpoch,context=collectionScope(),anchor=result.episodes[0];renderTmdbPanel(provider,anchor,"series",()=>collectionCurrent(epoch,context),async()=>{if(!collectionCurrent(epoch,context))return;const detail=await libraryRead({action:"media_detail",media_id:anchor.id},()=>collectionCurrent(epoch,context));if(detail)await openSeries(detail.media.tmdb_id?"tmdb:tv:"+detail.media.tmdb_id:result.name);await loadPage();});tools.append(provider);}
  const form=document.createElement("form"),edit=button(uiText("Modifier les informations communes"),()=>form.hidden=!form.hidden);form.className="series-form";form.hidden=true;
  const fields=[["name",uiText("Nom de la série"),m.name??collectionView.name],["year",uiText("Année"),m.year??""],["genres",uiText("Genres séparés par des virgules"),(m.genres||[]).join(", ")],["overview",uiText("Résumé commun"),m.overview??""],["cast",uiText("Distribution"),(m.cast||[]).join(", ")],["directors",uiText("Réalisation"),(m.directors||[]).join(", ")]];
  for(const [key,label,value] of fields){const input=key==="overview"?document.createElement("textarea"):document.createElement("input");input.name=key;input.value=value;input.maxLength=key==="name"?500:20000;if(key==="year"){input.type="number";input.min="0";input.max="2200";}const wrapper=document.createElement("label");wrapper.textContent=label;wrapper.append(input);form.append(wrapper);}form.append(button(uiText("Enregistrer")));const key=collectionView.id,epoch=collectionEpoch,context=collectionScope();form.onsubmit=async event=>{event.preventDefault();if(!collectionCurrent(epoch,context))return;const values=form.elements,split=key=>values[key].value.split(",").map(value=>value.trim()).filter(Boolean),patch={name:values.name.value,year:values.year.value===""?0:Number(values.year.value),genres:split("genres"),overview:values.overview.value,cast:split("cast"),directors:split("directors")};const updated=await run(seriesEditCommand(key,result.revision,patch),false);if(updated&&collectionCurrent(epoch,context)){await openSeries(updated.key||key);await loadPage();}};tools.append(edit,form);
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
  discardCollectionCards();
  const epoch=++collectionEpoch,context=collectionScope(),result=await libraryRead({action:kind==="playlist"?"playlists":"sagas"},()=>collectionCurrent(epoch,context));if(!result)return;
  collectionView=null;seriesView=null;collectionKind=kind;collectionContext=context;
  const title=kind==="playlist"?uiText("Listes de lecture"):uiText("Sagas");
  $("collections-title").textContent=title;$("collections-list").replaceChildren();$("collection-create").hidden=true;
  $("collection-create-toggle").setAttribute("aria-label",kind==="saga"?uiText("Créer une saga"):uiText("Créer une liste de lecture"));
  $("collection-create").elements.name.placeholder=kind==="saga"?uiText("Nom de la saga"):uiText("Nom de la liste de lecture");
  for(const c of result.items){
    const {tile,poster}=posterTile(c.name,c.media_ids.length+uiText(" vidéos"),()=>openCollection(c.id,kind));tile.dataset.collectionId=c.id;
    $("collections-list").append(tile);
    if(c.media_ids.length||c.poster)collectionMosaic(poster,c,kind,epoch,context);
  }
  if(!result.items.length)$("collections-list").append(node("p",uiText("Aucune collection. Utilisez + pour en créer une."),"muted"));
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
let targetGeneration=0;
$("playlist-target-dialog").addEventListener("close",()=>{if(!$("playlist-target-dialog").open)targetGeneration++;});
window.catalogPickCollection=async(media,category)=>{
 const generation=++targetGeneration,context=collectionScope(),current=()=>context===collectionScope()&&generation===targetGeneration;
 const [lists,sagas]=await Promise.all([libraryRead({action:"playlists"},current),category==="film"?libraryRead({action:"sagas"},current):Promise.resolve({items:[]})]);if(!lists||!sagas||!current())return;
 async function save(command){if(!current())return;const result=await libraryRead(command,current);if(result&&!result.discarded){await syncAfterMutation();if(current())$("playlist-target-dialog").close();}return result;}
 $("playlist-targets").replaceChildren();for(const [kind,title,result] of [["playlist",uiText("Listes de lecture"),lists],["saga",uiText("Sagas"),sagas]]){if(kind==="saga"&&category!=="film")continue;$("playlist-targets").append(node("h3",title));for(const c of result.items||[])$("playlist-targets").append(button(c.name,()=>save({action:kind==="saga"?"add_saga":"add_playlist",id:c.id,expected_revision:c.revision,media_ids:[media]})));}
 $("playlist-target-create").onsubmit=async e=>{e.preventDefault();if(await save({action:"create_playlist",name:e.currentTarget.elements.name.value,media_id:media}))$("playlist-target-create").reset();};if(!$("playlist-target-dialog").open)$("playlist-target-dialog").showModal();
};
async function openCollection(id,kind="playlist"){
  if(!await stopPlayerForNavigation())return;
  const epoch=++collectionEpoch,context=collectionScope();
  const r=await libraryRead({action:kind==="playlist"?"playlist":"saga",id},()=>collectionCurrent(epoch,context));if(!r)return;collectionKind=kind;collectionView=r.playlist||r.saga;collectionContext=context;
  $("series-common")?.remove();$("series-tools")?.remove();seriesView=null;
  $("collection-title").textContent=collectionView.name;$("collection-counts").textContent=collectionView.media_ids.length+uiText(" vidéos");$("collection-counts").hidden=false;
  $("collection-name").value=collectionView.name;$("collection-admin").hidden=false;$("collection-admin").open=false;$("collection-add").hidden=false;discardCollectionCards();$("collection-members").replaceChildren();$("collection-members").classList.remove("series-members");
  $("collection-admin").querySelector("summary").textContent=kind==="playlist"?uiText("Gérer la liste"):uiText("Gérer la saga");
  $("collection-delete-confirm").hidden=true;
  let artwork=$("collection-artwork"),preview=$("saga-artwork-preview");if(!artwork){artwork=button(uiText("Importer une image"),()=>{});artwork.id="collection-artwork";$("collection-admin").append(artwork);}if(!preview){preview=document.createElement("div");preview.id="saga-artwork-preview";$("collection-admin").append(preview);}artwork.hidden=false;preview.hidden=false;const view=collectionView;artwork.textContent=kind==="playlist"?uiText("Importer une image de liste"):uiText("Importer une image de saga");artwork.onclick=()=>importArtwork({kind,id:view.id,expected_revision:view.revision},async()=>await openCollection(view.id,kind));void loadCollectionArtwork(view,kind);
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
    const input=document.createElement("input");input.type="search";input.name="q";input.placeholder=uiText("Filtrer les titres");input.setAttribute("aria-label",uiText("Filtrer les titres de la collection"));input.maxLength=500;
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
window.restoreCollectionLayout=()=>renderCollectionMembers();
function discardCollectionCards(){for(const card of collectionCards.values())retireArtwork(card.querySelector(".poster"));collectionCards.clear();collectionCardsOrder=[];}
function renderCollectionMembers(){
  if(!collectionView||collectionKind==="series"||$("collection-panel").hidden)return;
  const target=$("collection-members"),visibleIds=visibleCollectionIds(),rect=target.getBoundingClientRect();
  if(document.body.classList.contains("player-open")||rect.width<=0)return;
  // Scroll keeps the same cards; changed membership/filter must renew action
  // closures and their move-button bounds, not retain an earlier visible order.
  if(visibleIds.length!==collectionCardsOrder.length||visibleIds.some((id,index)=>id!==collectionCardsOrder[index])){discardCollectionCards();collectionCardsOrder=visibleIds;}
  const view=catalogWindow(visibleIds.length,Math.max(1,rect.width),200,-rect.top,innerHeight);
  const focused=document.activeElement?.closest?.(".collection-media-card")?.dataset.mediaId;
  target.style.height=view.height+"px";
  const grid=node("div",undefined,"virtual-window"),wanted=new Set();grid.style.top=view.top+"px";grid.style.gridTemplateColumns="repeat("+view.columns+",minmax(0,1fr))";grid.style.gridAutoRows=view.gridRowHeight+"px";
  for(const id of visibleIds.slice(view.start,view.end)){wanted.add(id);let card=collectionCards.get(id);if(!card){card=collectionMemberCard(collectionDetails.get(id),visibleIds);collectionCards.set(id,card);}grid.append(card);}
  for(const [id,card] of collectionCards)if(!wanted.has(id)){retireArtwork(card.querySelector(".poster"));collectionCards.delete(id);}
  target.replaceChildren(grid);
  if(!visibleIds.length)target.append(node("p",collectionLoaded<collectionView.media_ids.length?uiText("Chargement des vidéos…"):uiText("Aucune vidéo. Utilisez + pour en ajouter ou modifiez le filtre."),"muted"));
  if(focused)[...grid.children].find(card=>card.dataset.mediaId===focused)?.focus({preventScroll:true});
}
function collectionMemberCard(m,visibleIds){
  const view=collectionView,kind=collectionKind,scope=collectionScope();
  const row=node("article",undefined,"media-card collection-media-card"),poster=node("div",m.title.slice(0,1).toUpperCase(),"poster"),copy=node("div",undefined,"info member-copy"),actions=node("div",undefined,"actions");row.dataset.mediaId=m.id;row.tabIndex=0;
  copy.append(button([m.season!=null?"S"+m.season:null,m.episode!=null?"E"+m.episode:null,m.title].filter(Boolean).join(" · "),()=>openMedia(m.id),"member-title"),node("p",[m.watched?uiText("✓ Vu"):uiText("Non vu"),!m.available?uiText("Indisponible"):null].filter(Boolean).join(" · "),"muted"));row.append(poster,copy);copy.append(actions);
  row.onclick=e=>{if(!e.target.closest("button,select"))openMedia(m.id);};row.onkeydown=e=>{if((e.key==="Enter"||e.key===" ")&&e.target===row){e.preventDefault();openMedia(m.id);}};
  const context={kind,id:view.id};const play=button(uiText("▶ Lire"),()=>{if(collectionView===view&&scope===collectionScope())window.chillPlay(m.id,context);});play.disabled=!m.available;actions.append(play);
  if(collectionKind!=="series"){
    const ids=collectionView.media_ids,index=ids.indexOf(m.id);
    for(const [text,delta] of [["↑",-1],["↓",1]]){const order=collectionKind==="playlist"?playlistOrderForVisibleMove(ids,visibleIds,m.id,delta):index+delta<0||index+delta>=ids.length?null:(()=>{const next=[...ids];[next[index],next[index+delta]]=[next[index+delta],next[index]];return next;})();const b=button(text,async()=>{
      if(collectionView!==view||scope!==collectionScope())return;if(order&&await run({action:kind==="playlist"?"reorder_playlist":"reorder_saga",id:view.id,expected_revision:view.revision,expected_order:ids,media_ids:order},false))await openCollection(view.id,kind);});b.disabled=!order;b.setAttribute("aria-label",(delta<0?uiText("Déplacer avant : "):uiText("Déplacer après : "))+m.title);actions.append(b);}
    actions.append(button(uiText("Retirer"),async()=>{if(collectionView!==view||scope!==collectionScope())return;if(await run({action:kind==="playlist"?"remove_playlist_member":"remove_saga_member",id:view.id,media_id:m.id,expected_revision:view.revision},false))await openCollection(view.id,kind);}));
    if(collectionKind==="saga"&&m.poster){const view=collectionView,server=state.server_id,profile=state.profile_id,label=m.poster_custom?uiText("Utiliser l’affiche personnelle du film"):m.tmdb_id?uiText("Utiliser l’affiche TMDB du film"):uiText("Utiliser l’affiche du film");actions.append(button(label,async()=>{if(collectionView!==view||scope!==collectionScope())return;const updated=await run(sagaMemberArtworkCommand(view.id,view.revision,m.id),false);if(updated&&collectionView===view&&server===state.server_id&&profile===state.profile_id)await openCollection(view.id,"saga");}));}
  }
  window.renderMediaProgress?.(poster,m);if(m.poster)observeArtwork(poster,m.id);return row;
}
function appendCollectionMember(m,visibleIds,target=$("collection-members")){target.append(collectionMemberCard(m,visibleIds));}
async function loadCollectionArtwork(view,kind){
  const preview=$("saga-artwork-preview"),server=state.server_id,profile=state.profile_id;
  if(!preview)return;
  const label=kind==="playlist"?uiText("liste"):"saga";
  preview.replaceChildren(node("span",view.poster?uiText("Chargement de l’affiche…"):uiText("Aucune image de ")+label+uiText(". La couverture est composée des neuf premières vidéos.")));
  if(!view.poster)return;
  const result=await libraryRead(collectionArtworkCommand(kind,view.id),()=>collectionView===view&&server===state.server_id&&profile===state.profile_id);
  if(collectionView!==view||server!==state.server_id||profile!==state.profile_id||!preview.isConnected)return;
  if(!result?.src)return;
  const image=node("img");image.src=result.src;image.alt=uiText("Image de la ")+label;image.decoding="async";preview.replaceChildren(image);
}
$("collection-rename").onsubmit=async e=>{e.preventDefault();if(!collectionView)return;if(await run({action:collectionKind==="playlist"?"rename_playlist":"rename_saga",id:collectionView.id,expected_revision:collectionView.revision,name:$("collection-name").value},false))await openCollection(collectionView.id,collectionKind);};
$("collection-delete").onclick=()=>{$("collection-delete-confirm").hidden=false;};
$("collection-delete-yes").onclick=async()=>{const cmd=collectionView&&{action:collectionKind==="playlist"?"delete_playlist":"delete_saga",id:collectionView.id};if(cmd&&collectionKind==="saga")cmd.expected_revision=collectionView.revision;if(cmd&&await run(cmd,false)){collectionView=null;$("collection-delete-confirm").hidden=true;await openCollections(collectionKind);}};
$("collection-delete-no").onclick=()=>{$("collection-delete-confirm").hidden=true;};
let pickerSelection=new Set(),pickerItems=[],pickerGeneration=0,pickerSearchTimer=null;
async function pickerPage(){
 const view=collectionView,epoch=++pickerGeneration,context=collectionScope();
 if(!view)return;
 pickerItems=[];$("picker-items").replaceChildren(node("p",uiText("Chargement des vidéos…"),"muted"));$("picker-items").scrollTop=0;
 const filters={category:collectionKind==="saga"?"film":$("picker-category").value,q:$("picker-search").value,watched:$("picker-watched").value,genre:"",year:"",sort:"",available:""},ids=new Set();
 const current=()=>epoch===pickerGeneration&&view===collectionView&&context===collectionScope()&&shellSection==="picker";
 for(let at=0;current();){
  const page=await libraryRead(catalogCommand(filters,at,false),current);if(!page)return;
  if(!Array.isArray(page.items)||page.items.length>CATALOG_PAGE_SIZE||page.has_more&&!page.items.length){message(uiText("Sélecteur catalogue incohérent."));return;}
  let added=0;
  for(const media of page.items){if(!ids.has(media.id)){ids.add(media.id);added++;if(!view.media_ids.includes(media.id))pickerItems.push(media);}}
  renderPicker();
  if(!page.has_more)break;if(!added||ids.size>=30000){message(uiText("Catalogue trop volumineux ou modifié. Relancez la recherche."));return;}at+=page.items.length;
 }
}
function renderPicker(){
 const container=$("picker-items");if($("picker-panel").hidden)return;
 const rect=container.getBoundingClientRect(),view=catalogWindow(pickerItems.length,Math.max(1,rect.width),176,container.scrollTop,container.clientHeight);
 const list=node("div",undefined,"picker-virtual");list.style.height=view.height+"px";
 const grid=node("div",undefined,"picker-window");grid.style.top=view.top+"px";grid.style.gridTemplateColumns="repeat("+view.columns+",minmax(0,1fr))";
 for(let index=view.start;index<view.end;index++){
  const media=pickerItems[index],subtitle=[({film:uiText("Film"),serie:uiText("Série"),documentaire:uiText("Documentaire"),spectacle:uiText("Spectacle")})[media.category],media.year,media.season!=null?"S"+media.season:null,media.episode!=null?"E"+media.episode:null].filter(Boolean).join(" · ");
  const {tile,poster}=posterTile(media.title,subtitle,()=>{
    if(pickerSelection.has(media.id))pickerSelection.delete(media.id);else if(pickerSelection.size<1000)pickerSelection.add(media.id);
    tile.classList.toggle("selected",pickerSelection.has(media.id));tile.setAttribute("aria-pressed",String(pickerSelection.has(media.id)));pickerCount();
  });
  tile.dataset.mediaId=media.id;tile.classList.toggle("selected",pickerSelection.has(media.id));tile.setAttribute("aria-pressed",String(pickerSelection.has(media.id)));grid.append(tile);
  window.renderMediaProgress?.(poster,media);
  if(media.poster)observeArtwork(poster,media.id);
 }
 list.append(grid);container.replaceChildren(list);
 if(!pickerItems.length)container.append(node("p",uiText("Aucune vidéo à ajouter pour ces filtres."),"muted"));pickerCount();
}
function pickerCount(){$("picker-selected").textContent=pickerSelection.size+uiText(" sélectionnée(s)")+(pickerSelection.size>=1000?uiText(" — maximum atteint"):"");$("picker-confirm").disabled=!pickerSelection.size;}
function closePicker(){pickerGeneration++;clearTimeout(pickerSearchTimer);pickerSelection.clear();showCollectionPage("collection");renderCollectionMembers();}
$("picker-items").addEventListener("scroll",renderPicker,{passive:true});
$("collection-add").onclick=async()=>{
 if(!collectionView||collectionKind==="series")return;
 pickerSelection=new Set();$("picker-search").value="";$("picker-category").value=collectionKind==="saga"?"film":"";$("picker-category").disabled=collectionKind==="saga";$("picker-watched").value="";
 $("picker-hint").textContent=collectionKind==="saga"?uiText("Une saga regroupe des films. Sélectionnez leurs vignettes."):uiText("Films, épisodes de séries, documentaires et spectacles : sélectionnez leurs vignettes.");
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
let artworkQueue=[],artworkLoading=false,artworkGeneration=0;
function invalidateArtwork(target){target.dataset.artworkToken=String(++artworkGeneration);delete target.dataset.loaded;delete target.dataset.queued;artworkQueue=artworkQueue.filter(item=>item.target!==target);}
function retireArtwork(target){
 if(!target)return;
 artworkObserver.unobserve(target);artworkTargets.delete(target);invalidateArtwork(target);
 delete target.dataset.mediaId;delete target.dataset.artworkKind;delete target.dataset.initial;
}
function artworkCurrent(item){return item.target.isConnected&&item.context===collectionScope()&&item.token===item.target.dataset.artworkToken;}
const artworkTargets=new Set();
function discardDetachedArtwork(){for(const target of artworkTargets){if(!target.isConnected){artworkObserver.unobserve(target);artworkTargets.delete(target);}}artworkQueue=artworkQueue.filter(item=>item.target.isConnected);}
window.resetArtwork=()=>{for(const target of [...artworkTargets])if(!target.isConnected||$("media").contains(target))retireArtwork(target);artworkQueue=artworkQueue.filter(item=>item.target.isConnected&&!$("media").contains(item.target));};
const artworkObserver=new IntersectionObserver(entries=>{
  // The callback runs after a whole virtual grid has been inserted. Pruning in
  // observeArtwork would unobserve earlier siblings still in its detached grid.
  discardDetachedArtwork();
 for(const e of entries){if(!artworkTargets.has(e.target)||!e.target.isConnected)continue;if(e.isIntersecting){if(!e.target.dataset.loaded&&!e.target.dataset.queued){e.target.dataset.queued="true";artworkQueue.push({target:e.target,id:e.target.dataset.mediaId,kind:e.target.dataset.artworkKind,token:e.target.dataset.artworkToken,context:collectionScope()});}}}
  drainArtwork();
},{rootMargin:"100px"});
function observeArtwork(target,id,kind="media"){invalidateArtwork(target);target.dataset.mediaId=id;target.dataset.artworkKind=kind;target.dataset.initial=target.textContent;artworkTargets.add(target);artworkObserver.unobserve(target);artworkObserver.observe(target);}
async function drainArtwork(){
  if(artworkLoading)return;artworkLoading=true;
  while(artworkQueue.length){
    const item=artworkQueue.shift();
    if(!artworkCurrent(item)||item.target.dataset.loaded){if(item.token===item.target.dataset.artworkToken)delete item.target.dataset.queued;continue;}
    if(busy||playerBusy){artworkQueue.unshift(item);await new Promise(r=>setTimeout(r,300));continue;}
    try{
      const r=await invoke(item.kind==="media"?{action:"artwork",media_id:item.id}:item.kind==="season"?{action:"season_artwork",tv_id:Number(item.id.split(":")[0]),season:Number(item.id.split(":")[1])}:collectionArtworkCommand(item.kind,item.id));
      if(!artworkCurrent(item))continue;
      const progress=item.target.querySelector?.(".media-progress"),img=node("img");img.src=r.src;img.alt="";img.decoding="async";item.target.replaceChildren(img);if(progress)item.target.append(progress);item.target.dataset.loaded="true";
    }catch(e){if(e.code==="CLIENT_BUSY"){artworkQueue.unshift(item);await new Promise(r=>setTimeout(r,300));}}
    finally{if(!artworkQueue.includes(item)&&item.token===item.target.dataset.artworkToken)delete item.target.dataset.queued;}
  }
  artworkLoading=false;
}
window.chillArtwork=observeArtwork;window.retireArtwork=retireArtwork;
