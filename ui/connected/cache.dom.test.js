"use strict";
const test=require("node:test"),assert=require("node:assert/strict"),vm=require("node:vm"),fs=require("node:fs");
function harness(invoke){
  const nodes=[];
  class Element{
    constructor(tag,text){this.tag=tag;this.textContent=text||"";this.children=[];this.value="";this.disabled=false;this.valid=true;nodes.push(this);}
    append(...items){this.children.push(...items);}reportValidity(){return this.valid;}
  }
  const grid=new Element("div"),admin={querySelector:()=>grid},window={chillReady:Promise.resolve()},state={playback_cache:{forward_mib:128,back_mib:64}};
  vm.runInNewContext(fs.readFileSync(__dirname+"/cache.js","utf8"),{window,state,node:(...a)=>new Element(...a),$:()=>admin,invoke});
  return {nodes,window,state,form:nodes.find(n=>n.tag==="form"),forward:nodes.find(n=>n.id==="cache-forward-mib"),back:nodes.find(n=>n.id==="cache-back-mib"),status:nodes.find(n=>n.tag==="p"&&!n.textContent),submit(){return this.form.onsubmit({preventDefault(){}});}};
}
test("device settings are displayed and submit only the closed local command",async()=>{
  const calls=[],h=harness(async c=>{calls.push(c);return {cache:c.cache};});await Promise.resolve();
  assert.equal(h.forward.value,"128");assert.equal(h.back.value,"64");
  h.forward.value="256";h.back.value="0";await h.submit();
  assert.deepEqual(JSON.parse(JSON.stringify(calls)),[{action:"save_cache_settings",cache:{forward_mib:256,back_mib:0}}]);
  assert.equal(h.state.playback_cache.forward_mib,256);assert.match(h.status.textContent,/prochaine lecture/);
});
test("invalid HTML values and duplicate submits never issue extra writes",async()=>{
  let resolve,calls=0;const h=harness(()=>{calls++;return new Promise(r=>{resolve=r;});});await Promise.resolve();
  h.form.valid=false;await h.submit();assert.equal(calls,0);h.form.valid=true;
  const pending=h.submit();await h.submit();assert.equal(calls,1);assert.equal(h.forward.disabled,true);
  h.window.refreshPlaybackCache({forward_mib:512,back_mib:256});assert.equal(h.forward.value,"128");
  resolve({cache:{forward_mib:128,back_mib:64}});await pending;assert.equal(h.forward.disabled,false);
});
test("a failed save shows its error and never changes persisted-view state",async()=>{
  const h=harness(async()=>{throw {message:"Enregistrement impossible"};});await Promise.resolve();
  h.forward.value="256";await h.submit();assert.match(h.status.textContent,/impossible/);assert.equal(h.state.playback_cache.forward_mib,128);assert.equal(h.forward.disabled,false);
});
