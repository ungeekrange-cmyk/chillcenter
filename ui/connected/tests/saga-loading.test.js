const test=require('node:test'),assert=require('node:assert/strict');
const {fixture}=require('./library-harness.js');
test('saga members open even while optional cover artwork is pending',async()=>{
 let release,artworkPending=false;const pending=new Promise(r=>release=r),attempts=[];
 const f=fixture(c=>{
  if(c.action==='saga')return {saga:{id:'1',name:'Saga',poster:'opaque',media_ids:['a'],revision:1}};
  if(c.action==='media_detail'){attempts.push(artworkPending);return {media:{id:'a',title:'Film',available:true}};}
  return {};
 });
 f.sandbox.collectionMosaic=()=>{artworkPending=true;return pending.finally(()=>{artworkPending=false;});};
 await f.execute('openCollection("1","saga")');
 assert.deepEqual(attempts,[true],'member reads must not wait for optional artwork');release();
});
