"use strict";
const { test } = require("node:test"), assert = require("node:assert/strict");
const { readFileSync } = require("node:fs"), { resolve } = require("node:path");
const source = readFileSync(resolve(__dirname, "../src/mobile-ui.js"), "utf8");
test("mobile adapter forwards to shared navigation instead of duplicating shell selectors", () => {
  assert.match(source, /data-mobile-category/);
  assert.match(source, /forward\(`\[data-category=/);
  assert.match(source, /forward\("#show-sagas"\)/);
  assert.match(source, /forward\("#admin-toggle"\)/);
  assert.match(source, /data-mobile-category=\"\$\{category\}\"/);
});
test("mobile virtualizer computes three columns from its live available width", () => {
  assert.match(source, /\(width - 36\) \/ 3/);
  assert.match(source, /root\.innerWidth <= 360/);
  assert.match(source, /root\.catalogWindow = \(count, width, posterSize/);
});
test("mobile event decoration has no hidden desktop sidebar reservation", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.app-shell\{display:block;min-height:100dvh;--event-sidebar-width:0px\}/);
});
test("bottom navigation is resynchronised after player class transitions", () => {
  assert.match(source, /new MutationObserver/);
  assert.match(source, /doc\.body\.classList\.contains\("player-open"\)/);
  assert.match(source, /attributeFilter: \["class"\]/);
  assert.match(source, /syncShell\(\);/);
});
test("mobile sheet keeps web AI absent and provides the four approved destinations", () => {
  for (const label of ["Toute la collection", "Sagas", "Listes de lecture", "Configuration"]) assert.match(source, new RegExp(label));
  assert.doesNotMatch(source, /Demande de conseil/);
});
test("phone header retains the approved logo while the desktop sidebar is hidden", () => {
  assert.match(source, /id = "mobile-brand"/);
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.mobile-brand/); assert.match(css, /\.sidebar\{display:none\}/);
});
test("web-only labels have an English fallback and More is selected for the library root", () => {
  assert.match(source, /"Plus": "More"/);
  assert.match(source, /"Docs": "Documentaries"/);
  assert.match(source, /category === ""/);
  assert.match(source, /aria-current/);
});
test("poster actions keep separate 44px touch targets and mobile header keeps profile at right", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.poster-watched\{left:5px;bottom:5px\}/);
  assert.match(css, /\.poster-add\{right:5px;bottom:5px\}/);
  assert.match(css, /min-width:44px;min-height:44px/);
  assert.match(css, /grid-template-columns:44px minmax\(0,1fr\) 44px max-content/);
  assert.match(css, /\.profile-menu\{grid-column:4/);
});
test("collection member actions are visible and touch-safe without changing virtual row info height", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.collection-media-card \.actions\{position:absolute/);
  assert.match(css, /bottom:104px/);
  assert.match(css, /overflow-x:auto/);
  assert.match(css, /min-width:44px;min-height:44px/);
});
test("remaining mobile player and profile controls meet the 44px touch target", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.profile-menu summary\{gap:6px;min-width:44px;min-height:44px/);
  assert.match(css, /#mobile-orientation-hint button\{min-height:44px/);
  assert.match(css, /\.player-seek input\{min-height:44px/);
});
