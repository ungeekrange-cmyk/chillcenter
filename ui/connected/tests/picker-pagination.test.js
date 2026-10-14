const test=require('node:test'),assert=require('node:assert/strict');
const {fixture}=require('./library-harness.js');
function picker(read){
 const f=fixture(read);
 f.execute('collectionView={id:"collection",revision:1,media_ids:[]};collectionKind="playlist";');
 return f;
}
test('shipped poster picker loads every batch and preserves selection while scrolling',async()=>{
 const media=Array.from({length:103},(_,i)=>({id:'id-'+i,title:'Video '+i,category:'film',year:2026}));
 const f=picker(async c=>({items:media.slice(c.offset,c.offset+48),has_more:c.offset+48<media.length}));
 await f.get('collection-add').onclick();
 const grid=()=>f.get('picker-items').children[0].children[0];
 grid().children[0].onclick();
 assert.deepEqual(f.json('pickerItems.map(m=>m.id)'),media.map(m=>m.id));
 assert.ok(grid().children.length<48);assert.equal(grid().children[0].children[0].className,'poster');
 assert.equal(grid().children[0].children[1].children[0].textContent,'Video 0');
 f.get('picker-items').scrollTop=100000;f.execute('renderPicker()');
 assert.ok(grid().children.some(n=>n.dataset.mediaId==='id-102'));
 f.get('picker-items').scrollTop=0;f.execute('renderPicker()');
 assert.equal(grid().children[0].attributes['aria-pressed'],'true');
 assert.deepEqual(f.json('[...pickerSelection]'),['id-0']);
 assert.deepEqual(f.calls.map(c=>c.offset),[0,48,96]);
});
test('picker category and watched filters are sent to the server; selection survives filters',async()=>{
 const f=picker(async c=>({items:[{id:c.category||'first',title:'Titre',category:c.category||'film'}],has_more:false}));
 await f.get('collection-add').onclick();f.get('picker-items').children[0].children[0].children[0].onclick();
 f.get('picker-category').value='documentaire';f.get('picker-watched').value='false';await f.get('picker-category').onchange();
 assert.equal(f.calls.at(-1).category,'documentaire');assert.equal(f.calls.at(-1).watched,false);
 assert.deepEqual(f.json('[...pickerSelection]'),['first']);
});
test('late picker results from an older search or profile never replace the new grid',async()=>{
 let finish;
 const f=picker(c=>!c.q?new Promise(r=>finish=r):Promise.resolve({items:[{id:'new',title:'Nouveau'}],has_more:false}));
 const initial=f.get('collection-add').onclick();await Promise.resolve();await Promise.resolve();
 f.get('picker-search').value='new';await f.execute('pickerPage()');
 finish({items:[{id:'old',title:'Ancien'}],has_more:false});await initial;
 assert.deepEqual(f.json('pickerItems.map(m=>m.id)'),['new']);
 let finishOther;f.sandbox.invoke=c=>new Promise(r=>finishOther=r);const pending=f.execute('pickerPage()');f.sandbox.state.profile_id='other';
 finishOther({items:[{id:'leak',title:'Ancien profil'}],has_more:false});await pending;assert.deepEqual(f.json('pickerItems'),[]);
});
test('saga picker remains film-only while playlists accept all supported categories',async()=>{
 const f=picker(async()=>({items:[],has_more:false}));f.execute('collectionKind="saga";');
 await f.get('collection-add').onclick();assert.equal(f.calls[0].category,'film');assert.equal(f.get('picker-category').disabled,true);
 f.get('picker-category').value='serie';await f.execute('pickerPage()');assert.equal(f.calls.at(-1).category,'film');
});
