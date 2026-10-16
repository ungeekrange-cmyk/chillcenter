const test=require('node:test'),assert=require('node:assert/strict');
const {fixture}=require('./library-harness');
test('pure collection redraw retains cards, but a changed filter renews move actions',async()=>{
 const f=fixture(async()=>null);
 f.execute('collectionView={id:"list",media_ids:["a","b"]};collectionKind="playlist";collectionDetails=new Map([["a",{id:"a",title:"Alpha",available:true}],["b",{id:"b",title:"Beta",available:true}]]);renderCollectionMembers();');
 const first=f.get('collection-members').children[0].children[0];
 const down=card=>card.children[1].children[2].children[2];
 assert.equal(down(first).disabled,false);
 f.execute('renderCollectionMembers();');assert.equal(f.get('collection-members').children[0].children[0],first);
 f.execute('collectionFilter="alpha";renderCollectionMembers();');
 const filtered=f.get('collection-members').children[0].children[0];
 assert.notEqual(filtered,first);assert.equal(down(filtered).disabled,true,'filtered single item cannot retain an old move-to-B action');
});
