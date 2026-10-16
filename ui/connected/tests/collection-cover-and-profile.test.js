const test=require("node:test"),assert=require("node:assert/strict"),fs=require("node:fs");
const html=fs.readFileSync(__dirname+"/../index.html","utf8");
const catalog=fs.readFileSync(__dirname+"/../catalog.js","utf8");
const app=fs.readFileSync(__dirname+"/../app.js","utf8");

test("collection covers are a bounded nine-poster mosaic unless a custom image exists",()=>{
  assert.match(catalog,/collection\.media_ids\.slice\(0,9\)/);
  assert.match(catalog,/if\(collection\.poster\)\{observeArtwork\(poster,collection\.id,kind\);return;\}/);
  assert.match(catalog,/importArtwork\(\{kind,id:view\.id,expected_revision:view\.revision\}/);
  assert.match(catalog,/action:"playlist_artwork",id/);
});

test("profile header keeps the name beside an initial avatar and exposes change/create controls",()=>{
  assert.match(html,/id="profile-name"/);
  assert.match(html,/id="profile-avatar"/);
  assert.match(html,/id="create-profile-menu"/);
  assert.match(app,/activeName\.trim\(\)\.slice\(0,1\)\.toLocaleUpperCase\(\)/);
  assert.match(app,/\["create-first","create-admin","create-profile-menu"\]/);
});

test("next-episode preferences include 15 seconds and cancelling closes the player",()=>{
  assert.match(app,/new Option\(uiText\("Après 15 secondes"\),"15"\)/);
  const player=fs.readFileSync(__dirname+"/../player.js","utf8");
  assert.match(player,/async function cancelNextAndClose\(\)/);
  assert.match(player,/await returnFromPlayback\(\)/);
  assert.match(player,/await stopPlayerForNavigation\(current\)/);
  assert.match(player,/\$\("player-next-cancel"\)\.onclick=cancelNextAndClose/);
});
