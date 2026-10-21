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
