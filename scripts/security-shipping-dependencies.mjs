#!/usr/bin/env node
// Target-scoped protection complements, never suppresses, Dependabot alerts.
import {execFileSync} from 'node:child_process';
import {dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

export const shippingGraphs=Object.freeze([
 Object.freeze({name:'Mac Apple Silicon',target:'aarch64-apple-darwin',package:'chillcenter-desktop'}),
 Object.freeze({name:'Linux ARM64 server',target:'aarch64-unknown-linux-gnu',package:'chill-server'})
]);

// GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429: glib >=0.15.0,<0.20.0.
// Do not scan Cargo.lock text as though every target-specific crate is shipped.
export function assessGraph(graph){
 const findings=[];
 for(const line of new Set(graph.split(/\r?\n/))){
  if(!line.startsWith('glib v'))continue;
  const version=/^glib v(\d+)\.(\d+)\.(\d+)(?:\s|$)/.exec(line);
  if(!version){findings.push('Unrecognized glib version: manual security review required');continue;}
  if(Number(version[1])===0&&Number(version[2])>=15&&Number(version[2])<20)
   findings.push('GHSA-wrw7-89jp-8q8g: affected glib '+version.slice(1,4).join('.'));
 }
 return findings;
}

export function checkShippingGraphs(run=execFileSync){
 const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
 const results=[];
 for(const graph of shippingGraphs){
  // Literal package/target arguments, locked resolution, no shell or application.
  const tree=run('cargo',['tree','--locked','--color','never','--target',graph.target,
   '-p',graph.package,'--edges','normal,build','--prefix','none','--format','{p}'],
   {cwd:root,encoding:'utf8',timeout:120000,maxBuffer:16*1024*1024});
  // A successful but empty/unexpected tool response must not certify a graph.
  if(typeof tree!=='string'||!tree.startsWith(graph.package+' v'))
   throw Error(graph.name+': expected dependency graph root missing');
  const findings=assessGraph(tree);
  if(findings.length)throw Error(graph.name+': '+findings.join('; '));
  results.push({name:graph.name,target:graph.target,status:'PASS'});
 }
 return results;
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{console.log(JSON.stringify({status:'PASS',advisory:'GHSA-wrw7-89jp-8q8g',graphs:checkShippingGraphs()}));}
 catch(error){console.error(error.message);process.exitCode=1;}
}
