"use strict";
const {test}=require("node:test"),assert=require("node:assert/strict"),vm=require("node:vm"),fs=require("node:fs");
const source=fs.readFileSync(__dirname+"/app.js","utf8");
const {catalogWindow}=require("./catalog-window.js");
function fixture(invoke){
 const nodes=new Map(),cardCounts=[],messages=[];
 function element(tag="div",text,cls){return {tagName:tag.toUpperCase(),textContent:text||"",className:cls||"",children:[],style:{},dataset:{},value:"",rect:{width:1000,top:0},
  append(...children){this.children.push(...children);},replaceChildren(...children){this.children=children;if(this.id==="media"&&children[0]?.className?.includes("virtual-window"))cardCounts.push(children[0].children.length);},
  setAttribute(){},addEventListener(){},getBoundingClientRect(){return this.rect;},closest(){return null;},focus(){}};}
 const get=id=>{if(!nodes.has(id)){const n=element();n.id=id;nodes.set(id,n);}return nodes.get(id);};
 get("poster-size").value="200";
 const sandbox={uiText:text=>text,$:get,node:element,invoke,message:m=>messages.push(m),catalogWindow,state:{server_id:"server",profile_id:"profile"},innerHeight:800,CATALOG_PAGE_SIZE:48,
  document:{body:{classList:{contains:()=>false}},activeElement:element()},window:{addEventListener(){}},requestAnimationFrame(){},setTimeout,clearTimeout};
 vm.createContext(sandbox);
 vm.runInContext(source.slice(source.indexOf("function catalogCommand"),source.indexOf("if(typeof module"))+source.slice(source.indexOf("let catalogItems="),source.indexOf('\n$("connect-form")')),sandbox);
 return {sandbox,get,cardCounts,messages,load:()=>vm.runInContext("loadPage()",sandbox),ids:()=>JSON.parse(vm.runInContext("JSON.stringify(catalogItems.map(m=>m.id))",sandbox))};
}
const media=(count,prefix="id-")=>Array.from({length:count},(_,i)=>({id:prefix+i,title:"Video "+i,category:"film",available:true,watched:false}));
test("shipped catalogue loadPage fetches 0/48/96 without gaps and keeps rendered cards bounded",async()=>{
 const items=media(103),offsets=[];
 const f=fixture(async c=>{offsets.push(c.offset);return {items:items.slice(c.offset,c.offset+48),has_more:c.offset+48<items.length};});
 await f.load();
 assert.deepEqual(offsets,[0,48,96]);assert.deepEqual(f.ids(),items.map(m=>m.id));
 assert.equal(new Set(f.ids()).size,103);assert.equal(f.get("catalog-count").textContent,"103 titres");
 assert.ok(f.cardCounts.length===1&&f.cardCounts.every(n=>n>0&&n<48));
 f.get("media").rect.top=-(Number.parseFloat(f.get("media").style.height)-800);
 vm.runInContext('catalogWindowKey="";renderCatalogWindow()',f.sandbox);
 assert.ok(f.get("media").children[0].children.some(n=>n.dataset.mediaId==="id-102"));
 assert.ok(f.get("media").children[0].children.length<48);assert.deepEqual(f.messages,[]);
});
test("background catalogue reload preserves old cards until all batches commit",async()=>{
 let pending,refresh=false;const original=media(2,"old-");
 const f=fixture(c=>refresh?new Promise(resolve=>pending=resolve):Promise.resolve({items:original,has_more:false}));
 await f.load();const grid=f.get("media").children[0];refresh=true;const task=f.load();
 assert.equal(f.get("media").children[0],grid);assert.deepEqual(f.ids(),original.map(m=>m.id));
 pending({items:media(3,"new-"),has_more:false});await task;
 assert.deepEqual(f.ids(),["new-0","new-1","new-2"]);
});
test("failed later catalogue page returns false and preserves the previous complete view",async()=>{
 let refresh=false;
 const original=media(2,"old-"),f=fixture(async c=>{
  if(!refresh)return {items:original,has_more:false};
  if(c.offset===0)return {items:media(48,"new-"),has_more:true};
  throw new Error("network interrupted");
 });
 assert.equal(await f.load(),true);const grid=f.get("media").children[0];refresh=true;
 assert.equal(await f.load(),false);assert.deepEqual(f.ids(),original.map(m=>m.id));assert.equal(f.get("media").children[0],grid);
});
test("profile switch clears old cards and scroll cannot repaint them while the new mirror loads",async()=>{
 let finish;const f=fixture(()=>f.sandbox.state.profile_id==="profile"?Promise.resolve({items:media(2,"old-"),has_more:false}):new Promise(resolve=>finish=resolve));
 await f.load();f.sandbox.state.profile_id="new-profile";const pending=f.load();vm.runInContext("renderCatalogWindow()",f.sandbox);
 assert.deepEqual(f.ids(),[]);assert.equal(f.get("media").children.length,0);
 finish({items:media(1,"new-"),has_more:false});await pending;assert.deepEqual(f.ids(),["new-0"]);
});
test("catalogue overlapping batches do not duplicate cards or lose later items",async()=>{
 const items=media(63),offsets=[];
 const f=fixture(async c=>{offsets.push(c.offset);return c.offset===0?{items:items.slice(0,48),has_more:true}:{items:items.slice(47),has_more:false};});
 await f.load();assert.deepEqual(offsets,[0,48]);assert.deepEqual(f.ids(),items.map(m=>m.id));assert.deepEqual(f.messages,[]);
});
test("late catalogue response cannot replace the new profile catalogue",async()=>{
 let finishOld;const calls=[];
 const f=fixture(c=>{calls.push(c.offset);if(f.sandbox.state.profile_id==="profile")return new Promise(resolve=>{finishOld=resolve;});return Promise.resolve({items:media(2,"new-"),has_more:false});});
 const old=f.load();f.sandbox.state.profile_id="new-profile";await f.load();
 finishOld({items:media(48,"old-"),has_more:true});await old;
 assert.deepEqual(f.ids(),["new-0","new-1"]);assert.deepEqual(calls,[0,0]);assert.equal(f.get("catalog-count").textContent,"2 titres");
});
test("late catalogue response cannot replace a newer query on the same profile",async()=>{
 let finishOld;const calls=[];
 const f=fixture(c=>{calls.push(c.q);if(!c.q)return new Promise(resolve=>{finishOld=resolve;});return Promise.resolve({items:media(1,"filtered-"),has_more:false});});
 const old=f.load();f.get("search").value="wanted";await f.load();finishOld({items:media(48),has_more:true});await old;
 assert.deepEqual(f.ids(),["filtered-0"]);assert.deepEqual(calls,[null,"wanted"]);assert.deepEqual(f.messages,[]);
});
test("warm facet base is reused locally and invalidated after a catalogue mutation",async()=>{
 const items=media(103),calls=[];const f=fixture(async command=>{calls.push(command.offset);return {items:items.slice(command.offset,command.offset+48),has_more:command.offset+48<items.length};});
 await f.load();assert.deepEqual(calls,[0,48,96]);
 await vm.runInContext("loadPage(true)",f.sandbox);assert.deepEqual(calls,[0,48,96],"a local facet redraw must not invoke UiClient again");
 vm.runInContext("invalidateFacetBase()",f.sandbox);await f.load();
 assert.deepEqual(calls,[0,48,96,0,48,96]);
});
