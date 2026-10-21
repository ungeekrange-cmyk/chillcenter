"use strict";
// Declarative theme packs: CSS/effects stay local; this accepts only named tokens and local PNG assets.
(() => {
 const FORMAT=1, RENDERER=2, HEX=/^#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?$/, PATH=/^assets\/(?:logos|backgrounds|decorations)\/[a-z0-9-]+\.png$/;
 const TOKENS=new Set(["bg","panel","raised","text","muted","accent","button","sidebar","line"]),THEME_IDS=new Set(["spring","summer","autumn","winter","christmas","halloween","easter","valentine"]),STATIC_IDS=new Set(["spring","summer","autumn","winter","christmas","halloween"]);
 const isObject=value=>value!==null&&typeof value==="object"&&!Array.isArray(value);
 const validDate=parts=>{const [month,day]=parts;return parts.length===2&&Number.isInteger(month)&&Number.isInteger(day)&&month>=1&&month<=12&&day>=1&&day<=new Date(2024,month,0).getDate();};
 const ordinal=([month,day])=>month*32+day;
 const inRange=(date,start,end)=>{const point=(date.getMonth()+1)*32+date.getDate(),a=ordinal(start),b=ordinal(end);return a<=b?point>=a&&point<=b:point>=a||point<=b;};
 const westernEaster=(year)=>{const a=year%19,b=Math.floor(year/100),c=year%100,d=Math.floor(b/4),e=b%4,f=Math.floor((b+8)/25),g=Math.floor((b-f+1)/3),h=(19*a+b-d-g+15)%30,i=Math.floor(c/4),k=c%4,l=(32+2*e+2*i-h-k)%7,m=Math.floor((a+11*h+22*l)/451),month=Math.floor((h+l-7*m+114)/31),day=(h+l-7*m+114)%31+1;return new Date(year,month-1,day);};
 const eventRange=(date,calendar)=>{const year=date.getFullYear(),end=calendar==="western_easter"?westernEaster(year):new Date(year,1,14),start=new Date(end);start.setDate(end.getDate()-(calendar==="western_easter"?30:14));end.setHours(23,59,59,999);return date>=start&&date<=end;};
 const validPalette=palette=>isObject(palette)&&Object.entries(palette).every(([key,value])=>TOKENS.has(key)&&typeof value==="string"&&HEX.test(value));
 let nativeBlobUrls=[];
 const revokeNativeBlobs=()=>{for(const url of nativeBlobUrls)URL.revokeObjectURL(url);nativeBlobUrls=[];};
 const nativeBlobAssets=(manifest,assets)=>{
  if(!isObject(assets))throw Error("Invalid native theme assets");
  const urls={},created=[];
  try{for(const [path,receipt] of Object.entries(manifest.assets)){const value=assets[path];if(typeof value!=="string"||!/^data:image\/png;base64,[a-z0-9+/=]+$/i.test(value))throw Error("Invalid native theme asset");const bytes=atob(value.slice(value.indexOf(",")+1));if(bytes.length!==receipt.size)throw Error("Invalid native theme asset size");const raw=new Uint8Array(bytes.length);for(let index=0;index<bytes.length;index++)raw[index]=bytes.charCodeAt(index);const url=URL.createObjectURL(new Blob([raw],{type:"image/png"}));created.push(url);urls[path]=url;}}catch(error){for(const url of created)URL.revokeObjectURL(url);throw error;}
  revokeNativeBlobs();nativeBlobUrls=created;return urls;
 };
 const validVariant=(variant,assets)=>isObject(variant)&&validPalette(variant.palette)&&validPalette(variant.sidebar)&&typeof variant.logo==="string"&&PATH.test(variant.logo)&&assets[variant.logo]&&(!Object.hasOwn(variant,"background")||(typeof variant.background==="string"&&PATH.test(variant.background)&&assets[variant.background]));
 function validate(manifest){
  if(!isObject(manifest)||manifest.format_version!==FORMAT||![1,RENDERER].includes(manifest.renderer_version)||typeof manifest.version!=="string"||!isObject(manifest.assets)||!Array.isArray(manifest.themes)||!PATH.test(manifest.default_logo)||!manifest.assets[manifest.default_logo])throw Error("Invalid theme manifest");
  for(const [path,asset] of Object.entries(manifest.assets))if(!PATH.test(path)||!isObject(asset)||!Number.isSafeInteger(asset.size)||asset.size<1||asset.size>5*1024*1024||typeof asset.sha256!=="string"||!/^[a-f0-9]{64}$/i.test(asset.sha256))throw Error("Invalid theme asset");
  if(Object.keys(manifest.assets).length>128||manifest.themes.length!==(manifest.renderer_version===1?6:8)||!/^\d+\.\d+\.\d+$/.test(manifest.version)||Object.values(manifest.assets).reduce((sum,asset)=>sum+asset.size,0)>12*1024*1024)throw Error("Invalid theme limits");
  const ids=new Set();
  for(const theme of manifest.themes){
   if(!isObject(theme)||typeof theme.id!=="string"||!THEME_IDS.has(theme.id)||ids.has(theme.id)||!Number.isInteger(theme.priority)||Math.abs(theme.priority)>1000||!validVariant(theme.light,manifest.assets)||!validVariant(theme.dark,manifest.assets))throw Error("Invalid theme entry");
   const dynamic=theme.calendar;
   if(dynamic===undefined){if(!STATIC_IDS.has(theme.id)||!Array.isArray(theme.start)||!Array.isArray(theme.end)||!validDate(theme.start)||!validDate(theme.end))throw Error("Invalid theme entry");}
   else if(manifest.renderer_version!==2||!((theme.id==="easter"&&dynamic==="western_easter")||(theme.id==="valentine"&&dynamic==="valentine"))||Object.hasOwn(theme,"start")||Object.hasOwn(theme,"end"))throw Error("Invalid theme entry");
   const hasDecoration=Object.hasOwn(theme,"decoration");
   if((theme.id==="easter")!==hasDecoration||(hasDecoration&&(!(typeof theme.decoration==="string")||!PATH.test(theme.decoration)||!theme.decoration.startsWith("assets/decorations/")||!manifest.assets[theme.decoration])))throw Error("Invalid theme entry");
   ids.add(theme.id);
  }
  for(let year=2026;year<=2038;year++)for(let month=1;month<=12;month++)for(let day=1;day<=new Date(year,month,0).getDate();day++){const date=new Date(year,month-1,day),active=manifest.themes.filter(theme=>theme.calendar?eventRange(date,theme.calendar):inRange(date,theme.start,theme.end));if(!active.length)throw Error("Uncovered theme calendar");for(const theme of active)if(active.filter(other=>other.priority===theme.priority).length>1)throw Error("Ambiguous theme priority");}
  return manifest;
 }
 const resolve=(manifest,{date=new Date(),mode="auto",manual=""}={},tone="light")=>{validate(manifest);if(mode==="off")return null;let choices=manifest.themes.filter(theme=>mode==="manual"?theme.id===manual:(theme.calendar?eventRange(date,theme.calendar):inRange(date,theme.start,theme.end)));choices.sort((a,b)=>b.priority-a.priority);const theme=choices[0];return theme?{id:theme.id,decoration:theme.decoration,variant:theme[tone==="dark"?"dark":"light"]}:null;};
 const safeAssetUrl=(path,{baseUrl="/themes/",assetUrls={}}={})=>{if(!PATH.test(path))throw Error("Invalid theme asset path");const supplied=assetUrls[path];if(typeof supplied==="string"&&(/^data:image\/png;base64,[a-z0-9+/=]+$/i.test(supplied)||/^\/themes\/assets\/(?:logos|backgrounds|decorations)\/[a-z0-9-]+\.png(?:\?v=[a-f0-9]{64})?$/.test(supplied)||nativeBlobUrls.includes(supplied)))return supplied;const base=/^\/?themes\/$/.test(String(baseUrl))?String(baseUrl):"/themes/";return base+path;};
 function apply(resolution,source={}){const root=document.documentElement,sidebar=document.querySelector(".sidebar"),logo=document.getElementById("brand-logo");root.dataset.ambiance=resolution?.id||"off";delete root.dataset.themeBackground;root.style.removeProperty("--event-art");root.style.removeProperty("--event-rabbit");for(const token of TOKENS){root.style.removeProperty("--"+token);sidebar?.style.removeProperty("--"+token);}
  if(!resolution){if(logo)logo.src=safeAssetUrl(source.defaultLogo||"assets/logos/default.png",source);return;}for(const [key,value] of Object.entries(resolution.variant.palette))root.style.setProperty("--"+key,value);for(const [key,value] of Object.entries(resolution.variant.sidebar))sidebar?.style.setProperty("--"+key,value);if(resolution.variant.background){root.dataset.themeBackground="true";root.style.setProperty("--event-art",`url("${safeAssetUrl(resolution.variant.background,source)}")`);}if(resolution.decoration)root.style.setProperty("--event-rabbit",`url("${safeAssetUrl(resolution.decoration,source)}")`);if(logo)logo.src=safeAssetUrl(resolution.variant.logo,source);
 }
 async function fetchPack(url){const response=await fetch(url,{cache:"no-store",credentials:"same-origin"});if(!response.ok)throw Error("Theme pack unavailable");let raw;if(response.body?.getReader){const reader=response.body.getReader(),chunks=[];let size=0;try{while(true){const {done,value}=await reader.read();if(done)break;size+=value.length;if(size>65536)throw Error("Theme manifest too large");chunks.push(value);}const data=new Uint8Array(size);let offset=0;for(const chunk of chunks){data.set(chunk,offset);offset+=chunk.length;}raw=JSON.parse(new TextDecoder("utf-8",{fatal:true}).decode(data));}finally{await reader.cancel();}}else raw=await response.json();const manifest=validate(raw),baseUrl=url.replace(/manifest\.json(?:\?.*)?$/,"");return {manifest,baseUrl,assetUrls:Object.fromEntries(Object.entries(manifest.assets).map(([path,asset])=>[path,"/themes/"+path+"?v="+asset.sha256]))};}
 async function load(){try{if(globalThis.__TAURI__?.core?.invoke){const pack=await globalThis.__TAURI__.core.invoke("theme_command",{command:{action:"load"}}),manifest=validate(pack.manifest);return {manifest,baseUrl:"themes/",assetUrls:nativeBlobAssets(manifest,pack.assets)};}}catch(_){/* invalid/unavailable desktop pack falls through to the embedded pack */}
  try{const pack=await fetchPack("/themes/manifest.json");revokeNativeBlobs();return pack;}catch(_){try{const pack=await fetchPack("themes/manifest.json");revokeNativeBlobs();return pack;}catch(error){revokeNativeBlobs();throw error;}}}
 globalThis.ChillThemePack={FORMAT,RENDERER,validate,resolve,apply,load,safeAssetUrl,revokeNativeBlobs};
 if(typeof module!=="undefined")module.exports=globalThis.ChillThemePack;
})();
