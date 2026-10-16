"use strict";
var uiText=globalThis.chillI18n?.t||((text)=>text);

(function(root){
 function normalizeGenres(items){
  return [...new Set(items.flatMap(item=>Array.isArray(item.genres)?item.genres:[]).map(value=>String(value).trim()).filter(Boolean))].sort((a,b)=>a.localeCompare(b,"fr"));
 }
 function availableYears(items){
  return [...new Set(items.map(item=>Number(item.year)).filter(year=>Number.isInteger(year)&&year>0))].sort((a,b)=>b-a);
 }
 function yearMatches(year,selection){
  if(!selection.length)return true;
  if(!Number.isInteger(Number(year))||Number(year)<=0)return false;
  if(selection.length===1)return Number(year)===selection[0];
  // Two selected endpoints form an inclusive interval.
  return Number(year)>=Math.min(...selection)&&Number(year)<=Math.max(...selection);
 }
 function matches(item,genres,years){
  const itemGenres=Array.isArray(item.genres)?item.genres:[];
  // Genres are alternatives (OR); the genre and year groups still combine with AND.
  return (!genres.length||itemGenres.some(genre=>genres.includes(genre)))&&yearMatches(item.year,years);
 }
 function describe(genres,years){
  const parts=[];
  if(genres.length)parts.push(genres.length===1?genres[0]:genres.length+uiText(" genres"));
  if(years.length===1)parts.push(String(years[0]));
  if(years.length===2)parts.push(Math.min(...years)+"–"+Math.max(...years));
  return parts;
 }
 function create(){
  let selectedGenres=[],selectedYears=[],onChange=()=>{},rootNode=null,open=null;
  const get=id=>document.getElementById(id);
  const values=()=>({genres:[...selectedGenres],years:[...selectedYears]});
  function close(){
   if(!open)return;
   const button=get("facet-"+open),pop=get("facet-"+open+"-pop");
   if(button)button.setAttribute("aria-expanded","false");
   if(pop)pop.hidden=true;
   open=null;
  }
  function updateButton(){
   const genreButton=get("facet-genres"),yearButton=get("facet-years"),reset=get("facet-reset"),summary=get("facet-selected");
   if(!genreButton)return;
   const genres=get("facet-genre-count"),years=get("facet-year-count"),parts=describe(selectedGenres,selectedYears);
   genres.textContent=selectedGenres.length?" "+selectedGenres.length:"";
   years.textContent=selectedYears.length?" "+(selectedYears.length===2?Math.min(...selectedYears)+"–"+Math.max(...selectedYears):selectedYears[0]):"";
   genreButton.classList.toggle("facet-active",Boolean(selectedGenres.length));
   yearButton.classList.toggle("facet-active",Boolean(selectedYears.length));
   reset.hidden=!parts.length;
   summary.replaceChildren(...parts.map(text=>{
    const tag=document.createElement("span");
    tag.className="facet-tag";
    tag.textContent=text;
    return tag;
   }));
  }
  function draw(items,focusChoice){
   if(!rootNode)return;
   const genreList=get("facet-genre-list"),yearList=get("facet-year-list");
   if(!genreList||!yearList)return;
   const active=document.activeElement;
   // Replacing the lists would otherwise discard keyboard focus after a selection.
   if(!focusChoice&&active){
    if(genreList.contains(active))focusChoice="genre:"+active.textContent;
    else if(yearList.contains(active))focusChoice="year:"+active.textContent;
   }
   const genres=normalizeGenres(items),years=availableYears(items);
   genreList.replaceChildren(...genres.map(genre=>{
    const button=document.createElement("button");
    button.type="button";
    button.className="facet-chip";
    button.textContent=genre;
    button.setAttribute("aria-pressed",String(selectedGenres.includes(genre)));
    button.onclick=()=>{
     selectedGenres=selectedGenres.includes(genre)?selectedGenres.filter(value=>value!==genre):[...selectedGenres,genre];
     updateButton();draw(items,"genre:"+genre);onChange(values());
    };
    return button;
   }));
   // A third choice starts a new interval instead of creating an ambiguous range.
   yearList.replaceChildren(...years.map(year=>{
    const button=document.createElement("button");
    button.type="button";
    button.className="facet-year";
    button.textContent=String(year);
    button.setAttribute("aria-pressed",String(selectedYears.includes(year)));
    if(selectedYears.length===2&&year>Math.min(...selectedYears)&&year<Math.max(...selectedYears))button.classList.add("facet-year-range");
    button.onclick=()=>{
     selectedYears=selectedYears.includes(year)?selectedYears.filter(value=>value!==year):(selectedYears.length>=2?[year]:[...selectedYears,year]);
     updateButton();draw(items,"year:"+year);onChange(values());
    };
    return button;
   }));
   const focused=focusChoice?.split(":");
   if(focused){
    const list=focused[0]==="genre"?genreList:yearList,choice=focused[1];
    [...list.children].find(button=>button.textContent===choice)?.focus();
   }
   get("facet-genre-empty").hidden=genres.length>0;
   get("facet-year-empty").hidden=years.length>0;
   updateButton();
  }
  function init(options={}){
   rootNode=get("facet-controls");
   if(!rootNode||rootNode.dataset.ready)return;
   rootNode.dataset.ready="true";
   onChange=options.onChange||onChange;
   for(const name of ["genres","years"]){
    const button=get("facet-"+name),pop=get("facet-"+name+"-pop");
    button.onclick=()=>{
     const next=open===name?null:name;
     close();
     if(next){open=next;button.setAttribute("aria-expanded","true");pop.hidden=false;}
    };
   }
   get("facet-reset").onclick=()=>{
    selectedGenres=[];selectedYears=[];updateButton();draw(options.items||[]);onChange(values());
   };
   document.addEventListener("keydown",event=>{
    if(event.key==="Escape"&&open){event.preventDefault();const name=open;close();get("facet-"+name)?.focus();}
   });
   document.addEventListener("pointerdown",event=>{if(open&&!rootNode.contains(event.target))close();});
   updateButton();draw(options.items||[]);
  }
  return {
   init,draw,values,
   filter:items=>items.filter(item=>matches(item,selectedGenres,selectedYears)),
   clear:()=>{selectedGenres=[];selectedYears=[];updateButton();}
  };
 }
 const api={normalizeGenres,availableYears,yearMatches,matches,describe,create};
 if(typeof module!=="undefined")module.exports=api;
 if(typeof document!=="undefined")root.catalogFacets=create();
})(typeof window!=="undefined"?window:globalThis);
