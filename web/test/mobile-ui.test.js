"use strict";
const { test } = require("node:test"), assert = require("node:assert/strict");
const { readFileSync } = require("node:fs"), { resolve } = require("node:path");
const source = readFileSync(resolve(__dirname, "../src/mobile-ui.js"), "utf8");
test("mobile adapter forwards to shared navigation instead of duplicating shell selectors", () => {
  assert.match(source, /data-mobile-category/);
  assert.match(source, /forward\(`\[data-category=/);
  for (const target of ["#show-home", "#show-downloads", "#show-sagas", "#admin-toggle"]) {
    assert.ok(source.includes(`"${target}"`));
  }
});
test("mobile virtualizer computes three columns from its live available width", () => {
  assert.match(source, /width - \(columns - 1\) \* 18/);
  assert.match(source, /root\.innerWidth <= 360/);
  assert.match(source, /root\.catalogWindow = \(count, width, posterSize/);
});
test("fractional available widths cannot randomly drop a mobile poster column", () => {
  const {runInNewContext}=require("node:vm"),{catalogWindow}=require("../../ui/connected/catalog-window.js");
  const install=source.match(/function installVirtualizer\(\) \{[\s\S]*?\n  \}/)[0];
  const root={catalogWindow,innerWidth:430};runInNewContext(`(${install})()`,{root,isPhone:()=>true});
  for(let width=280;width<600;width+=1/64)assert.equal(root.catalogWindow(100,width,160,0,844,false).columns,3,`width ${width}`);
  root.innerWidth=320;for(let width=200;width<400;width+=1/64)assert.equal(root.catalogWindow(100,width,160,0,844,false).columns,2,`width ${width}`);
  assert.equal(root.catalogWindow(100,366.03125,160,0,844,true).columns,1,"list layout is unchanged");
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
test("mobile sheet contains the secondary destinations", () => {
  for (const label of ["Documentaires", "Spectacles", "Toute la collection", "Sagas", "Listes de lecture", "Configuration"]) assert.match(source, new RegExp(label));
  assert.doesNotMatch(source, /Demande de conseil/);
});
test("phone header retains the approved logo while the desktop sidebar is hidden", () => {
  assert.match(source, /id = "mobile-brand"/);
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.mobile-brand/); assert.match(css, /\.sidebar\{display:none\}/);
});
test("web-only labels have an English fallback and More is selected for the library root", () => {
  assert.match(source, /"Plus": "More"/);
  assert.match(source, /"Téléchargés": "Downloads"/);
  assert.match(source, /: category === "film" \? "films" : "more"/);
  assert.match(source, /aria-current/);
});
test("mobile catalogue exposes compact playback controls and keeps profile at right", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.media-card\.catalog-open \.poster-actions\{opacity:1;background:none\}/);
  assert.doesNotMatch(css, /\.media-card \.poster-actions\{inset:0;display:block/);
  assert.match(css, /grid-template-columns:44px minmax\(0,1fr\) 44px 44px max-content/);
  assert.match(css, /\.profile-menu\{grid-column:5/);
});
test("collection member actions stay below posters with touch targets and matching row geometry", () => {
  const css = readFileSync(resolve(__dirname, "../../ui/connected/collections.css"), "utf8");
  assert.match(css, /#collection-members \.collection-media-card \.info\{height:168px/);
  assert.match(css, /\.actions\{position:static;display:grid;grid-template-columns:repeat\(2,minmax\(44px,1fr\)\)/);
  assert.match(css, /min-width:44px;min-height:44px/);
  const {runInNewContext}=require("node:vm"),{catalogWindow}=require("../../ui/connected/catalog-window.js");
  const source=readFileSync(resolve(__dirname,"../src/mobile-ui.js"),"utf8"),install=source.slice(source.indexOf("function installVirtualizer()"),source.indexOf("\n  function install()"));
  const root={catalogWindow,innerWidth:375};runInNewContext(`(${install})()`,{root,isPhone:()=>true});
  const w=root.catalogWindow(100,343,200,0,812,false,168);
  assert.equal(w.columns,3);assert.equal(w.gridRowHeight,Math.ceil(((343-36)/3)*1.5)+168);assert.equal(w.gridRowHeight+18,w.rowHeight);
  for(let width=160;width<=400;width++){const row=root.catalogWindow(100,width,200,0,812,false,168),card=(width-(row.columns-1)*18)/row.columns;assert.ok(card>=96,`collection controls require at least96px, got ${card}`);}
});
test("remaining mobile player and profile controls meet the 44px touch target", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.profile-menu summary\{gap:6px;min-width:44px;min-height:44px/);
  assert.match(css, /#mobile-orientation-hint button\{min-height:44px/);
  assert.match(css, /\.player-seek input\{min-height:44px/);
});
test("content-first mobile search and filters are opened by an accessible small control", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(source, /mobile-search-toggle/);
  assert.match(source, /aria-expanded/);
  assert.match(source, /aria-controls/);
  assert.match(source, /event\.key === "Escape"/);
  assert.match(css, /\.app-topbar \.global-search\{display:none/);
  assert.match(css, /\.mobile-discovery-open #filter-form\{display:flex\}/);
});
test("web-only toggle is absent from desktop and landscape layout remains phone-scoped", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /\.mobile-brand,\.mobile-nav,\.mobile-search-toggle\{display:none\}/);
  assert.match(css, /orientation:landscape\) and \(max-height:500px\) and \(pointer:coarse\)/);
  assert.match(css, /text-overflow:ellipsis/);
});
test("every mobile forwarding destination closes discovery without clearing its values", () => {
  assert.match(source, /if \(source && !source\.hidden && !source\.disabled\) \{\s*closeSheet\(\);\s*setDiscoveryOpen\(false\);\s*source\.click\(\);/);
  assert.match(source, /searchToggle\.hidden = !active/);
});
test("closing discovery survives a desktop transition and never hides picker filters", () => {
  assert.match(source, /if \(open && !isPhone\(\)\) return/);
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /#filter-form\{display:none/);
  assert.doesNotMatch(css, /\.filters\{display:none/);
});
test("landscape navigation stays at the bottom, with dynamic columns", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /orientation:landscape/);
  assert.match(css, /grid-auto-flow:column;grid-auto-columns:minmax\(0,1fr\)/);
  assert.doesNotMatch(css, /\.mobile-nav\{top:0;bottom:0;left:0;right:auto/);
});
test("portrait header assigns distinct columns to sync, search and profile", () => {
  const css = readFileSync(resolve(__dirname, "../src/mobile.css"), "utf8");
  assert.match(css, /grid-template-columns:44px minmax\(0,1fr\) 44px 44px max-content/);
  assert.match(css, /#sync-indicator\{grid-column:3/);
  assert.match(css, /\.mobile-search-toggle\{display:grid;grid-column:4/);
  assert.match(css, /\.profile-menu\{grid-column:5/);
});
