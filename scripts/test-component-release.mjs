import {test} from "node:test";import assert from "node:assert/strict";import {manifestFor,validateAssetInput,isComponentReleaseTag} from "./component-release.mjs";
test("release ledger treats component names as fixed identifiers",()=>{
 for(const component of ["mac","server","web","themes"])assert.equal(isComponentReleaseTag(component,component+"-v0.7.1"),true);
 for(const [component,tag] of [[".*","mac-v0.7.1"],["server","server-v0.7.1-rc1"],["server","server-v0.7.1\n"],["server","mac-v0.7.1"],["server",null],["(a+)+$","a".repeat(10000)]])assert.equal(isComponentReleaseTag(component,tag),false);
});
const base={component:"server",version:"0.7.0",sequence:2,previousSequence:1,source:"a".repeat(40),assets:[{target:"linux-aarch64",url:"https://github.com/ungeekrange-cmyk/chillcenter/releases/download/server-v0.7.0/server",byte_length:1,sha256:"b".repeat(64),format:"raw"}],keyId:"18FD790B48C202F8",now:new Date("2026-10-19T02:00:00Z")};
test("component manifest binds independent version, source and rollback schema",()=>{const m=manifestFor(base);assert.equal(m.release_tag,"server-v0.7.0");assert.equal(m.compatibility.database.rollback_max,3);assert.equal(m.compatibility.api.min,"2026.10.14");assert.equal(m.source_commit,base.source);assert.ok(Date.parse(m.expires_at)>Date.parse(m.issued_at));});
test("theme and web compatibility is explicit",()=>{assert.equal(manifestFor({...base,component:"themes"}).compatibility.renderer.theme_format,1);assert.equal(manifestFor({...base,component:"web"}).compatibility.required_server.min,"0.7.0");});
test("annual-event packs require renderer2 without admitting old clients",()=>{
 assert.deepEqual(manifestFor({...base,component:"themes",themeRenderer:2}).compatibility.renderer,{theme_format:1,renderer_min:2,renderer_max:2});
 assert.throws(()=>manifestFor({...base,component:"themes",themeRenderer:0}));
});
test("invalid identity is refused",()=>{for(const bad of [{component:"shell"},{version:"latest"},{source:"HEAD"},{assets:[]}])assert.throws(()=>manifestFor({...base,...bad}));});
test("equal, decreasing or clock-derived missing sequences cannot be signed",()=>{for(const sequence of [undefined,0,1,-1,1.5,Number.MAX_SAFE_INTEGER+1])assert.throws(()=>manifestFor({...base,sequence}));assert.equal(manifestFor({...base,now:new Date("2020-01-01Z")}).release_sequence,2,"A corrected clock cannot decrease a reserved sequence");});
test("wrong payload type and target are rejected before signing",()=>{
 assert.throws(()=>validateAssetInput("mac","darwin-aarch64",Buffer.from("a private key or text file")));
 assert.throws(()=>validateAssetInput("server","linux-aarch64",Buffer.from("#!/bin/sh")));
 assert.throws(()=>validateAssetInput("themes","universal",Buffer.from("not a zip")));
 assert.throws(()=>validateAssetInput("server","unit-shell",Buffer.from("[Unit]")));
 assert.doesNotThrow(()=>validateAssetInput("server","unit-chillcenter-service",Buffer.from("[Unit]\nDescription=Fixture")));
});
