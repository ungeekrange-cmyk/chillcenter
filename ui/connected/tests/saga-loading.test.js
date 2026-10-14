const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
test('shipped saga opener waits for artwork before fetching members on single UI slot',async()=>{
 const source=fs.readFileSync(__dirname+'/../catalog.js','utf8'),start=source.indexOf('async function openCollection('),end=source.indexOf('\nfunction ensureCollectionFilter',start);
 let release,busy=false;const pending=new Promise(r=>release=r),attempts=[];
 const element=()=>({hidden:false,value:'',append(){},replaceChildren(){},remove(){},showModal(){}}),nodes=new Map(),get=id=>{if(!nodes.has(id))nodes.set(id,element());return nodes.get(id);};
 const sandbox={$:get,document:{createElement:element},button:element,run:async()=>({saga:{id:'1',name:'Saga',poster:'opaque',media_ids:['a']}}),loadSagaArtwork:async()=>{busy=true;await pending;busy=false;},collectionPage:async()=>attempts.push(busy),ensureCollectionFilter(){},importArtwork(){},Map};
 vm.createContext(sandbox);vm.runInContext(source.slice(start,end),sandbox);
 const opening=vm.runInContext('openCollection("1","saga")',sandbox);for(let i=0;i<8;i++)await Promise.resolve();
 assert.equal(attempts.length,0,'member reads started while artwork owns the UI slot');release();await opening;assert.deepEqual(attempts,[false]);
});
