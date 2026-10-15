const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
test('saga members open even while the optional artwork request is still pending',async()=>{
 const source=fs.readFileSync(__dirname+'/../catalog.js','utf8'),start=source.indexOf('async function openCollection('),end=source.indexOf('\nfunction ensureCollectionFilter',start);
 let release,busy=false;const pending=new Promise(r=>release=r),attempts=[];
 const element=()=>({hidden:false,value:'',classList:{remove(){}},append(){},replaceChildren(){},remove(){},showModal(){},querySelector(){return element();}}),nodes=new Map(),get=id=>{if(!nodes.has(id))nodes.set(id,element());return nodes.get(id);};
 const sandbox={$:get,document:{createElement:element},button:element,stopPlayerForNavigation:async()=>true,collectionEpoch:0,collectionScope:()=>"s:p",collectionCurrent:()=>true,libraryRead:async()=>({saga:{id:'1',name:'Saga',poster:'opaque',media_ids:['a']}}),loadCollectionArtwork:async()=>{busy=true;await pending;busy=false;},collectionPage:async()=>attempts.push(busy),ensureCollectionFilter(){},showCollectionPage(){},importArtwork(){},Map};
 vm.createContext(sandbox);vm.runInContext(source.slice(start,end),sandbox);
 const opening=vm.runInContext('openCollection("1","saga")',sandbox);for(let i=0;i<8;i++)await Promise.resolve();
 assert.deepEqual(attempts,[true],'member reads must not wait for the optional artwork request');release();await opening;
});
