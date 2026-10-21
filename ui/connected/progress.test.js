"use strict";
const {test}=require("node:test"),assert=require("node:assert/strict"),fs=require("node:fs"),vm=require("node:vm");
const {percent,started,inProgress,parse}=require("./progress.js");
const id="11111111-1111-4111-8111-111111111111",row=(position=5000,duration=100000)=>({media_id:id,position_ms:position,duration_ms:duration,watched:false});
test("In progress uses measured time, inclusive 5%, and excludes watched videos",()=>{
 assert.equal(percent(row(42900)),42.9);assert.equal(inProgress(row(4999)),false);assert.equal(inProgress(row(5000)),true);
 assert.equal(started(row(1)),true);assert.equal(inProgress({...row(),watched:true}),false);assert.equal(inProgress(row(),true),false);
 for(const duration of [null,undefined,0,-1,NaN,Infinity,"100000"]){const value={...row(),duration_ms:duration};assert.equal(percent(value),null);assert.equal(inProgress(value),false);}
 assert.equal(percent({runtime_minutes:120,position_ms:10000}),null,"TMDB is not a measured duration");
 assert.equal(percent(row(200000)),100);assert.equal(percent(row(-1)),null);
});
test("batched progress validates boundaries, unknown fields and duplicate identities",()=>{
 assert.equal(parse({revision:"r",unchanged:true}),null);assert.equal(parse({revision:"r",items:[row()]}).get(id).position_ms,5000);
 for(const payload of [{revision:"r",items:[row(),row()]},{revision:"r",items:[{...row(),media_id:"../bad"}]},{revision:"r",items:[{...row(),watched:"false"}]},{revision:"r",items:[row(1,0)]},{revision:"r",items:Array(30001).fill(row())}])assert.throws(()=>parse(payload));
});

function fixture(invoke){
 const nodes=new Map(),storage=new Map(),calls=[];
 function element(tag="div"){
  const result={tag,children:[],dataset:{},style:{},attrs:{},isConnected:true,hidden:false,value:"",textContent:"",
   setAttribute(k,v){this.attrs[k]=v;},append(...children){for(const child of children){child.parent=this;this.children.push(child);}},before(child){nodes.set(child.id,child);},
   querySelector(selector){return this.children.find(child=>child.className===selector.slice(1))||null;},remove(){if(this.parent)this.parent.children=this.parent.children.filter(c=>c!==this);}};return result;
 }
 const get=id=>{if(!nodes.has(id)){const e=element();e.id=id;nodes.set(id,e);}return nodes.get(id);};
 const sandbox={state:{stage:"connected",server_id:"s",profile_id:"A"},$:get,node:(tag,text,cls)=>Object.assign(element(tag),{textContent:text||"",className:cls||""}),uiText:(text,...values)=>text.replace(/\{(\d+)\}/g,(_,i)=>values[i]),
  localStorage:{getItem:k=>storage.get(k)||null,setItem:(k,v)=>storage.set(k,v),removeItem:k=>storage.delete(k)},document:{visibilityState:"visible",querySelectorAll:()=>[],addEventListener(){}},setInterval(){},Date,invoke:command=>{calls.push(command);return invoke(command);},message(){}};
 sandbox.window=sandbox;sandbox.globalThis=sandbox;vm.createContext(sandbox);vm.runInContext(fs.readFileSync(__dirname+"/progress.js","utf8"),sandbox);
 return {sandbox,get,calls,storage,element};
}
test("one lightweight background request populates bars, details and local profile cache",async()=>{
 const f=fixture(async()=>({revision:"r",items:[row(42900)]}));await f.sandbox.refreshProgress();
 const poster=f.element();f.sandbox.renderMediaProgress(poster,{id,watched:false});
 const bar=poster.children[0];assert.equal(bar.attrs.role,"progressbar");assert.equal(bar.attrs["aria-valuenow"],"42");assert.equal(bar.children[0].style.width,"42.9%");
 f.sandbox.renderDetailProgress({id,watched:false});assert.equal(f.get("detail-progress").textContent,"Lecture en cours · 42 %");
 await f.sandbox.refreshProgress();assert.equal(f.calls.length,1,"no per-card or per-detail HTTP calls");assert.equal(f.storage.size,1);
 f.sandbox.renderMediaProgress(poster,{id,watched:true});assert.equal(poster.children.length,0);
});
test("late reply from an old profile never paints or persists over the current profile",async()=>{
 const pending=[];const f=fixture(()=>new Promise(resolve=>pending.push(resolve)));
 const old=f.sandbox.refreshProgress();f.sandbox.state.profile_id="B";const current=f.sandbox.refreshProgress();
 pending[1]({revision:"B",items:[]});await current;pending[0]({revision:"A",items:[row()]});await old;
 const poster=f.element();f.sandbox.renderMediaProgress(poster,{id,watched:false});assert.equal(poster.children.length,0);
 assert.deepEqual([...f.storage.keys()],["chillcenter-measured-progress:s:B"]);
});
test("unchanged replies retain the measured cache and 404 does not block the app",async()=>{
 let n=0;const f=fixture(async()=>++n===1?{revision:"r",items:[row()]}:{revision:"r",unchanged:true});
 await f.sandbox.refreshProgress();await f.sandbox.refreshProgress(true);assert.equal(f.calls[1].known_revision,"r");
 const poster=f.element();f.sandbox.renderMediaProgress(poster,{id,watched:false});assert.equal(poster.children.length,1);
 const older=fixture(async()=>{throw {status:404};});await older.sandbox.refreshProgress();await older.sandbox.refreshProgress();assert.equal(older.calls.length,1);await older.sandbox.refreshProgress(true);assert.equal(older.calls.length,2,"explicit action can detect a newly upgraded server");
});

test("In progress navigation keeps search text and can return to ordinary category browsing",async()=>{
 const f=fixture(async()=>({revision:"r",items:[]}));let loads=0;
 f.get("search").value="Doctor";f.get("category").value="film";
 f.sandbox.stopPlayerForNavigation=async()=>true;f.sandbox.shellShow=section=>assert.equal(section,"catalog");f.sandbox.scrollTo=()=>{};f.sandbox.loadPage=async()=>{loads++;};
 await f.get("show-in-progress").onclick();assert.equal(f.sandbox.isProgressView(),true);assert.equal(f.get("category").value,"");assert.equal(f.get("search").value,"Doctor");assert.ok(loads>=1);
 f.sandbox.leaveProgressView();assert.equal(f.sandbox.isProgressView(),false);
});

test("a post-playback refresh waits for an older read then obtains one fresh observation",async()=>{
 const pending=[];const f=fixture(()=>new Promise(resolve=>pending.push(resolve)));
 const first=f.sandbox.refreshProgress(),after=f.sandbox.refreshProgress(true),coalesced=f.sandbox.refreshProgress(true);
 assert.equal(f.calls.length,1);pending.shift()({revision:"old",items:[]});await first;await Promise.resolve();
 assert.equal(f.calls.length,2);pending.shift()({revision:"new",items:[row(42000)]});await Promise.all([after,coalesced]);
 const poster=f.element();f.sandbox.renderMediaProgress(poster,{id,watched:false});assert.equal(poster.children[0].attrs["aria-valuenow"],"42");
});
