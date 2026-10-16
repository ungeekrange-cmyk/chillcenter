const test=require('node:test'),assert=require('node:assert/strict');
const {configurationCategories,configurationMatches,configurationLibraryCommand}=require('./configuration');
test('configuration offers one global view and at most nine dedicated categories',()=>{
 assert.ok(configurationCategories.length<=10);assert.equal(configurationCategories[0][0],'all');assert.equal(new Set(configurationCategories.map(([key])=>key)).size,configurationCategories.length);
});
test('global settings search normalizes accents/case and combines search terms',()=>{
 assert.equal(configurationMatches('all','BIBLIOTHEQUE local','library','Localisation de la bibliothèque'),true);
 assert.equal(configurationMatches('playback','bibliothèque','library','Bibliothèque'),false);
 assert.equal(configurationMatches('all','audio anglais','playback','Langue audio'),false);
});
test('one-root save stays legacy compatible; additional locations are explicit including removal',()=>{
 assert.deepEqual(configurationLibraryCommand(' /films ',[],{revision:3}),{action:'save_library',video_root:'/films',expected_revision:3});
 assert.deepEqual(configurationLibraryCommand('/films',[' /series ',''],{revision:3}),{action:'save_library',video_root:'/films',expected_revision:3,additional_video_roots:['/series']});
 assert.deepEqual(configurationLibraryCommand('',[],{revision:4,additional_video_roots:['/series']}),{action:'save_library',video_root:null,expected_revision:4,additional_video_roots:[]});
 assert.throws(()=>configurationLibraryCommand('/films',['/films'],{revision:0}),/unique/);
 assert.throws(()=>configurationLibraryCommand('/films',Array.from({length:16},(_,i)=>'/root'+i),{revision:0}),/16/);
});
