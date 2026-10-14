"use strict";
// Catalogue and personal collections share the allowlisted Rust bridge. No URLs or
// filesystem paths from media metadata are evaluated by this interface.
let catalogProfile=null,collectionView=null,collectionKind="playlist",detailView=null;
function closeCatalogDialogs(){for(const id of ["media-dialog","collections-dialog","collection-dialog","picker-dialog"])if($(id).open)$(id).close();collectionView=null;detailView=null;}
window.catalogProfileChanged=()=>{if(catalogProfile!==state.profile_id){closeCatalogDialogs();if(typeof hidePlayer==="function")hidePlayer();catalogProfile=state.profile_id;}};
function button(text,fn,cls="secondary"){const b=node("button",text,cls);b.type="button";b.onclick=fn;return b;}
function options(select,values){select.replaceChildren();for(const [value,label] of values){const o=node("option",label);o.value=value;select.append(o);}}
window.catalogEnhanceCard=(card,m)=>{
  const actions=card.querySelector(".actions");actions.append(button("Fiche",()=>openMedia(m.id)),button("Ajouter à une playlist",()=>pickPlaylist(m.id)));
  if(m.category==="serie")actions.append(button("Voir la série",()=>openSeries(m.series||m.title)));
  if(m.poster)observeArtwork(card.querySelector(".poster"),m.id);
};
async function openMedia(id){
  const result=await run({action:"media_detail",media_id:id},false);if(!result)return;detailView=result;
  const m=result.media;$("detail-title").textContent=m.title;$("detail-overview").textContent=m.overview||"Informations à compléter";
  $("detail-metadata").textContent=[m.year,m.runtime_minutes?m.runtime_minutes+" min":null,...m.genres,...m.directors.map(n=>"Réalisation : "+n),...m.cast].filter(Boolean).join(" · ");
  $("detail-availability").textContent=m.available?"Disponible":"Vidéo indisponible";
  $("detail-play").disabled=!m.available;$("detail-play").onclick=()=>{$("media-dialog").close();window.chillPlay(m.id);};
  $("detail-edit").hidden=false;$("metadata-form").hidden=true;
  options($("detail-rating-scope"),m.category==="serie"?[["video","Cet épisode"],["season","Cette saison"],["series","Toute la série"]]:[["video","Cette vidéo"]]);
  $("detail-rating").value=m.rating?String(m.rating):"";$("detail-rating-scope").value="video";$("media-dialog").showModal();
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
  collectionKind="series";collectionView={id:name,name,media_ids:result.episodes.map(m=>m.id),episodes:result.episodes};
  $("collection-title").textContent=name;$("collection-counts").textContent=result.counters.watched+" vus · "+result.counters.unwatched+" non vus";
  $("collection-admin").hidden=true;$("collection-add").hidden=true;$("collection-members").replaceChildren();
  let season=undefined;for(const m of result.episodes){if(m.season!==season){season=m.season;$("collection-members").append(node("h3",season===null?"Saison inconnue":"Saison "+season));}appendCollectionMember(m);}
  $("collection-dialog").showModal();
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
  $("collection-title").textContent=collectionView.name;$("collection-counts").textContent=collectionView.media_ids.length+" vidéos";
  $("collection-name").value=collectionView.name;$("collection-admin").hidden=false;$("collection-add").hidden=false;$("collection-members").replaceChildren();
  // Fetch fixed pages of one item at a time, bounded by the collection limit; never
  // load media files or make the webview a direct server HTTP client.
  await collectionPage(id,0);
  $("collection-dialog").showModal();
}
async function collectionPage(id,start){
  const view=collectionView,kind=collectionKind;
  if(!view||view.id!==id)return;
  const ids=view.media_ids.slice(start,start+50);
  for(const media_id of ids){const d=await run({action:"media_detail",media_id},false);if(collectionView!==view||kind!==collectionKind)return;if(d)appendCollectionMember(d.media);}
  if(start+50<view.media_ids.length){const more=button("Afficher les 50 vidéos suivantes",async()=>{more.remove();await collectionPage(id,start+50);});$("collection-members").append(more);}
}
function appendCollectionMember(m){
  const row=node("article",undefined,"collection-row");row.append(node("span",[m.season!==null?"S"+m.season:null,m.episode!==null?"E"+m.episode:null,m.title,m.watched?"Vu ✓":null,!m.available?"Indisponible":null].filter(Boolean).join(" · ")));
  const context={kind:collectionKind,id:collectionView.id};const play=button("Lecture",()=>{$("collection-dialog").close();window.chillPlay(m.id,context);});play.disabled=!m.available;row.append(play);
  if(collectionKind!=="series"){
    const ids=collectionView.media_ids,index=ids.indexOf(m.id);
    for(const [text,delta] of [["↑",-1],["↓",1]]){const b=button(text,async()=>{const order=[...ids],to=index+delta;[order[index],order[to]]=[order[to],order[index]];
      if(await run({action:collectionKind==="playlist"?"reorder_playlist":"reorder_saga",id:collectionView.id,expected_revision:collectionView.revision,expected_order:ids,media_ids:order},false))await openCollection(collectionView.id,collectionKind);});b.disabled=index+delta<0||index+delta>=ids.length;row.append(b);}
    row.append(button("Retirer",async()=>{if(await run({action:collectionKind==="playlist"?"remove_playlist_member":"remove_saga_member",id:collectionView.id,media_id:m.id,expected_revision:collectionView.revision},false))await openCollection(collectionView.id,collectionKind);}));
  }
  $("collection-members").append(row);
}
$("collection-rename").onsubmit=async e=>{e.preventDefault();if(!collectionView)return;if(await run({action:collectionKind==="playlist"?"rename_playlist":"rename_saga",id:collectionView.id,expected_revision:collectionView.revision,name:$("collection-name").value},false))await openCollection(collectionView.id,collectionKind);};
$("collection-delete").onclick=()=>{$("collection-delete-confirm").hidden=false;};
$("collection-delete-yes").onclick=async()=>{const cmd=collectionView&&{action:collectionKind==="playlist"?"delete_playlist":"delete_saga",id:collectionView.id};if(cmd&&collectionKind==="saga")cmd.expected_revision=collectionView.revision;if(cmd&&await run(cmd,false)){$("collection-dialog").close();collectionView=null;$("collection-delete-confirm").hidden=true;await openCollections(collectionKind);}};
$("collection-delete-no").onclick=()=>{$("collection-delete-confirm").hidden=true;};
let pickerOffset=0,pickerSelection=new Set();
async function pickerPage(){const page=await run({action:"catalog",offset:pickerOffset,category:null,q:$("picker-search").value||null},false);if(!page)return;$("picker-items").replaceChildren();
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
