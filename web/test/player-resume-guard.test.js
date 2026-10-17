"use strict";
const test=require("node:test"),assert=require("node:assert/strict");
const {WebPlayer}=require("../src/player-web.js");

function fixture(request){
 let onPause,plays=0;
 const engine={pause(){},play(){plays++;return true;},close(){},snapshot:()=>({paused:true,loaded:true,position:0,duration:10,tracks:[]})};
 class Events{addEventListener(name,callback){if(name==="pause")onPause=callback;}close(){}}
 const player=new WebPlayer({request,EventSource:Events,clearInterval(){},now:()=>0});
 player.ctx={profile:"fixture"};player.info={media:{title:"Fixture"}};player.engine=engine;
 player.lease={session_id:"session",generation:1,media_id:"media",media_revision:1};
 player.observe=()=>({});
 return{player,get plays(){return plays;},pause:event=>onPause(event)};
}

test("server pause blocks both resume paths before a delayed transfer ACK completes",async()=>{
 let acknowledge;
 const ack=new Promise(resolve=>{acknowledge=resolve;});
 const f=fixture(async(method,path)=>path.endsWith("/transfer/ack")?ack:{});
 f.player.openEvents();
 const pending=f.pause({data:JSON.stringify({token:"transfer"})});
 assert.equal(f.player.serverPaused,true);
 assert(f.player.lease,"ACK is still pending, so the guard cannot rely on a removed lease");
 assert.equal(f.player.resume(),false);
 await f.player.command({action:"resume"},f.player.ctx);
 assert.equal(f.plays,0);
 acknowledge({});await pending;
 assert.equal(f.player.engine,null);assert.equal(f.player.lease,null);
});

test("an invalid lease cannot be restarted by a fresh touch or the regular control",async()=>{
 const f=fixture(async()=>{throw Object.assign(new Error("expired"),{code:"LEASE_INVALID",status:409});});
 await f.player.heartbeat();assert.equal(f.player.serverPaused,true);
 assert.equal(f.player.resume(),false);await f.player.command({action:"resume"},f.player.ctx);
 assert.equal(f.plays,0);
});

test("a valid existing lease resumes without opening another stream",async()=>{
 const f=fixture(async()=>{throw new Error("resume must not request anything");});
 assert.equal(f.player.resume(),true);assert.equal(f.plays,1);
});
