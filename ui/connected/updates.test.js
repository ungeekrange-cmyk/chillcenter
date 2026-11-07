"use strict";
const {test}=require("node:test"),assert=require("node:assert/strict"),{visibleVersion,commandForPolicy,isUnsupportedServerError}=require("./updates.js");
test("installed version is reported, not inferred",()=>{assert.equal(visibleVersion({version:"0.1.0"}),"0.1.0");assert.equal(visibleVersion(null),null);});
test("bounded policy and mandatory Mac installation confirmation",()=>{assert.deepEqual(commandForPolicy("server",{revision:3},2,0,"automatic"),{component:"server",mode:"automatic",hour:2,minute:0,expected_revision:3});for(const args of [["mac",{},2,0,"automatic"],["server",{},24,0,"manual"],["shell",{},2,0,"automatic"]])assert.throws(()=>commandForPolicy(...args));});
test("updates do not reset catalogue or use global busy lock; Mac consent is native",()=>{const fs=require("node:fs"),source=fs.readFileSync(__dirname+"/updates.js","utf8").replace(/\/\/[^\n]*/g,"");assert.ok(!/\brun\(/.test(source));assert.ok(!/loadPage\(|syncCatalog\(|catalogItems\s*=/.test(source));assert.ok(!source.includes("window.confirm"));const rust=fs.readFileSync(__dirname+"/../../apps/desktop/src/update_bridge.rs","utf8");assert.match(rust,/app\.dialog\(\)/);assert.match(rust,/receive\.await\.unwrap_or\(false\)/);});
test("only an explicit old-server response is unsupported",()=>{assert.equal(isUnsupportedServerError({status:404}),true);assert.equal(isUnsupportedServerError({code:"ROUTE_NOT_FOUND"}),true);assert.equal(isUnsupportedServerError({status:503,code:"UPDATE_SERVER_UNAVAILABLE"}),false);assert.equal(isUnsupportedServerError(new Error("network")),false);});
test("install appears only for a detected strictly newer stable version",()=>{
 const {hasNewerVersion}=require('./updates.js');
 for(const [installed,available,wanted]of [
  ['0.7.1',null,false],['0.7.1','0.7.1',false],['0.7.1','0.7.0',false],
  ['0.7.1','0.7.2',true],['0.9.9','0.10.0',true],['1.0.0','0.99.99',false],
  [null,'0.7.2',false],['unknown','0.7.2',false],['0.7.1','invalid',false],
  ['0.7.1','0.7.2-beta.1',false]
 ])assert.equal(hasNewerVersion({version:installed},{version:available}),wanted);
});
function updateHarness(){
 const vm=require("node:vm"),fs=require("node:fs"),nodes=[];
 class Element{
  constructor(tag,text){this.tag=tag;this.textContent=text||"";this.children=[];this.dataset={};this.hidden=false;this.disabled=false;this.value="";this.isConnected=true;nodes.push(this);}
  append(...items){this.children.push(...items);for(const item of items)item.parentElement=this;}
  replaceChildren(...items){this.children=[];this.append(...items);}
  contains(target){return this===target||this.children.some(child=>child?.contains?.(target));}
  remove(){if(this.parentElement)this.parentElement.children=this.parentElement.children.filter(child=>child!==this);this.isConnected=false;}
  setAttribute(){}querySelector(selector){return selector===".admin-grid"?grid:null;}
 }
 const grid=new Element("div"),admin=new Element("section");admin.hidden=false;admin.querySelector=selector=>selector===".admin-grid"?grid:null;
 const document={hidden:false,activeElement:null,createElement:tag=>new Element(tag)};
 const states=[
  {installed:{web:{version:"0.7.2"}},policies:{web:{mode:"check",hour:2,minute:0,revision:1}},operations:[],available:{},helper_available:true,check_available:true,timezone:"UTC"},
  {installed:{web:{version:"0.7.2"}},policies:{web:{mode:"check",hour:2,minute:0,revision:1}},operations:[{component:"web",kind:"check",status:"pending"}],available:{},helper_available:true,check_available:true,timezone:"UTC"},
  {installed:{web:{version:"0.7.2"}},policies:{web:{mode:"check",hour:2,minute:0,revision:1}},operations:[{component:"web",kind:"check",status:"failed",result_code:"check_failed"}],available:{},helper_available:true,check_available:true,timezone:"UTC"},
  {installed:{web:{version:"0.7.2"}},policies:{web:{mode:"check",hour:2,minute:0,revision:1}},operations:[{component:"web",kind:"check",status:"succeeded",result_code:"available"}],available:{web:{version:"0.7.4"}},helper_available:true,check_available:true,timezone:"UTC"},
  {installed:{web:{version:"0.7.2"}},policies:{web:{mode:"check",hour:2,minute:0,revision:1}},operations:[{component:"web",kind:"check",status:"succeeded",result_code:"available"}],available:{},helper_available:true,check_available:true,timezone:"UTC"},
  {unsupported:true,installed:{web:{version:"0.7.2"}}},
  {installed:{web:{version:"0.7.2"}},policies:{},operations:[],available:{},helper_available:true,check_available:true,timezone:"UTC"}
 ];let index=0;const commands=[];
 const window={ChillBridge:{},chillReady:Promise.resolve(),__TAURI__:{core:{invoke:async(_name,{command})=>{commands.push(command);if(command.action==="status")return states[index];if(command.action==="server_check"){index=1;return {operation:{id:"check"}};}throw Error("unexpected "+command.action);}}}};
 vm.runInNewContext(fs.readFileSync(__dirname+"/updates.js","utf8"),{window,document,node:(tag,text,cls)=>{const e=new Element(tag,text);e.className=cls||"";return e;},$:id=>id==="admin-panel"?admin:null,MutationObserver:class{observe(){}},setInterval(){},globalThis:{},console,BigInt});
 return {window,document,nodes,commands,setState(value){index=value;},async settled(){for(let i=0;i<4;i++)await new Promise(resolve=>setImmediate(resolve));}};
}
test("check status refreshes in place while a policy input has focus",async()=>{
 const h=updateHarness();await h.settled();const card=h.nodes.find(node=>node.dataset.updateComponent==="web"),form=card.children.find(node=>node.tag==="form"),time=form.children.find(node=>node.tag==="input"),check=card.children.find(node=>node.textContent==="Vérifier maintenant");
 time.value="06:30";h.document.activeElement=time;check.onclick();check.onclick();await h.settled();assert.equal(h.commands.filter(command=>command.action==="server_check").length,1);assert.ok(card.contains(time));assert.equal(time.value,"06:30");assert.ok(card.children.some(node=>node.textContent.includes("En attente du service")));
 h.setState(2);await h.window.chillUpdates.refresh();assert.ok(card.children.some(node=>node.textContent.includes("Vérification impossible")));assert.equal(time.value,"06:30");
 h.setState(3);await h.window.chillUpdates.refresh();assert.ok(card.children.some(node=>node.textContent.includes("Version disponible : 0.7.4")));assert.equal(time.value,"06:30");
});
test("available without cached details remains visible and never claims installation",async()=>{
 const h=updateHarness();await h.settled();h.setState(4);await h.window.chillUpdates.refresh();assert.ok(h.nodes.some(node=>node.textContent.includes("Mise à jour détectée ; détails indisponibles")));assert.ok(!h.nodes.some(node=>node.textContent==="Mise à jour installée"));
});
test("obsolete cards are removed after unsupported or policy-less status",async()=>{
 const h=updateHarness();await h.settled();const first=h.nodes.find(node=>node.dataset.updateComponent==="web");h.setState(5);await h.window.chillUpdates.refresh();assert.equal(first.isConnected,false);
 h.setState(0);await h.window.chillUpdates.refresh();const replacement=h.nodes.filter(node=>node.dataset.updateComponent==="web").at(-1);assert.notEqual(replacement,first);assert.equal(replacement.isConnected,true);
 h.setState(6);await h.window.chillUpdates.refresh();assert.equal(replacement.isConnected,false);
});
