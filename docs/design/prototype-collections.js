"use strict";
// Interactive design fixtures only; no server, media file, persistence or AI.
(() => {
 const make=(tag,text,cls)=>{const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(cls)node.className=cls;return node;};
 const button=(text,action,cls="secondary compact")=>{const node=make("button",text,cls);node.type="button";node.onclick=action;return node;};
 const profile=()=>$("profile").querySelector(".profile-name").textContent;
 data.forEach((media,index)=>media.id="demo-"+index);
 const series=data.find(media=>media.category==="serie");
 const episodes=[
  [1,1,true,true],[1,2,false,true],[1,3,true,true],[2,1,false,true],[2,2,false,false]
 ].map(([season,episode,watched,available])=>({id:`episode-${season}-${episode}`,title:`The Expanse — S${season}E${episode}`,label:`ÉPISODE ${episode}`,category:"episode",genre:"Science-fiction",year:2015,art:"space",series,season,episode,watched,available}));
 const videos=[...data.filter(media=>media!==series),...episodes];
 const collections=[
  {id:"saga-dune",kind:"saga",name:"Dune",members:[data[0].id,data[1].id]},
  {id:"playlist-evening",kind:"playlist",name:"Soirée science-fiction",owner:"Profil 1",members:[data[0].id,data[4].id]}
 ];
 const watchedByProfile=new Map([["Profil 1",new Set([...data,...episodes].filter(media=>media.watched).map(media=>media.id))],["Profil 2",new Set()]]);
 let selectedCollection=null,editorTarget=null,sequence=0;
 const plural=(n,one,many=one+"s")=>n+" "+(n===1?one:many);
 const stats=items=>plural(items.length,"épisode")+" · "+plural(items.filter(media=>!media.watched).length,"épisode non vu","épisodes non vus");
 const allStats=()=>plural(new Set(episodes.map(media=>media.season)).size,"saison")+" · "+stats(episodes);
 const accessibleCollections=()=>collections.filter(item=>item.kind===collection&&(item.kind==="saga"||item.owner===profile()));
 window.prototypeCardEnhance=(card,media)=>{
  if(media===series)card.append(make("p",allStats(),"series-stats"));
 };
 window.prototypeDetailEnhance=media=>{
  $("detail-unavailable").hidden=media.available!==false;
  $("detail-kind").textContent=media===series?"FICHE DE LA SÉRIE":media.category==="episode"?"FICHE DE L’ÉPISODE":"FICHE DU FILM";
  $("detail-watched").hidden=media===series;
  $("detail-watched").textContent=media.watched?"✓ Vu":"Non vu";
  $("detail-watched").onclick=()=>{
   media.watched=!media.watched;
   const seen=watchedByProfile.get(profile());if(media.watched)seen.add(media.id);else seen.delete(media.id);
   window.prototypeDetailEnhance(media);render();
  };
  $("detail-play").disabled=media.available===false;
  $("detail-play").textContent=media===series?"▶ Reprendre la série":"▶ Lire";
  $("detail-play").onclick=()=>{
   if(media===series){const next=episodes.find(item=>!item.watched&&item.available!==false)||episodes.find(item=>item.available!==false);if(!next)return;active=next;}
   else active=media;
   if(active.available!==false)start();
  };
  const explorer=$("series-explorer");explorer.hidden=media!==series&&media.category!=="episode";explorer.replaceChildren();
  if(media===series){renderSeasons();}
  else if(media.category==="episode")explorer.append(button("‹ Revenir à la saison "+media.season,()=>{openDetail(series);renderSeason(media.season);}));
 };
 function renderSeasons(){
  const explorer=$("series-explorer");explorer.replaceChildren(make("p",allStats(),"series-summary"));
  explorer.append(make("p","Données de maquette : exemple de deux saisons et cinq épisodes, pas le nombre réel d’épisodes de la série.","preview-note"));
  const seasons=make("div",undefined,"season-list");
  for(const number of new Set(episodes.map(media=>media.season))){
   const items=episodes.filter(media=>media.season===number),open=button("",()=>renderSeason(number),"season-button");open.setAttribute("aria-label","Ouvrir la saison "+number);open.append(make("strong","Saison "+number),make("small",stats(items)));seasons.append(open);
  }
  explorer.append(seasons);
 }
 function renderSeason(number){
  const explorer=$("series-explorer"),items=episodes.filter(media=>media.season===number);
  explorer.replaceChildren(button("‹ Toutes les saisons",renderSeasons,"secondary compact season-back"),make("p","Saison "+number+" · "+stats(items),"series-summary"));
  for(const episode of items){
   const row=make("article",undefined,"episode-row"),copy=make("div",undefined,"episode-copy");
   copy.append(button("Épisode "+episode.episode,()=>openDetail(episode),"episode-title"),make("p",episode.watched?"✓ Vu":"Non vu"));
   row.append(copy);
   if(episode.available===false)row.append(make("span","Indisponible","tag unavailable"));
   const play=button("▶ Lire",()=>{active=episode;start();},"primary compact");play.disabled=episode.available===false;row.append(play);explorer.append(row);
  }
 }
 window.prototypeCollectionReset=()=>{selectedCollection=null;};
 window.prototypeCollectionRender=()=>{
  document.querySelector("#catalogue-view .toolbar").hidden=true;
  $("grid").classList.remove("list");$("grid").classList.add("collection-tiles");$("grid").replaceChildren();$("collection-toolbar")?.remove();
  const toolbar=make("div",undefined,"actions");toolbar.id="collection-toolbar";$("grid").before(toolbar);
  document.querySelectorAll("[data-collection]").forEach(node=>node.classList.toggle("active",node.dataset.collection===collection));
  const selected=accessibleCollections().find(item=>item.id===selectedCollection);
  $("page-title").textContent=selected?selected.name:collection==="saga"?"Sagas":"Mes listes de lecture";
  if(selected){
   $("count").textContent=plural(selected.members.length,"vidéo")+" · "+(collection==="saga"?"Saga commune au foyer":"Liste de "+profile());
   toolbar.append(button("‹ "+(collection==="saga"?"Toutes les sagas":"Mes listes de lecture"),()=>{selectedCollection=null;render();}),button("＋ Ajouter des vidéos",()=>openEditor(selected),"primary compact"));
   $("grid").classList.remove("collection-tiles");$("grid").classList.add("collection-members");
   for(const id of selected.members){
    const media=videos.find(item=>item.id===id),row=make("article",undefined,"member-row"),copy=make("div",undefined,"member-info");
    copy.append(button(media.title,()=>openDetail(media),"media-title"),make("small",media.watched?"✓ Vu":"Non vu"));row.append(copy);
    if(media.available===false)row.append(make("span","Indisponible","tag unavailable"));
    const play=button("▶ Lire",()=>{active=media;start();},"primary compact");play.disabled=media.available===false;row.append(play);$("grid").append(row);
   }
   if(!selected.members.length)$("grid").append(make("p","Cette collection est vide. Ajoute les vidéos que tu veux y retrouver.","empty"));
  }else{
   $("grid").classList.remove("collection-members");
   const items=accessibleCollections();$("count").textContent=plural(items.length,collection==="saga"?"saga":"liste")+" · "+(collection==="saga"?"Communes au foyer":"Personnelles à "+profile());
   toolbar.append(button(collection==="saga"?"＋ Créer une saga":"＋ Créer une liste de lecture",()=>openEditor(null),"primary compact"));
   for(const item of items){const tile=button("",()=>{selectedCollection=item.id;render();},"collection-tile");tile.setAttribute("aria-label","Ouvrir "+item.name);tile.append(make("strong",item.name),make("p",plural(item.members.length,"vidéo")));$("grid").append(tile);}
   if(!items.length)$("grid").append(make("p","Aucune collection pour le moment. Crée la première.","empty"));
  }
  toolbar.append(make("p","Maquette : créations et ajouts temporaires en mémoire, aucun changement du serveur.","collection-disclaimer"));
 };
 const editor=make("dialog");editor.id="collection-editor";editor.className="collection-editor";
 const title=make("h2"),form=make("form"),label=make("label","Nom","field"),name=make("input"),picker=make("fieldset",undefined,"video-picker"),confirm=make("button","Créer","primary"),cancel=button("Annuler",()=>editor.close()),feedback=make("p","","success");
 name.id="collection-name";name.required=true;name.maxLength=160;label.append(name);picker.append(make("legend","Vidéos à ajouter"));
 const actions=make("div",undefined,"actions");actions.append(confirm,cancel);form.append(label,picker,feedback,actions);editor.append(title,form);document.body.append(editor);
 function openEditor(target){
  editorTarget=target;title.textContent=target?"Ajouter des vidéos":collection==="saga"?"Créer une saga":"Créer une liste de lecture";confirm.textContent=target?"Ajouter les vidéos":"Créer";label.hidden=!!target;name.required=!target;name.value="";feedback.textContent="";picker.replaceChildren(make("legend","Vidéos à ajouter"));
  for(const media of videos.filter(item=>(collection!=="saga"||item.category==="film")&&!target?.members.includes(item.id))){const row=make("label"),checkbox=make("input");checkbox.type="checkbox";checkbox.value=media.id;row.append(checkbox,make("span",media.title));picker.append(row);}
  editor.showModal();
 }
 form.onsubmit=event=>{
  event.preventDefault();const members=[...picker.querySelectorAll("input:checked")].map(input=>input.value);
  if(!editorTarget){const value=name.value.trim();if(!value){feedback.textContent="Indique un nom.";name.focus();return;}
   if(accessibleCollections().some(item=>item.name.toLocaleLowerCase("fr")===value.toLocaleLowerCase("fr"))){feedback.textContent="Ce nom existe déjà.";return;}
   editorTarget={id:"created-"+(++sequence),kind:collection,name:value,owner:collection==="playlist"?profile():undefined,members:[]};collections.push(editorTarget);
  }
  editorTarget.members.push(...members.filter(id=>!editorTarget.members.includes(id)));selectedCollection=editorTarget.id;editor.close();render();
 };
 const previousProfile=$("profile").onclick;$("profile").onclick=()=>{
  previousProfile();const seen=watchedByProfile.get(profile());for(const media of [...data,...episodes])media.watched=seen.has(media.id);
  $("detail").close();editor.close();selectedCollection=null;$("advice-preview").hidden=true;$("advice-results").replaceChildren();render();
 };
 const previousRender=render;render=function(){
  if(!collection){$("grid").classList.remove("collection-tiles","collection-members");document.querySelectorAll("[data-collection]").forEach(node=>node.classList.remove("active"));}
  previousRender();
 };
 render();
})();
