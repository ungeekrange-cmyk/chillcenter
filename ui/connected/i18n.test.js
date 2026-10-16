"use strict";
const test=require("node:test"),assert=require("node:assert/strict"),fs=require("node:fs"),vm=require("node:vm");
const translations=require("./locales.js"),{resolve}=require("./i18n.js");
test("automatic uses French only for a French primary system locale, otherwise English",()=>{
 for(const system of ["fr","fr-FR","fr_CA","fr-BE"])assert.equal(resolve("auto",system),"fr");
 for(const system of ["en-US","de-DE","es","","af-FR"])assert.equal(resolve("auto",system),"en");
 assert.equal(resolve("en","fr-FR"),"en");assert.equal(resolve("fr","en-US"),"fr");
});
test("translation catalogue covers the primary UI and preserves opaque metadata",()=>{
 assert.equal(translations["Toute la collection"],"Entire library");assert.equal(translations["Sous-titres"],"Subtitles");
 const context={chillEnglish:translations};vm.runInNewContext(fs.readFileSync(__dirname+"/i18n.js","utf8"),context);
 assert.equal(context.chillI18n.t("my-original-title"),"my-original-title");
 assert.equal(context.chillI18n.t("toString"),"toString");
 assert.equal(context.chillI18n.t("{0} fiche(s) mises à jour.",7),"7 metadata entries updated.");
 for(const value of Object.values(translations))assert.equal(typeof value,"string");
});
test("manual subtitle network download is spawned, cancelled on newer choice, and never pauses or reloads video",()=>{
 const source=fs.readFileSync(__dirname+"/../../apps/desktop/src/player_bridge.rs","utf8");
 const queue=source.slice(source.indexOf("async fn queue_external"),source.indexOf("fn matching_external"));
 assert.ok(queue.indexOf("task.abort()")<queue.indexOf("subtitle_cache.iter()"));
 assert.ok(queue.indexOf("tokio::spawn(async move")<queue.indexOf("download_subtitle"));
 assert.doesNotMatch(queue,/NativeAction::Pause|native_player::load|release\(s\)/);
 const select=source.slice(source.indexOf("PlayerCommand::Subtitle { track_id } =>"),source.indexOf("PlayerCommand::Fullscreen { enabled } =>"));
 assert.match(select,/task.abort\(\)/);assert.doesNotMatch(select,/NativeAction::Pause|native_player::load/);
});
