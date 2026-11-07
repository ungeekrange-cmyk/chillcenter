"use strict";
const test=require("node:test"),assert=require("node:assert/strict"),{fixture}=require("./library-harness.js");
function model() {
 const view={id:"group",name:"Pour ce soir",revision:3,poster:null,media_ids:["a","b"]};
 const movies={a:{id:"a",title:"Premier film",year:2025,poster:"a.jpg",available:true},b:{id:"b",title:"Autre film",poster:"b.jpg",available:true}};
 const read=async c=>c.action==="playlist"?{playlist:{...view}}:c.action==="playlists"?{items:[view]}:c.action==="media_detail"?{media:movies[c.media_id],revision:1}:{ok:true};
 return {view,movies,read};
}
test("playlist poster selection uses a member copy only after explicit confirmation",async()=>{
 const m=model(),f=fixture(m.read);await f.execute('openCollection("group","playlist")');f.sandbox.window.collectionManagement.openCover();
 const choices=f.get("cover-members-grid").children;assert.equal(choices.length,2);assert.equal(f.get("cover-apply").disabled,true);
 choices[1].onclick();assert.equal(f.get("cover-apply").disabled,false);assert.equal(f.calls.some(c=>c.action==="playlist_member_artwork"),false);
 await f.get("cover-apply").onclick();const command=f.calls.find(c=>c.action==="playlist_member_artwork");
 assert.equal(command.media_id,"b");assert.equal(command.id,"group");assert.equal(command.expected_revision,3);assert.ok(!f.calls.some(c=>c.action==="edit_media"||c.action==="tmdb_select"));
});
test("deletion is available on the index, names its target and prevents duplicate submissions",async()=>{
 const m=model();let finish;const f=fixture(c=>c.action==="delete_playlist"?new Promise(resolve=>{finish=resolve;}):m.read(c));
 await f.execute('openCollections("playlist")');const card=f.get("collections-list").children[0];card.children[1].children[1].onclick();
 assert.equal(f.get("collection-delete-confirm").open,true);assert.equal(f.get("collection-delete-name").textContent,"Pour ce soir");
 const pending=f.get("collection-delete-yes").onclick();await f.get("collection-delete-yes").onclick();assert.equal(f.calls.filter(c=>c.action==="delete_playlist").length,1);
 finish({ok:true});await pending;assert.equal(f.get("collection-delete-confirm").open,false);assert.equal(f.sandbox.shellSection,"collections");
});
test("closing deletion performs no mutation and profile changes discard a late search",async()=>{
 const m=model();let finish;const f=fixture(c=>c.action==="collection_artwork_candidates"?new Promise(resolve=>{finish=resolve;}):m.read(c));
 await f.execute('openCollection("group","playlist")');f.get("collection-delete").onclick();f.get("collection-delete-no").onclick();assert.ok(!f.calls.some(c=>c.action.startsWith("delete_")));
 f.sandbox.window.collectionManagement.openCover();f.get("cover-tmdb-tab").onclick();f.get("cover-tmdb-query").value="Space";
 const pending=f.get("cover-tmdb-search").onsubmit({preventDefault(){}});f.sandbox.state.profile_id="other";f.sandbox.window.catalogProfileChanged();
 finish({candidates:[{token:"old",title:"Old result",source:"TMDB"}]});await pending;
 assert.equal(f.get("collection-cover-dialog").open,false);assert.equal(f.get("cover-tmdb-grid").children.length,0);assert.ok(!f.calls.some(c=>c.action==="collection_artwork_preview"));
});
test("a stale TMDB search cannot replace newer results",async()=>{
 const m=model();let old;const f=fixture(c=>c.action==="collection_artwork_candidates"?(c.query==="Old"?new Promise(r=>{old=r;}):{candidates:[]}):m.read(c));
 await f.execute('openCollection("group","playlist")');f.sandbox.window.collectionManagement.openCover();f.get("cover-tmdb-tab").onclick();
 f.get("cover-tmdb-query").value="Old";const first=f.get("cover-tmdb-search").onsubmit({preventDefault(){}});
 f.get("cover-tmdb-query").value="New";await f.get("cover-tmdb-search").onsubmit({preventDefault(){}});
 old({candidates:[{token:"stale",title:"Stale",source:"TMDB"}]});await first;
 assert.equal(f.get("cover-tmdb-grid").children.length,0);assert.ok(!f.calls.some(c=>c.action==="collection_artwork_preview"));
});
test("revision conflict recovery stays in the cover dialog and requires a new confirmation",async()=>{
 const m=model();let recovery;const f=fixture(c=>{if(c.action==='playlist_member_artwork')throw {status:409,code:'PROFILE_CONFLICT',message:'Conflict'};return m.read(c);});
 f.sandbox.window.offerConflictRecovery=(error,target)=>{assert.equal(error.code,'PROFILE_CONFLICT');recovery=target;};
 await f.execute('openCollection("group","playlist")');f.sandbox.window.collectionManagement.openCover();f.get('cover-members-grid').children[0].onclick();await f.get('cover-apply').onclick();
 assert.equal(recovery.root,f.get('collection-cover-dialog'));assert.equal(recovery.current(),true);
 m.view.revision=4;await recovery.reload();assert.equal(f.get('cover-apply').disabled,true);assert.equal(f.json('collectionView.revision'),4);
 assert.equal(f.calls.filter(c=>c.action==='playlist_member_artwork').length,1,'refresh must never retry a write');
 f.get('cover-members-grid').children[1].onclick();assert.equal(f.get('cover-apply').disabled,false);
});
test("rename recovery preserves the draft while refreshing its CAS revision",async()=>{
 const m=model();let recovery;const f=fixture(c=>{if(c.action==='rename_playlist')throw {status:409,code:'PROFILE_CONFLICT',message:'Conflict'};return m.read(c);});
 f.sandbox.window.offerConflictRecovery=(error,target)=>{recovery=target;};
 await f.execute('openCollection("group","playlist")');f.sandbox.window.collectionManagement.openEditor();f.get('collection-name').value='My draft';await f.get('collection-rename').onsubmit({preventDefault(){}});
 assert.equal(recovery.root,f.get('collection-settings-dialog'));m.view.revision=5;await recovery.reload();assert.equal(f.get('collection-name').value,'My draft');assert.equal(f.json('collectionView.revision'),5);
});
