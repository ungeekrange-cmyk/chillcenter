const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
test('admin navigation starts nonblocking sync independently of sequential admin reads',async()=>{
 const source=fs.readFileSync(__dirname+'/app.js','utf8'),calls=[],events=[],panel={hidden:true};
 const toggle={addEventListener(_,fn){events.push(fn);}},get=id=>id==='admin-toggle'?toggle:panel;
 let busy=false;
 const read=async command=>{if(busy){calls.push('SKIPPED:'+command.action);return null;}busy=true;calls.push(command.action);await new Promise(resolve=>setImmediate(resolve));busy=false;return {stage:'connected'};};
 const sandbox={$:get,state:{server_id:'s',profile_id:'p'},shellSection:'catalog',syncCatalog:()=>{calls.push('background_sync');return new Promise(()=>{});},stopPlayerForNavigation:async()=>true,run:read,pollScan:()=>read({action:'scan_status'}),unavailableAdmin:()=>read({action:'unavailable'}),window:{refreshTmdbStatus:()=>read({action:'tmdb_status'})}};
 sandbox.shellShow=section=>{sandbox.shellSection=section;};
 vm.createContext(sandbox);vm.runInContext(source.match(/\$\("admin-toggle"\)\.onclick=[^\n]*/)[0],sandbox);
 const oldSecondary=source.match(/\$\("admin-toggle"\)\.addEventListener\("click",\(\)=>unavailableAdmin\(\)\);/);if(oldSecondary)vm.runInContext(oldSecondary[0],sandbox);
 const primary=toggle.onclick();for(const fn of events)fn();await primary;await Promise.resolve();
 assert.equal(panel.hidden,false,'Administration never opened');assert.deepEqual(calls,['background_sync','scan_status','unavailable','tmdb_status']);
});
