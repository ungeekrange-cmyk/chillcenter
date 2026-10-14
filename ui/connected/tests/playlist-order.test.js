"use strict";
const assert=require("node:assert/strict");
const fs=require("node:fs"),vm=require("node:vm");
const source=fs.readFileSync(require.resolve("../catalog.js"),"utf8");
const start=source.indexOf("function playlistOrderForVisibleMove");
const end=source.indexOf("\nif(typeof module",start);
assert.notEqual(start,-1,"the playlist-order helper must be present");
const playlistOrderForVisibleMove=vm.runInNewContext(source.slice(start,end)+";playlistOrderForVisibleMove");

const canonical=["first","hidden-middle","last"];
const visible=["first","last"];

const movedUp=Array.from(playlistOrderForVisibleMove(canonical,visible,"last",-1));
assert.deepEqual(movedUp,["last","hidden-middle","first"]);
assert.deepEqual([...movedUp].sort(),[...canonical].sort(),"a filtered move must not lose membership");
assert.equal(movedUp[1],"hidden-middle","the hidden member stays in the canonical permutation");

const movedDown=Array.from(playlistOrderForVisibleMove(canonical,visible,"first",1));
assert.deepEqual(movedDown,["last","hidden-middle","first"]);
assert.equal(playlistOrderForVisibleMove(canonical,visible,"first",-1),null,"the first visible member cannot move above the filtered view");
assert.equal(playlistOrderForVisibleMove(canonical,visible,"last",1),null,"the last visible member cannot move below the filtered view");
