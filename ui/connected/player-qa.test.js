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
