const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
test('shipped picker covers every page without gaps and preserves selection across navigation',async()=>{
 const source=fs.readFileSync(__dirname+'/../catalog.js','utf8'),app=fs.readFileSync(__dirname+'/../app.js','utf8');
 const size=Number(app.match(/const CATALOG_PAGE_SIZE=(\d+)/)[1]);
 const rust=fs.readFileSync(__dirname+'/../../../crates/chill-client/src/ui.rs','utf8');
 assert.ok(rust.includes('("limit", "'+size+'".into())'),'JS page size must match closed Rust transport');
 const start=source.indexOf('let pickerOffset='),end=source.indexOf('\n$("show-playlists")',start),nodes=new Map(),seen=[],offsets=[];
 function element(){return {value:'',disabled:false,children:[],append(x){this.children.push(x);},replaceChildren(){this.children=[];},showModal(){},close(){}};}
 const get=id=>{if(!nodes.has(id))nodes.set(id,element());return nodes.get(id);};
 const media=Array.from({length:103},(_,i)=>({id:'id-'+i,title:'Video '+i}));
 const sandbox={$:get,Set,Math,CATALOG_PAGE_SIZE:size,collectionView:{id:'collection',revision:1,media_ids:[]},collectionKind:'playlist',
  button:(text,onclick)=>({...element(),textContent:text,onclick}),catalogCommand:(_,offset)=>({offset}),openCollection:async()=>{},
  run:async command=>{offsets.push(command.offset);const items=media.slice(command.offset,command.offset+size);seen.push(...items.map(m=>m.id));return {items,has_more:command.offset+size<media.length};}};
 vm.createContext(sandbox);vm.runInContext(source.slice(start,end),sandbox);
 await get('collection-add').onclick();
 get('picker-items').children[0].onclick();await new Promise(r=>setImmediate(r));
 const pages=[...seen];seen.length=0;
 get('picker-next').onclick();await new Promise(r=>setImmediate(r));pages.push(...seen);seen.length=0;
 get('picker-next').onclick();await new Promise(r=>setImmediate(r));pages.push(...seen);
 assert.deepEqual([...new Set(pages)],media.map(m=>m.id));assert.equal(get('picker-next').disabled,true);
 get('picker-prev').onclick();await new Promise(r=>setImmediate(r));get('picker-prev').onclick();await new Promise(r=>setImmediate(r));
 assert.equal(get('picker-items').children[0].textContent,'✓ Video 0');
 assert.deepEqual(JSON.parse(vm.runInContext('JSON.stringify([...pickerSelection])',sandbox)),['id-0']);
 assert.deepEqual(offsets,[0,0,48,96,48,0]);
});
