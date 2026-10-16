"use strict";
const test=require("node:test"),assert=require("node:assert/strict"),vm=require("node:vm"),fs=require("node:fs");
function harness(){
 const nodes=new Map(),listeners={};
 class Element{
  constructor(id=""){this.id=id;this.children=[];this.hidden=false;this.textContent="";this.onclick=null;this.attributes={};this.dataset={};this.className="";this.classList={toggle(){},add(){}};}
  append(...children){this.children.push(...children);children.forEach(child=>{if(child&&typeof child==="object")child.parent=this;});}
  replaceChildren(...children){this.children=[];this.append(...children);}
  setAttribute(key,value){this.attributes[key]=String(value);}getAttribute(key){return this.attributes[key];}
  focus(){this.focused=true;}contains(target){for(let node=target;node;node=node.parent)if(node===this)return true;return false;}
 }
 const ids=["facet-controls","facet-genres","facet-years","facet-genres-pop","facet-years-pop","facet-genre-list","facet-year-list","facet-genre-empty","facet-year-empty","facet-reset","facet-selected","facet-genre-count","facet-year-count"];
 ids.forEach(id=>nodes.set(id,new Element(id)));const root=nodes.get("facet-controls");for(const id of ids.filter(id=>id!=="facet-controls"))root.append(nodes.get(id));
 const document={getElementById:id=>nodes.get(id)||null,createElement:()=>new Element(),addEventListener:(type,fn)=>listeners[type]=fn};
 const window={};vm.runInNewContext(fs.readFileSync(__dirname+"/filters.js","utf8"),{window,document,globalThis:window,module:{exports:{}},String,Number,Set,Array,Math});
 return {window,nodes,listeners};
}
test("facet DOM updates instantly, starts a new interval on a third year, and closes accessibly",()=>{
 const f=harness(),changes=[];f.window.catalogFacets.init({onChange:value=>changes.push(value)});f.window.catalogFacets.draw([{year:2025,genres:["Drame"]},{year:2023,genres:["Aventure"]},{year:2021,genres:["Drame"]}]);
 const genres=f.nodes.get("facet-genre-list").children,years=f.nodes.get("facet-year-list").children;
 genres.find(button=>button.textContent==="Drame").onclick();genres.find(button=>button.textContent==="Aventure").onclick();
 years.find(button=>button.textContent==="2025").onclick();years.find(button=>button.textContent==="2021").onclick();years.find(button=>button.textContent==="2023").onclick();
 assert.deepEqual(JSON.parse(JSON.stringify(f.window.catalogFacets.values())),{genres:["Drame","Aventure"],years:[2023]});assert.equal(changes.length,5);
 f.nodes.get("facet-years").onclick();assert.equal(f.nodes.get("facet-years-pop").hidden,false);f.listeners.keydown({key:"Escape",preventDefault(){}});assert.equal(f.nodes.get("facet-years-pop").hidden,true);assert.equal(f.nodes.get("facet-years").focused,true);
 f.nodes.get("facet-genres").onclick();f.listeners.pointerdown({target:new (f.nodes.get("facet-controls").constructor)()});assert.equal(f.nodes.get("facet-genres-pop").hidden,true);
});
