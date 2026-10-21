const {test}=require('node:test');
const assert=require('node:assert/strict');
const {readFileSync}=require('node:fs');
const path=require('node:path');
const read=name=>readFileSync(path.join(__dirname,name),'utf8');
const group=svg=>svg.match(/<g\b[^>]*>([\s\S]*?)<\/g>/)[1].replace(/\s+/g,'');

test('silhouette keeps reference anatomy and source camera without decoration',()=>{
 const original=read('assets/rabbit-reference.svg'),silhouette=read('assets/rabbit-silhouette.svg');
 assert.match(silhouette,/viewBox="0 0 80 64"/);
 assert.equal(group(silhouette),group(original));
 assert.deepEqual([...silhouette.matchAll(/fill="([^"]+)"/g)].map(m=>m[1]),['#7c8966']);
 assert.doesNotMatch(silhouette,/<path\b|stroke=|<filter\b|<image\b/);
});

test('only the new silhouette is used by the preview; dark variant is one flat tone',()=>{
 assert.match(read('ambient.js'),/image\.src='assets\/rabbit-silhouette\.svg'/);
 assert.doesNotMatch(read('ambient.js'),/rabbit-reference\.svg/);
 assert.match(read('ambient.css'),/\.dark \.rabbit-walk img\{filter:brightness\(0\) invert\(1\)\}/);
 assert.match(read('ambient.css'),/prefers-reduced-motion:reduce/);
});
