const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
test('shipped picker loads every batch without gaps and preserves selection while scrolling',async()=>{
 const source=fs.readFileSync(__dirname+'/../catalog.js','utf8'),app=fs.readFileSync(__dirname+'/../app.js','utf8');
 const size=Number(app.match(/const CATALOG_PAGE_SIZE=(\d+)/)[1]);
 const rust=fs.readFileSync(__dirname+'/../../../crates/chill-client/src/ui.rs','utf8');
 assert.ok(rust.includes('("limit", "'+size+'".into())'),'JS page size must match closed Rust transport');
 const start=source.indexOf('let pickerOffset='),end=source.indexOf('\n$("show-playlists")',start),nodes=new Map(),seen=[],offsets=[];
 function element(){return {value:'',disabled:false,children:[],style:{},scrollTop:0,clientHeight:240,append(x){this.children.push(x);},replaceChildren(...x){this.children=x;},setAttribute(){},addEventListener(){},showModal(){},close(){}};}
 const get=id=>{if(!nodes.has(id))nodes.set(id,element());return nodes.get(id);};
 const media=Array.from({length:103},(_,i)=>({id:'id-'+i,title:'Video '+i}));
 const sandbox={$:get,Set,Math,state:{server_id:'s',profile_id:'p'},message:()=>{},node:()=>element(),CATALOG_PAGE_SIZE:size,collectionView:{id:'collection',revision:1,media_ids:[]},collectionKind:'playlist',
  button:(text,onclick)=>({...element(),textContent:text,onclick}),catalogCommand:(_,offset)=>({offset}),openCollection:async()=>{},
  run:async command=>{offsets.push(command.offset);const items=media.slice(command.offset,command.offset+size);seen.push(...items.map(m=>m.id));return {items,has_more:command.offset+size<media.length};}};
 vm.createContext(sandbox);vm.runInContext(source.slice(start,end),sandbox);
 await get('collection-add').onclick();
 get('picker-items').children[0].children[0].onclick();
 assert.deepEqual([...new Set(seen)],media.map(m=>m.id));
 assert.ok(get('picker-items').children[0].children.length<20);
 get('picker-items').scrollTop=96*48;vm.runInContext('renderPicker()',sandbox);
 assert.ok(get('picker-items').children[0].children.some(n=>n.textContent==='Video 102'));
 get('picker-items').scrollTop=0;vm.runInContext('renderPicker()',sandbox);
 assert.equal(get('picker-items').children[0].children[0].textContent,'✓ Video 0');
 assert.deepEqual(JSON.parse(vm.runInContext('JSON.stringify([...pickerSelection])',sandbox)),['id-0']);
 assert.deepEqual(offsets,[0,48,96]);
});
