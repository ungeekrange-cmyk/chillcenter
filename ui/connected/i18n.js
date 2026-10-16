"use strict";
(() => {
 const catalogue=globalThis.chillEnglish||{},supported=["auto","en","fr"];
 let preference="auto",language="en";
 function resolve(choice,system){return choice==="en"||choice==="fr"?choice:/^fr(?:[-_]|$)/i.test(system||"")?"fr":"en";}
 function text(source,...values){const translated=language==="en"&&Object.hasOwn(catalogue,source)?catalogue[source]:source;return typeof translated==="string"?translated.replace(/\{(\d+)\}/g,(match,index)=>index<values.length?String(values[index]):match):translated;}
 const api={t:text,resolve,get language(){return language;},get preference(){return preference;}};
 globalThis.chillI18n=api;
 if(typeof module!=="undefined")module.exports=api;
 if(typeof document==="undefined")return;
 // Snapshot only the original static HTML, before application modules insert
 // metadata. Never observe/translate arbitrary DOM text from users or TMDB.
 const staticText=[],attributes=[];
 const walker=document.createTreeWalker(document.body,NodeFilter.SHOW_TEXT);
 while(walker.nextNode()){
  const n=walker.currentNode;
  if(n.parentElement.closest("script,style,code,pre"))continue;
  const source=n.nodeValue.trim();if(Object.hasOwn(catalogue,source))staticText.push({node:n,source,leading:n.nodeValue.match(/^\s*/)[0],trailing:n.nodeValue.match(/\s*$/)[0]});
 }
 for(const element of document.querySelectorAll("[title],[placeholder],[aria-label]"))for(const name of ["title","placeholder","aria-label"]){const source=element.getAttribute(name);if(Object.hasOwn(catalogue,source))attributes.push({element,name,source});}
 api.set=choice=>{
  preference=supported.includes(choice)?choice:"auto";
  language=resolve(preference,navigator.language||navigator.languages?.[0]);
  document.documentElement.lang=language;
  for(const item of staticText)if(item.node.isConnected)item.node.nodeValue=item.leading+text(item.source)+item.trailing;
  for(const item of attributes)if(item.element.isConnected)item.element.setAttribute(item.name,text(item.source));
  const select=document.getElementById("interface-language");if(select)select.value=preference;
 };
 window.addEventListener("languagechange",()=>{if(preference==="auto"&&!document.body.classList.contains("player-open"))window.location.reload();});
 const select=document.getElementById("interface-language");
 if(select)select.onchange=async()=>{
  const previous=preference;select.disabled=true;
  try{
   await window.__TAURI__.core.invoke("ui_command",{command:{action:"save_language",language:select.value}});
   // Recreate UI literals consistently. The native player is already stopped
   // when entering Settings; this never restarts the app, server or catalogue.
   localStorage.setItem("chillcenter-interface-language",select.value);
   sessionStorage.setItem("chillcenter-language-return","settings");
   window.location.reload();
  }catch(error){select.value=previous;document.getElementById("message").textContent=text(error.message||"Opération échouée.");}
  finally{select.disabled=false;}
 };
 // Rust settings are authoritative. This small hint only selects the correct
 // language before the deferred modules construct their labels on next boot.
 api.hydrate=choice=>{
  if(localStorage.getItem("chillcenter-interface-language")!==choice)localStorage.setItem("chillcenter-interface-language",choice);
  if(resolve(choice,navigator.language||navigator.languages?.[0])!==language){if(!document.body.classList.contains("player-open")){window.location.reload();return true;}return false;}
  api.set(choice);return false;
 };
 api.set(localStorage.getItem("chillcenter-interface-language")||"auto");
})();
