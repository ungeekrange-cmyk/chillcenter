import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {assessGraph,checkShippingGraphs,shippingGraphs} from './security-shipping-dependencies.mjs';

test('known affected versions are blocked; absence or patched versions pass',()=>{
 for(const version of ['0.15.0','0.18.5','0.19.9'])assert.equal(assessGraph('glib v'+version).length,1);
 for(const graph of ['tauri v2.12.3','glib v0.14.8','glib v0.20.0','glib v0.21.5','glib v1.0.0','not-glib v0.18.5'])assert.deepEqual(assessGraph(graph),[]);
 assert.equal(assessGraph('glib v0.18.5\nglib v0.18.5 (*)').length,2);
 assert.equal(assessGraph('glib vunknown').length,1);
});
test('only actual shipping graphs are checked with locked literal arguments',()=>{
 const calls=[];const results=checkShippingGraphs((file,args,options)=>{calls.push({file,args,options});return args[args.indexOf('-p')+1]+' v0.7.1\nserde v1.0.0\n';});
 assert.equal(results.length,2);
 assert.deepEqual(calls.map(call=>call.args[call.args.indexOf('-p')+1]),shippingGraphs.map(graph=>graph.package));
 assert.deepEqual(calls.map(call=>call.args[call.args.indexOf('--target')+1]),['aarch64-apple-darwin','aarch64-unknown-linux-gnu']);
 for(const call of calls){assert.equal(call.file,'cargo');assert(call.args.includes('--locked'));assert.equal(call.options.shell,undefined);assert.equal(call.options.encoding,'utf8');}
 for(const call of calls){
  assert.equal(call.args[call.args.indexOf('--edges')+1],'normal,build');
  assert.equal(call.args[call.args.indexOf('--prefix')+1],'none');
  assert.equal(call.args[call.args.indexOf('--format')+1],'{p}');
 }
});
test('command failure and affected shipping graph both fail closed',()=>{
 assert.throws(()=>checkShippingGraphs(()=>{throw Error('resolution failed');}),/resolution failed/);
 assert.throws(()=>checkShippingGraphs(()=> 'chillcenter-desktop v0.7.1\nglib v0.18.5'),/GHSA-wrw7/);
 for(const response of ['', 'serde v1.0.0', undefined])
  assert.throws(()=>checkShippingGraphs(()=>response),/expected dependency graph root missing/);
});
test('CI stays read-only, pinned and non-deploying; Dependabot has no ignore list',()=>{
 const workflow=readFileSync(new URL('../.github/workflows/dependency-security.yml',import.meta.url),'utf8');
 assert.match(workflow,/contents: read/);assert.match(workflow,/persist-credentials: false/);
 assert.doesNotMatch(workflow,/pull_request_target|secrets\.|write-all|security-events: write/);
 for(const [,action]of workflow.matchAll(/uses: ([^\s]+)/g))assert.match(action,/^actions\/[\w-]+@[a-f0-9]{40}$/);
 assert.match(workflow,/security-shipping-dependencies\.mjs/);
 const dependabot=readFileSync(new URL('../.github/dependabot.yml',import.meta.url),'utf8');
 assert.match(dependabot,/package-ecosystem: cargo/);assert.match(dependabot,/package-ecosystem: github-actions/);
 assert.doesNotMatch(dependabot,/\bignore:|insecure-external-code-execution/);
});
