const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(__dirname+'/player-qa.js','utf8');
const host=fs.readFileSync(__dirname+'/../../apps/desktop/src/main.rs','utf8');
const native=fs.readFileSync(__dirname+'/../../apps/desktop/src/native_player.rs','utf8');

// Inspect each actual function rather than counting literals across an entire
// file: adding a new QA entry point must not invalidate an unrelated mode.
function assertPhaseTracking(script){
 const window={};vm.runInNewContext(script,{window});
 for(const name of ['chillPlayerQa','chillNextSeriesQa','chillTransferQa']){
  assert.equal(typeof window[name],'function',name+' must exist');
  const body=window[name].toString();
  assert.match(body,/\blet\s+step\s*=\s*"boot"/,name+' initializes its phase');
  assert.match(body,/const\s+setStep\s*=\s*async\s+next\s*=>\s*\{\s*step\s*=\s*next\s*;/,name+' updates phase through its trace helper');
  assert.equal((body.match(/\bstep\s*=(?!=)/g)||[]).length,2,name+' has only initialization and the traced setter');
 }
}

function assertNativePreparation(hostSource,nativeSource){
 assert.match(hostSource,/#\[cfg\(debug_assertions\)\]\s*let\s+builder\s*=\s*builder[\s\S]*?\.on_page_load\(/);
 const launch=hostSource.slice(hostSource.indexOf('.on_page_load('));
 // Whitespace and extra modes are immaterial. Required modes, error handling
 // and preparation before script evaluation are the contract under test.
 const guard=launch.match(/if\s+matches!\(\s*mode\.as_str\(\)\s*,\s*((?:"[^"]+"\s*\|\s*)*"[^"]+")\s*\)\s*&&\s*native_player::qa_prepare_visible_player\(webview\)\.is_err\(\)\s*\{([\s\S]*?)\n\s*\}/);
 assert.ok(guard,'debug QA must handle native visibility preparation failure');
 const modes=[...guard[1].matchAll(/"([^"]+)"/g)].map(match=>match[1]);
 for(const mode of ['player','next_series','detail_manual'])assert.ok(modes.includes(mode),'visibility preparation must include '+mode);
 assert.match(guard[2],/\breturn\s*;/,'failed preparation must prevent QA evaluation');
 const evaluate=launch.indexOf('webview.eval(format!(');
 assert.ok(evaluate>guard.index+guard[0].length,'prepare visibility before evaluating QA');
 const preparation=nativeSource.match(/#\[cfg\(debug_assertions\)\]\s*pub\s+fn\s+qa_prepare_visible_player[\s\S]*?\n\}/);
 assert.ok(preparation,'native preparation must remain debug-only');
 for(const action of ['unhide:', 'activateIgnoringOtherApps: true', '.show()', '.set_focus()'])assert.ok(preparation[0].includes(action),'native debug preparation must '+action);
}
function withModes(hostSource,change){
 return hostSource.replace(/(matches!\(\s*mode\.as_str\(\)\s*,\s*)((?:"[^"]+"\s*\|\s*)*"[^"]+")(\s*\))/,(_match,before,modes,after)=>before+change([...modes.matchAll(/"([^"]+)"/g)].map(match=>match[1]))+after);
}

test('player QA exposes bounded phase traces and reveals controls before the DOM hit-test',()=>{
 assert.match(source,/detail:"TRACE player phase="\+next\+" elapsed_ms="\+\(Date\.now\(\)-startedAt\)\+" document_hidden="\+document\.hidden/);
 const phase=source.indexOf('await setStep("native overlay hit-test");');
 const reveal=source.indexOf('showPlayerControls();assert(!$("player-panel").classList.contains("controls-hidden"),"overlay controls must be shown before hit-test");',phase);
 const rect=source.indexOf('const pauseRect=$("player-pause").getBoundingClientRect();',phase);
 const hit=source.indexOf('document.elementFromPoint(',phase);
 assert.ok(phase>=0&&reveal>phase&&rect>reveal&&hit>rect,'the hit-test must synchronously reveal the existing controls first');
 assertPhaseTracking(source);
});

test('timer-sensitive player QA waits for a visibility event instead of polling while hidden',()=>{
 const wait=source.indexOf('const waitForVisible=async()=>');
 const listener=source.indexOf('document.addEventListener("visibilitychange",function visible()',wait);
 const ready=source.indexOf('phase=visible-ready',wait);
 const boot=source.indexOf('await setStep("boot");',wait);
 const helperEnd=source.indexOf('const wait=async',wait);
 assert.ok(wait>=0&&listener>wait&&ready>listener&&boot>ready,'player phases must start only after visible readiness');
 assert.equal(source.slice(wait,helperEnd).includes('setTimeout'),false,'visibility readiness must not poll with timers while hidden');
});

test('debug player launch prepares the native application before evaluating QA',()=>{
 assertNativePreparation(host,native);
});

test('phase contract rejects untraced changes without depending on unrelated QA helpers',()=>{
 assertPhaseTracking(source+'\nwindow.unrelatedQa=()=>{let step="boot";};');
 const untraced=source.replace('await setStep("boot");','step="untraced";await setStep("boot");');
 assert.notEqual(untraced,source);assert.throws(()=>assertPhaseTracking(untraced),/only initialization/);
});

test('native visibility contract tolerates mode-list formatting but rejects missing or late preparation',()=>{
 const formatted=withModes(host,modes=>'\n '+[...modes,'future_mode'].map(mode=>JSON.stringify(mode)).join(' |\n ')+'\n');
 assert.notEqual(formatted,host);assertNativePreparation(formatted,native);
 assert.throws(()=>assertNativePreparation(host.replace('native_player::qa_prepare_visible_player(webview).is_err()','false'),native),/preparation failure/);
 const missingMode=withModes(host,modes=>modes.filter(mode=>mode!=='player').map(mode=>JSON.stringify(mode)).join('|'));
 assert.throws(()=>assertNativePreparation(missingMode,native),/must include player/);
 const tooLate=host.replace(/if\s+matches!\(\s*mode\.as_str/,match=>'let _ = webview.eval(format!("too early"));\n'+match);
 assert.notEqual(tooLate,host);assert.throws(()=>assertNativePreparation(tooLate,native),/before evaluating/);
 assert.throws(()=>assertNativePreparation(host,native.replace('.set_focus()','.not_focused()')),/set_focus/);
});

// Execute only the readiness prefix of the shipped helpers. Boot stays pending,
// so these tests cannot connect, scan, create profiles, or launch a decoder.
function readiness(mode,hidden){
 const traces=[],listeners=new Map(),window={chillReady:new Promise(()=>{}),__TAURI__:{core:{invoke:async(command,args)=>{
  assert.equal(command,'qa_result');assert.equal(args.success,false);traces.push(args.detail);
 }}}};
 const document={hidden,addEventListener:(name,fn)=>listeners.set(name,fn),removeEventListener:(name,fn)=>{if(listeners.get(name)===fn)listeners.delete(name);}};
 vm.runInNewContext(source,{window,document,Date,setTimeout(){throw Error('readiness must not poll with timers');}});
 void window.chillPlayerQa({mode});
 return {traces,document,listeners};
}
async function until(check){for(let i=0;i<30;i++){if(check())return;await Promise.resolve();}assert.fail('readiness did not reach expected phase');}

test('all player QA modes emit bounded phase, elapsed-time and visibility traces',async()=>{
 for(const [mode,prefix] of [['player','player'],['next_series','player-next'],['transfer_a','player-transfer']]){
  const f=readiness(mode,false);await until(()=>f.traces.some(trace=>trace.includes('phase=boot ')));
  for(const trace of f.traces)assert.match(trace,new RegExp('^TRACE '+prefix+' phase=[a-z-]+ elapsed_ms=\\d+ document_hidden=false$'));
 }
});

test('player and next-series readiness wait for visibility without booting or polling while hidden',async()=>{
 for(const mode of ['player','next_series']){
  const f=readiness(mode,true);await until(()=>f.listeners.has('visibilitychange'));
  assert.equal(f.traces.length,1);assert.match(f.traces[0],/phase=visible-wait .*document_hidden=true$/);
  f.document.hidden=false;f.listeners.get('visibilitychange')();
  await until(()=>f.traces.some(trace=>trace.includes('phase=boot ')));
  assert.equal(f.listeners.size,0);assert.match(f.traces.at(-1),/document_hidden=false$/);
 }
});
