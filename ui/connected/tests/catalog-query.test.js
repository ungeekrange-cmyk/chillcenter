"use strict";
const assert=require("node:assert/strict"),fs=require("node:fs"),vm=require("node:vm");
const source=fs.readFileSync(require.resolve("../app.js"),"utf8"),start=source.indexOf("function catalogCommand"),end=source.indexOf("\nfunction catalogFilters",start);
assert.notEqual(start,-1,"catalog command helper must be present");
const catalogCommand=vm.runInNewContext(source.slice(start,end)+";catalogCommand");
const filters={category:"serie",q:"episode",genre:"Drame",year:"0",watched:"false",sort:"rating_desc",available:"true"};
assert.deepEqual(JSON.parse(JSON.stringify(catalogCommand(filters,0,true))),{action:"catalog",offset:0,category:"serie",q:"episode",genre:"Drame",year:0,watched:false,sort:"rating_desc",grouped:true,available:true});
assert.equal(catalogCommand({...filters,year:"",watched:"",available:""},48,false).grouped,false,"the picker requests ungrouped individual media");
assert.equal(catalogCommand({...filters,year:"",watched:"",available:""},48,false).offset,48,"the picker page stays bounded through the allowlisted offset");
