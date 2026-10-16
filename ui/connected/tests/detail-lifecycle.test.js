const {test}=require('node:test'),assert=require('node:assert/strict');
const {fixture,Element}=require('./library-harness');
function prepare(f){f.get('related-media').append(new Element(),new Element());f.sandbox.loadPage=async()=>{};f.sandbox.syncCatalog=async()=>{};f.sandbox.syncAfterMutation=async()=>{};}
const media=id=>({id,title:id,genres:[],directors:[],cast:[],available:true,category:'film',poster:false});
test('late detail A cannot repaint B; B clears old content immediately',async()=>{
 let release;const f=fixture(c=>c.action==='related_media'?{items:[]}:c.media_id==='A'?new Promise(r=>release=r):{media:media('B'),revision:2});
 prepare(f);
 f.sandbox.Option=class{constructor(text,value){this.textContent=text;this.value=value;}};
 const old=f.execute('openMedia("A")');
 await f.execute('openMedia("B")');
 release({media:media('A'),revision:1});await old;
 assert.equal(f.get('detail-title').textContent,'B');
 assert.equal(f.json('detailView.media.id'),'B');
});
test('detail rating is five immediate stars without a scope selector',async()=>{
 const f=fixture(c=>c.action==='media_detail'?{media:media('B'),revision:2}:{items:[]});
 prepare(f);
 f.sandbox.Option=class{constructor(text,value){this.textContent=text;this.value=value;}};
 await f.execute('openMedia("B")');
 const stars=f.get('detail-stars');assert.equal(stars.children.length,5);
 await stars.children[3].onclick();
 assert.ok(f.calls.some(c=>c.action==='group_rating'&&c.rating===4&&c.scope==='video'));
});
for(const kind of ['series','saga','playlist'])test('play from a member detail preserves its '+kind+' ordering context',async()=>{
 const f=fixture(c=>c.action==='media_detail'?{media:media('B'),revision:2}:{items:[]});prepare(f);
 const played=[];f.sandbox.window.chillPlay=(id,context)=>played.push(JSON.parse(JSON.stringify({id,context})));
 await f.execute('shellSection="collection";collectionKind='+JSON.stringify(kind)+';collectionView={id:"origin",media_ids:["B"]};openMedia("B");');
 await f.get('detail-play').onclick();assert.deepEqual(played,[{id:'B',context:{kind,id:'origin'}}]);
});
test('a film opened from the catalogue never inherits a stale collection context',async()=>{
 const f=fixture(c=>c.action==='media_detail'?{media:media('B'),revision:2}:{items:[]});prepare(f);
 const played=[];f.sandbox.window.chillPlay=(id,context)=>played.push({id,context});
 await f.execute('shellSection="catalog";collectionKind="saga";collectionView={id:"stale",media_ids:["B"]};openMedia("B");');
 await f.get('detail-play').onclick();assert.deepEqual(played,[{id:'B',context:null}]);
});
