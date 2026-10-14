const test=require('node:test'),assert=require('node:assert/strict'),{fixture}=require('./library-harness.js');
test('already-booted context is baselined before the first later refresh and profile changes invalidate pages',async()=>{
 const f=fixture(async()=>({items:[]}));await f.execute('openCollections("saga")');assert.equal(f.sandbox.shellSection,'collections');
 f.sandbox.window.catalogProfileChanged();assert.equal(f.sandbox.shellSection,'collections','a first refresh must not be mistaken for a profile switch');
 f.sandbox.state.profile_id='other';f.sandbox.window.catalogProfileChanged();assert.equal(f.sandbox.shellSection,'catalog');assert.equal(f.json('collectionView'),null);
});
test('collections use poster tiles and an empty collection keeps its plus and back navigation',async()=>{
 const f=fixture(async c=>c.action==='sagas'?{items:[{id:'saga',name:'Ma saga',media_ids:[]}]}:{saga:{id:'saga',name:'Ma saga',revision:1,media_ids:[]}});
 await f.execute('openCollections("saga")');const card=f.get('collections-list').children[0];assert.equal(card.className,'poster-tile');assert.equal(card.children[0].className,'poster');assert.equal(f.get('collection-create').hidden,true);
 await card.onclick();assert.equal(f.sandbox.shellSection,'collection');assert.equal(f.get('collection-add').hidden,false);await f.get('collection-back').onclick();assert.equal(f.sandbox.shellSection,'collections');
});
