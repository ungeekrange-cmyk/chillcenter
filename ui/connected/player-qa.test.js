const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs');
const source=fs.readFileSync(__dirname+'/player-qa.js','utf8');

test('player QA exposes bounded phase traces and reveals controls before the DOM hit-test',()=>{
 assert.match(source,/detail:"TRACE player phase="\+next\+" elapsed_ms="\+\(Date\.now\(\)-startedAt\)\+" document_hidden="\+document\.hidden/);
 const phase=source.indexOf('await setStep("native overlay hit-test");');
 const reveal=source.indexOf('showPlayerControls();assert(!$("player-panel").classList.contains("controls-hidden"),"overlay controls must be shown before hit-test");',phase);
 const rect=source.indexOf('const pauseRect=$("player-pause").getBoundingClientRect();',phase);
 const hit=source.indexOf('document.elementFromPoint(',phase);
 assert.ok(phase>=0&&reveal>phase&&rect>reveal&&hit>rect,'the hit-test must synchronously reveal the existing controls first');
 assert.equal((source.match(/step="/g)||[]).length,3,'phase transitions must use the trace helper rather than silently assigning step');
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
 const host=fs.readFileSync(__dirname+'/../../apps/desktop/src/main.rs','utf8');
 const native=fs.readFileSync(__dirname+'/../../apps/desktop/src/native_player.rs','utf8');
 const prepare=host.indexOf('if mode == "player" &&');
 const evaluate=host.indexOf('webview.eval(format!(',prepare);
 assert.ok(prepare>=0&&evaluate>prepare&&host.slice(prepare,evaluate).includes('qa_prepare_visible_player(webview)'),'player QA must prepare visibility before evaluation');
 assert.match(native,/\#\[cfg\(debug_assertions\)\][\s\S]*qa_prepare_visible_player/);
 for(const action of ['unhide:', 'activateIgnoringOtherApps: true', '.show()', '.set_focus()'])assert.ok(native.includes(action),'native debug preparation must '+action);
});
