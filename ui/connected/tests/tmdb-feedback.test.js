const {test}=require('node:test'),assert=require('node:assert/strict');
const {fixture}=require('./library-harness');
const media={id:'fixture',title:'Old title',category:'film',tmdb_id:null};
const candidate={id:42,title:'Selected title',kind:'movie',year:2024};
for(const scope of ['media','series','season'])test('TMDB '+scope+' selection immediately closes choices and reports pending without a global lock',async()=>{
 let release,reloaded=0;
 const f=fixture(c=>c.action==='tmdb_candidates'?{candidates:[{...candidate,kind:scope==='media'?'movie':'tv'}]}:c.action==='tmdb_select'?new Promise(resolve=>release=resolve):{media:{...media,tmdb_id:42},revision:7});
 f.sandbox.syncAfterMutation=async()=>{};f.sandbox.reload=async()=>{reloaded++;};f.sandbox.current=()=>true;
 f.execute('renderTmdbPanel($("test-target"),'+JSON.stringify(media)+','+JSON.stringify(scope)+',current,reload)');
 const target=f.get('test-target');await target.children.find(n=>n.textContent==='Rechercher TMDB').onclick();
 const results=target.children.find(n=>n.className==='tmdb-candidates'),choice=results.children.find(n=>n.tagName==='BUTTON');
 const pending=choice.onclick(),status=target.children.find(n=>n.className.includes('tmdb-association-status'));
 assert.equal(status.textContent,'Association en cours avec Selected title · 2024…');assert.equal(status.attributes['aria-busy'],'true');assert.equal(results.children.length,0);assert.equal(f.sandbox.busy,false);
 await new Promise(resolve=>setImmediate(resolve));assert.equal(f.calls.filter(c=>c.action==='tmdb_select').length,1);
 await choice.onclick();assert.equal(f.calls.filter(c=>c.action==='tmdb_select').length,1,'Double click never resubmits');
 release({saved:true});await pending;assert.equal(status.textContent,'Association confirmée : Selected title · 2024.');assert.equal(status.attributes['aria-busy'],'false');assert.equal(reloaded,1);
});
test('failed TMDB association stays visibly unconfirmed and can be retried',async()=>{
 const f=fixture(c=>c.action==='tmdb_candidates'?{candidates:[candidate]}:c.action==='tmdb_select'?Promise.reject({message:'Fixture failure'}):{media,revision:1});
 f.sandbox.current=()=>true;f.sandbox.reload=async()=>{throw Error('Failure must not reload as success');};
 f.execute('renderTmdbPanel($("test-target"),'+JSON.stringify(media)+',"media",current,reload)');const target=f.get('test-target');
 const search=target.children.find(n=>n.textContent==='Rechercher TMDB');await search.onclick();await target.children.find(n=>n.className==='tmdb-candidates').children.find(n=>n.tagName==='BUTTON').onclick();
 assert.match(target.children.find(n=>n.className==='tmdb-association-status').textContent,/Association non confirmée.*relancez la recherche/);assert.equal(search.disabled,false);
});
test('late TMDB confirmation after leaving the detail cannot repaint or reload another detail',async()=>{
 let release,active=true,reloaded=0;const f=fixture(c=>c.action==='tmdb_candidates'?{candidates:[candidate]}:c.action==='tmdb_select'?new Promise(resolve=>release=resolve):{media,revision:1});
 f.sandbox.current=()=>active;f.sandbox.reload=async()=>reloaded++;f.sandbox.syncAfterMutation=async()=>{};
 f.execute('renderTmdbPanel($("test-target"),'+JSON.stringify(media)+',"media",current,reload)');const target=f.get('test-target');await target.children.find(n=>n.textContent==='Rechercher TMDB').onclick();
 const pending=target.children.find(n=>n.className==='tmdb-candidates').children.find(n=>n.tagName==='BUTTON').onclick();await new Promise(resolve=>setImmediate(resolve));
 active=false;target.replaceChildren();target.textContent='Different detail';release({saved:true});await pending;assert.equal(target.textContent,'Different detail');assert.equal(target.children.length,0);assert.equal(reloaded,0);
});
