const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../catalog.js'),'utf8');
const start=source.indexOf('function recommendationItems'),end=source.indexOf('function catalogRecommendationRail');
const helpers=vm.runInNewContext(source.slice(start,end)+';({recommendationItems,recommendationScroll})');
test('recommendation cards exclude episodes and deduplicate opaque show keys, never display titles',()=>{
 const items=[{id:'film',category:'film'},{id:'episode',category:'serie',episode:1,series_key:'tmdb:tv:1'},{id:'legacy',category:'serie',series:'Same'},
 {id:'old',category:'serie',series_key:'tmdb:tv:1',title:'Same'},{id:'duplicate',category:'serie',series_key:'tmdb:tv:1',title:'Same'},
 {id:'new',category:'serie',series_key:'tmdb:tv:2',title:'Same'},{id:'film',category:'film'}];
 assert.deepEqual(Array.from(helpers.recommendationItems(items),m=>m.id),['film','old','new']);
});
test('recommendation rail scrolls a full visible width in either direction',()=>{
 const calls=[],viewport={clientWidth:650,scrollBy:args=>calls.push(args)};
 helpers.recommendationScroll(viewport,1);helpers.recommendationScroll(viewport,-1);
 assert.deepEqual(calls.map(x=>x.left),[650,-650]);
});
