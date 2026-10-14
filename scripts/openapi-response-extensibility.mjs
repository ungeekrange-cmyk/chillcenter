// Mechanical normalization of response DTO graphs. Request schemas remain
// unchanged; a schema used by both directions gets a response-only copy.
import {readFileSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const path='docs/api/openapi-2026-10-14.json',spec=JSON.parse(readFileSync(path,'utf8'));
const schemas=spec.components.schemas,requestNames=new Set();
function requestGraph(value,seen=new Set()){
 if(!value||typeof value!=='object')return;
 if(value.$ref?.startsWith('#/components/schemas/')){const name=value.$ref.split('/').at(-1);requestNames.add(name);if(!seen.has(name)){seen.add(name);requestGraph(schemas[name],seen);}}
 for(const [key,item]of Object.entries(value))if(key!=='$ref')requestGraph(item,seen);
}
for(const item of Object.values(spec.paths))for(const operation of Object.values(item))if(operation.requestBody)requestGraph(operation.requestBody);
const visited=new Set(),clones=new Map();
function responseGraph(value){
 if(!value||typeof value!=='object')return;
 if(value.$ref?.startsWith('#/components/schemas/')){
  const original=value.$ref.split('/').at(-1);let name=original;
  if(requestNames.has(original)){
   name=clones.get(original)||original+'Response';
   if(!clones.has(original)){assert.ok(!schemas[name],name+' already exists');clones.set(original,name);schemas[name]=structuredClone(schemas[original]);}
   value.$ref='#/components/schemas/'+name;
  }
  if(!visited.has(name)){visited.add(name);responseGraph(schemas[name]);}
 }
 if(value.type==='object'||value.properties)value.additionalProperties=value.additionalProperties&&typeof value.additionalProperties==='object'?value.additionalProperties:true;
 for(const [key,item]of Object.entries(value))if(key!=='$ref')responseGraph(item);
}
for(const item of Object.values(spec.paths))for(const operation of Object.values(item))for(const response of Object.values(operation.responses||{})){
 const target=response.$ref?.startsWith('#/components/responses/')?spec.components.responses[response.$ref.split('/').at(-1)]:response;
 responseGraph(target.content);
}
schemas.PreferencesInput.properties.autoplay_seconds.default=10;
writeFileSync(path,JSON.stringify(spec,null,2)+'\n');
console.log('Response graphs normalized; '+clones.size+' shared schemas split');
