// Opt-in synthetic loopback smoke. Never runs against a household/Pi server.
// Structural subset of JSON Schema used by this OpenAPI, not a general validator.
import {readFileSync,writeFileSync} from 'node:fs';
import {request} from 'node:https';
import {randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
const [address,certificate,consent,output]=process.argv.slice(2);
const base=new URL(address);
assert.equal(base.protocol,'https:');assert.equal(base.hostname,'127.0.0.1');
assert.equal(base.pathname,'/api/2026-10-14');assert.equal(consent,'--synthetic-fixture-consent');
const ca=readFileSync(certificate),spec=JSON.parse(readFileSync('docs/api/openapi-2026-10-14.json','utf8'));
let checks=0;
const samples=[];
// media_revision is a u64 file token, frequently beyond JS's 53-bit precision.
// Node24 keeps its JSON lexeme in the reviver context; echo the exact integer.
const parse=bytes=>JSON.parse(bytes,(key,value,context)=>typeof value==='number'&&!Number.isSafeInteger(value)?BigInt(context.source):value);
const stringify=value=>JSON.stringify(value,(key,item)=>typeof item==='bigint'?JSON.rawJSON(item.toString()):item);
function resolve(schema){return schema?.$ref?spec.components.schemas[schema.$ref.split('/').at(-1)]:schema;}
function validate(value,schema,where){
 schema=resolve(schema);assert.ok(schema,where+': missing schema');
 if(schema.anyOf||schema.oneOf){const options=schema.anyOf||schema.oneOf;assert.ok(options.some(option=>{try{validate(value,option,where);return true;}catch{return false;}}),where+': no matching variant');return;}
 if(schema.allOf)for(const option of schema.allOf)validate(value,option,where);
 if(schema.enum)assert.ok(schema.enum.includes(value),where+': enum');
 if(schema.const!==undefined)assert.deepEqual(value,schema.const,where+': const');
 if(schema.type){const types=Array.isArray(schema.type)?schema.type:[schema.type];assert.ok(types.some(type=>type==='null'?value===null:type==='array'?Array.isArray(value):type==='object'?value!==null&&typeof value==='object'&&!Array.isArray(value):type==='integer'?Number.isInteger(value)||typeof value==='bigint':type==='number'?typeof value==='number'&&Number.isFinite(value):typeof value===type),where+': '+JSON.stringify(schema.type));}
 if(value&&typeof value==='object'&&!Array.isArray(value)){
  for(const name of schema.required||[])assert.ok(Object.hasOwn(value,name),where+': required '+name);
  for(const [name,item]of Object.entries(value)){if(schema.properties?.[name])validate(item,schema.properties[name],where+'.'+name);else if(schema.additionalProperties===false)assert.fail(where+': unexpected '+name);else if(schema.additionalProperties&&typeof schema.additionalProperties==='object')validate(item,schema.additionalProperties,where+'.'+name);}
 }
 if(Array.isArray(value)&&schema.items)value.forEach((item,index)=>validate(item,schema.items,where+'['+index+']'));
 if(typeof value==='number'){if(schema.minimum!==undefined)assert.ok(value>=schema.minimum,where+': minimum');if(schema.maximum!==undefined)assert.ok(value<=schema.maximum,where+': maximum');}
}
async function call(method,path,route,body,headers={}){
 const operation=spec.paths[route][method.toLowerCase()];assert.ok(operation);
 if(body!==undefined)validate(body,operation.requestBody.content['application/json'].schema,method+' '+route+' request');
 const data=body===undefined?null:stringify(body);
 const result=await new Promise((resolve,reject)=>{const req=request(base.origin+base.pathname+path,{method,ca,headers:{...(data?{'content-type':'application/json','content-length':Buffer.byteLength(data)}:{}),...headers},timeout:20000},res=>{const chunks=[];res.on('data',part=>chunks.push(part));res.on('end',()=>resolve({status:res.statusCode,headers:res.headers,bytes:Buffer.concat(chunks)}));});req.on('error',reject);req.on('timeout',()=>req.destroy(Error('timeout')));req.end(data);});
 assert.equal(result.headers['x-chill-api-version'],'2026-10-14');
 let response=operation.responses[String(result.status)]||operation.responses.default;assert.ok(response,method+' '+route+': undocumented status '+result.status);
 if(response.$ref)response=spec.components.responses[response.$ref.split('/').at(-1)];
 if(result.bytes.length){const value=parse(result.bytes.toString());validate(value,response.content['application/json'].schema,method+' '+route+' response');checks++;assert.ok(result.status<400,method+' '+route+': error '+value.code);samples.push({route,method:method.toLowerCase(),status:result.status,request:body,response:value});return value;}
 assert.equal(result.status,204);checks++;return null;
}
const get=(path,route=path)=>call('GET',path,route);
const name='Contract QA '+randomUUID();let profile,lease,playlist,saga;
try{
 await get('/server');await get('/profiles');const library=await get('/admin/library');
 assert.ok(library.video_root.startsWith('/private/tmp/chillcenter-ux-api-qa.'),'Only isolated synthetic library may be mutated');
 await get('/admin/scan');await get('/admin/tmdb');await get('/admin/unavailable?offset=0&limit=48','/admin/unavailable');await get('/admin/reassociations?offset=0&limit=48','/admin/reassociations');
 profile=await call('POST','/profiles','/profiles',{name});
 const p='/profiles/'+profile.id;
 await get(p+'/preferences','/profiles/{profile}/preferences');
 const catalog=await get(p+'/catalog?limit=48&grouped=false','/profiles/{profile}/catalog');assert.ok(catalog.items.length,'synthetic catalogue required');
 const media=catalog.items[0],m=p+'/media/'+media.id;
 if(media.category==='serie')await get(p+'/series/'+encodeURIComponent(media.series||media.title),'/profiles/{profile}/series/{series}');
 await get(m,'/profiles/{profile}/media/{media}');await get(m+'/related','/profiles/{profile}/media/{media}/related');await get(m+'/state','/profiles/{profile}/media/{media}/state');await get(m+'/playback-info','/profiles/{profile}/media/{media}/playback-info');
 await get(p+'/ai/context?categories=film,serie&watched=false&available=true','/profiles/{profile}/ai/context');
 await get(p+'/sagas','/profiles/{profile}/sagas');await get(p+'/playlists','/profiles/{profile}/playlists');
 playlist=(await call('POST',p+'/playlists','/profiles/{profile}/playlists',{name})).playlist;
 await get(p+'/playlists/'+playlist.id,'/profiles/{profile}/playlists/{playlist}');
 saga=(await call('POST',p+'/sagas','/profiles/{profile}/sagas',{name})).saga;
 await get(p+'/sagas/'+saga.id,'/profiles/{profile}/sagas/{saga}');
 const grant=await call('POST',p+'/playback/acquire','/profiles/{profile}/playback/acquire',{media_id:media.id,device_id:randomUUID(),context:{kind:'standalone',id:null}});
 lease={session_id:grant.lease.session_id,generation:grant.lease.generation,media_id:grant.lease.media_id,media_revision:grant.lease.media_revision};
 await call('POST',p+'/playback/heartbeat','/profiles/{profile}/playback/heartbeat',lease);
 const bytes=await new Promise((resolve,reject)=>{const req=request(base.origin+base.pathname+'/media/'+media.id+'/stream',{ca,headers:{'x-chill-session':lease.session_id,'x-chill-generation':String(lease.generation),range:'bytes=0-31'},timeout:20000},res=>{const chunks=[];res.on('data',part=>chunks.push(part));res.on('end',()=>resolve({status:res.statusCode,headers:res.headers,body:Buffer.concat(chunks)}));});req.on('error',reject);req.on('timeout',()=>req.destroy(Error('Range timeout')));req.end();});
 assert.equal(bytes.status,206);assert.equal(bytes.body.length,32);assert.match(bytes.headers['content-range'],/^bytes 0-31\/\d+$/);assert.ok(bytes.headers.etag);assert.equal(bytes.headers['x-chill-api-version'],'2026-10-14');
 await call('POST',p+'/playback/progress','/profiles/{profile}/playback/progress',{...lease,sequence:1,position_ms:0,duration_ms:60000,speed:1,final_elapsed_ms:0,playing:false,buffering:false,seek:false});
 await get(m+'/subtitles?session='+lease.session_id+'&generation='+lease.generation,'/profiles/{profile}/media/{media}/subtitles');
 await call('POST',p+'/playback/release','/profiles/{profile}/playback/release',{lease});lease=null;
 await call('DELETE',p+'/sagas/'+saga.id,'/profiles/{profile}/sagas/{saga}',{expected_revision:saga.revision});saga=null;
 await call('DELETE',p+'/playlists/'+playlist.id,'/profiles/{profile}/playlists/{playlist}');playlist=null;
 await call('DELETE',p,'/profiles/{profile}',undefined,{'x-chill-confirm-delete':profile.id});profile=null;
 if(output){assert.ok(output.startsWith('/private/tmp/chillcenter-ux-api-qa.'),'Fixture-only exclusive output');writeFileSync(output,stringify(samples),{flag:'wx',mode:0o600});}
 console.log('HTTPS/OpenAPI structural smoke PASS: '+checks+' JSON responses + video206/32bytes; no NAS/provider calls');
}finally{
 // Delete only objects created by this invocation on the isolated fixture.
 if(profile){const p='/profiles/'+profile.id;
  if(lease)await call('POST',p+'/playback/release','/profiles/{profile}/playback/release',{lease}).catch(()=>{});
  if(saga)await call('DELETE',p+'/sagas/'+saga.id,'/profiles/{profile}/sagas/{saga}',{expected_revision:saga.revision}).catch(()=>{});
  if(playlist)await call('DELETE',p+'/playlists/'+playlist.id,'/profiles/{profile}/playlists/{playlist}').catch(()=>{});
  await call('DELETE',p,'/profiles/{profile}',undefined,{'x-chill-confirm-delete':profile.id}).catch(()=>{});
 }
}
