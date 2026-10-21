"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Json64 = require("../src/json64.js");
const { createBridge, resolveAppearance, BASE } = require("../src/bridge.js");
const { WatchClock, finalTenthStart } = require("../src/watch-clock.js");

const P = "11111111-1111-4111-8111-111111111111";
const M = "22222222-2222-4222-8222-222222222222";

function memory() { const m = new Map(); return { getItem: k => m.get(k) ?? null, setItem: (k, v) => m.set(k, v) }; }
function server(routes) {
  const calls = [];
  const fetch = async (url, init = {}) => {
    // The retired alias must never leave the shipped browser adapter, including
    // requests made after reading old catalogue/poster references from storage.
    assert.ok(url.startsWith(BASE + "/"), `non-canonical API request: ${url}`);
    calls.push({ url, method: init.method || "GET", body: init.body, headers: init.headers });
    const key = (init.method || "GET") + " " + url.replace(BASE, "").split("?")[0];
    const r = routes[key];
    if (!r) return { ok: false, status: 404, text: async () => '{"code":"NOT_FOUND"}' };
    const [status, body, extra] = await Promise.resolve(typeof r === "function" ? r(url, init) : r);
    return { ok: status < 400, status, text: async () => (typeof body === "string" ? body : JSON.stringify(body)), blob: async () => extra };
  };
  return { fetch, calls };
}
const ok = body => [200, body];
const baseRoutes = {
  "GET /server": ok({ server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"], tls: {} }),
  "GET /profiles": ok([{ id: P, name: "A", revision: 1 }]),
  "GET /admin/library": ok({ video_root: null, revision: 0 }),
  [`GET /profiles/${P}/preferences`]: ok({ audio_language: "en", subtitle_language: "en", autoplay_seconds: 10 }),
};
const make = (routes = {}) => { const s = server({ ...baseRoutes, ...routes }); const b = createBridge({ fetch: s.fetch, storage: memory(), objectUrl: () => "blob:x", address: "pi:8443" }); return { ...s, b, ui: command => b.invoke("ui_command", { command }) }; };

test("json64 keeps uint64 exact and leaves strings alone", () => {
  const text = '{"media_revision":18446744073709551615,"t":"123456789012345678901","n":12,"f":1.5}';
  const v = Json64.parse(text);
  assert.equal(v.media_revision, 18446744073709551615n);
  assert.equal(v.t, "123456789012345678901");
  assert.equal(Json64.stringify(v), '{"media_revision":18446744073709551615,"t":"123456789012345678901","n":12,"f":1.5}');
});

test("boot connects same-origin, restores nothing without a saved profile", async () => {
  const { ui } = make();
  const v = await ui({ action: "boot" });
  assert.equal(v.stage, "connected"); assert.equal(v.server_id, "srv"); assert.equal(v.profile_id, null); assert.equal(v.profiles.length, 1);
});

test("component updates use dated same-origin closed intent bodies only",async()=>{
 const {b,calls}=make({"POST /admin/updates/check":ok({operation:{status:"pending"}}),"POST /admin/updates/install":ok({operation:{status:"pending"}}),"PUT /admin/updates/policy":ok({policy:{revision:2}})});
 for(const component of ['server','web','themes']){
  await b.invoke('update_command',{command:{action:'server_check',component,url:'https://untrusted.invalid/'}});
  assert.equal(calls.at(-1).url,BASE+'/admin/updates/check');
  assert.deepEqual(JSON.parse(calls.at(-1).body),{component});
 }
 await b.invoke("update_command",{command:{action:"server_install",component:"web",force:true,url:"https://untrusted.invalid/",confirmed:true}});
 assert.equal(calls.at(-1).url,BASE+"/admin/updates/install");
 assert.deepEqual(JSON.parse(calls.at(-1).body),{component:"web"});
 await b.invoke("update_command",{command:{action:"server_policy",component:"themes",mode:"check",hour:2,minute:0,expected_revision:1,path:"/not/accepted"}});
 assert.deepEqual(JSON.parse(calls.at(-1).body),{component:"themes",mode:"check",hour:2,minute:0,expected_revision:1});
 const count=calls.length;
 await assert.rejects(b.invoke("update_command",{command:{action:"server_install",component:"mac"}}));
 assert.equal(calls.length,count);
});

test("old-server update fallback keeps actual identity; transient failures stay errors",async()=>{
 const old=make({"GET /server":ok({version:"0.6.0",api_version:"2026-10-14"})});
 assert.deepEqual(await old.b.invoke("update_command",{command:{action:"status"}}),{unsupported:true,installed:{server:{version:"0.6.0"},api:"2026-10-14"}});
 const busy=make({"GET /admin/updates":[503,{code:"BUSY"}]});
 await assert.rejects(busy.b.invoke("update_command",{command:{action:"status"}}),e=>e.status===503);
 assert.equal(busy.calls.length,1,"503 must not be hidden as an unsupported server");
});

test("boot refuses a server without required capabilities, without leaving a connection", async () => {
  const { ui } = make({ "GET /server": ok({ server_id: "old", capabilities: ["profiles"] }) });
  const v = await ui({ action: "boot" });
  assert.equal(v.stage, "server"); assert.match(v.notice, /capacités manquantes/);
  await assert.rejects(ui({ action: "catalog", offset: 0 }), /profil/i);
});

test("profile choice is remembered per server and a failed switch keeps the previous one", async () => {
  const { ui, b } = make();
  await ui({ action: "boot" });
  const v = await ui({ action: "select_profile", id: P });
  assert.equal(v.profile_id, P);
  assert.equal(b._state().settings.last_profile.srv, P);
  await assert.rejects(ui({ action: "select_profile", id: M }), /Profil absent/);
  assert.equal(b._state().conn.active, P);
  const again = make();
  again.b._state().settings.last_profile.srv = P;
  assert.equal((await again.ui({ action: "boot" })).profile_id, P);
});

test("catalog query is allow-listed and paged at 48", async () => {
  const { ui, calls } = make({ [`GET /profiles/${P}/catalog`]: ok({ items: [], has_more: false }) });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  await ui({ action: "catalog", offset: 96, category: "serie", q: "dune", sort: "year_desc", watched: false, year: 0 });
  const url = calls.at(-1).url;
  assert.match(url, /offset=96&limit=48&category=serie&q=dune&year=0&watched=false&sort=year_desc$/);
  await assert.rejects(ui({ action: "catalog", offset: 0, category: "x" }), /Catégorie invalide/);
  await assert.rejects(ui({ action: "catalog", offset: 0, sort: "drop" }), /Tri invalide/);
});

test("mutations keep strict bodies; delete profile sends the exact confirmation header", async () => {
  const { ui, calls } = make({ [`DELETE /profiles/${P}`]: [204, ""], [`PUT /profiles/${P}/media/${M}`]: ok({ revision: 5 }) });
  await ui({ action: "boot" });
  await ui({ action: "delete_profile", id: P, confirm_id: P });
  assert.equal(calls.find(c => c.method === "DELETE").headers["x-chill-confirm-delete"], P);
  await ui({ action: "select_profile", id: P }).catch(() => {});
  const { ui: ui2, calls: c2 } = make({ [`PUT /profiles/${P}/media/${M}`]: ok({}) });
  await ui2({ action: "boot" }); ui2({ action: "select_profile", id: P }); await ui2({ action: "select_profile", id: P });
  await ui2({ action: "edit_media", media_id: M, expected_revision: 4, patch: { title: "T" } });
  assert.deepEqual(JSON.parse(c2.at(-1).body), { title: "T", expected_revision: 4 });
  await assert.rejects(ui2({ action: "edit_media", media_id: M, expected_revision: 4, patch: { path: "/etc" } }), /Champs de fiche invalides/);
});

test("API errors become French messages with status and code", async () => {
  const { ui } = make({ "POST /profiles": [409, { code: "PROFILE_NAME_TAKEN" }] });
  await ui({ action: "boot" });
  await assert.rejects(ui({ action: "create_profile", name: "A" }), e => e.status === 409 && e.code === "PROFILE_NAME_TAKEN" && /déjà utilisé/.test(e.message));
});

test("artwork becomes a blob URL once; TMDB status uses the explicit API", async () => {
  const { ui, calls } = make({ [`GET /profiles/${P}/media/${M}/artwork`]: [200, "", { size: 10 }] });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  assert.deepEqual(await ui({ action: "artwork", media_id: M }), { src: "blob:x" });
  await ui({ action: "artwork", media_id: M });
  assert.equal(calls.filter(c => c.url.endsWith("/artwork")).length, 1);
  const tmdb = make({ "GET /admin/tmdb": ok({ configured: false }) }); await tmdb.ui({ action: "boot" });
  assert.deepEqual(await tmdb.ui({ action: "tmdb_status" }), { configured: false });
});

test("legacy poster references do not become legacy network requests", async () => {
  const { ui, calls } = make({
    [`GET /profiles/${P}/media/${M}`]: ok({ media: { id: M, title: "Cached title", poster: `/api/v1/profiles/${P}/media/${M}/artwork` }, revision: 1 }),
    [`GET /profiles/${P}/media/${M}/artwork`]: [200, "", { size: 10 }],
    [`GET /profiles/${P}/series/123/seasons/1/artwork`]: [200, "", { size: 10 }],
  });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  await ui({ action: "media_detail", media_id: M });
  await ui({ action: "artwork", media_id: M });
  await ui({ action: "season_artwork", tv_id: 123, season: 1 });
  assert.ok(calls.every(c => c.url.startsWith("/api/2026-10-14/")));
});

test("delayed artwork cannot republish after a mutation or profile switch", async () => {
  const B = "33333333-3333-4333-8333-333333333333";
  let releaseFirst, releaseSecond, imageCalls = 0;
  const first = new Promise(resolve => { releaseFirst = resolve; });
  const second = new Promise(resolve => { releaseSecond = resolve; });
  const urls = [];
  const s = server({ ...baseRoutes,
    "GET /profiles": ok([{ id: P, name: "A", revision: 1 }, { id: B, name: "B", revision: 1 }]),
    [`GET /profiles/${B}/preferences`]: ok({}),
    [`GET /profiles/${P}/media/${M}/artwork`]: () => [200, "", [first, { size: 10, label: "after-mutation" }, second, { size: 10, label: "after-switch" }][imageCalls++]],
    [`PUT /profiles/${P}/media/${M}/watched`]: ok({}),
  });
  const b = createBridge({ fetch: s.fetch, storage: memory(), objectUrl: blob => { urls.push(blob.label); return `blob:${blob.label}`; } });
  const ui = command => b.invoke("ui_command", { command });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });

  const oldAfterMutation = ui({ action: "artwork", media_id: M });
  await new Promise(resolve => setImmediate(resolve));
  await ui({ action: "set_watched", media_id: M, watched: true });
  releaseFirst({ size: 10, label: "stale-mutation" });
  await assert.rejects(oldAfterMutation, e => e.code === "ARTWORK_DISCARDED");
  assert.deepEqual(await ui({ action: "artwork", media_id: M }), { src: "blob:after-mutation" });

  await ui({ action: "select_profile", id: B }); await ui({ action: "select_profile", id: P });
  const oldAfterSwitch = ui({ action: "artwork", media_id: M });
  await new Promise(resolve => setImmediate(resolve));
  await ui({ action: "select_profile", id: B });
  releaseSecond({ size: 10, label: "stale-switch" });
  await assert.rejects(oldAfterSwitch, e => e.code === "ARTWORK_DISCARDED");
  await ui({ action: "select_profile", id: P });
  assert.deepEqual(await ui({ action: "artwork", media_id: M }), { src: "blob:after-switch" });
  assert.deepEqual(urls, ["after-mutation", "after-switch"], "stale responses never create or cache blob URLs");
});

test("artwork of sagas and playlists, and the player presentation setting, are supported", async () => {
  const SAGA = "saga_1-A";
  const { ui, calls } = make({
    [`GET /profiles/${P}/sagas/${SAGA}/artwork`]: [200, "", { size: 10 }],
    [`GET /profiles/${P}/playlists/${M}/artwork`]: [200, "", { size: 10 }],
  });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  assert.deepEqual(await ui({ action: "saga_artwork", id: SAGA }), { src: "blob:x" });
  assert.deepEqual(await ui({ action: "playlist_artwork", id: M }), { src: "blob:x" });
  await ui({ action: "saga_artwork", id: SAGA });
  assert.equal(calls.filter(c => c.url.endsWith("/artwork")).length, 2, "each artwork fetched once");
  await assert.rejects(ui({ action: "saga_artwork", id: "../x" }), /Identifiant de saga invalide/);
  await assert.rejects(ui({ action: "playlist_artwork", id: "not-a-uuid" }), /Identifiant invalide/);
  assert.equal((await ui({ action: "refresh" })).player_presentation.fullscreen_on_start, true, "default: fullscreen at launch");
  assert.deepEqual(await ui({ action: "save_player_settings", fullscreen_on_start: false }), { player_presentation: { fullscreen_on_start: false } });
  assert.equal((await ui({ action: "refresh" })).player_presentation.fullscreen_on_start, false, "persisted and returned in the state");
  await assert.rejects(ui({ action: "save_player_settings", fullscreen_on_start: "false" }), /Réglage du lecteur invalide/);
});

test("revision sync is context-scoped, cheap after its first check, and invalidates artwork", async () => {
  let revision = "epoch:1";
  let revokes = 0, blobs = 0;
  const s = server({ ...baseRoutes,
    [`GET /profiles/${P}/catalog/revision`]: () => ok({ revision }),
    [`GET /profiles/${P}/media/${M}/artwork`]: () => [200, "", { size: 10 }],
    [`PUT /profiles/${P}/media/${M}/watched`]: ok({}),
  });
  const b = createBridge({ fetch: s.fetch, storage: memory(), objectUrl: () => `blob:${++blobs}`, revoke: () => revokes++ });
  const invoke = command => b.invoke("ui_command", { command });
  await invoke({ action: "boot" }); await invoke({ action: "select_profile", id: P });
  const first = await invoke({ action: "sync_catalog" });
  assert.equal(first.changed, true);
  assert.deepEqual(await invoke({ action: "ack_catalog_sync", ticket: first.ack_ticket }), { discarded: false });
  assert.equal((await invoke({ action: "sync_catalog" })).changed, false);
  assert.equal(s.calls.filter(c => c.url.includes("/catalog/revision")).length, 2, "revision checks transfer no catalog page");
  await invoke({ action: "artwork", media_id: M });
  revision = "epoch:2";
  assert.equal((await invoke({ action: "sync_catalog" })).changed, true);
  assert.equal(revokes, 1, "a changed revision revokes the old detail image");
  await invoke({ action: "artwork", media_id: M });
  await invoke({ action: "set_watched", media_id: M, watched: true });
  assert.equal(revokes, 2, "a contextual mutation revokes the replacement detail image");
});

test("an unapplied revision retries after failed catalog pages; old tickets cannot acknowledge newer checks", async () => {
  let revision = "R1", catalogFails = false;
  const { ui } = make({
    [`GET /profiles/${P}/catalog/revision`]: () => ok({ revision }),
    [`GET /profiles/${P}/catalog`]: () => catalogFails ? [503, { code: "BUSY" }] : ok({ items: [], has_more: false }),
  });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  const first = await ui({ action: "sync_catalog" });
  await ui({ action: "ack_catalog_sync", ticket: first.ack_ticket });
  revision = "R2";
  const failed = await ui({ action: "sync_catalog" });
  catalogFails = true;
  await assert.rejects(ui({ action: "catalog", offset: 0 }), e => e.status === 503);
  // The UI did not acknowledge because a catalog page failed.
  const retry = await ui({ action: "sync_catalog" });
  assert.equal(failed.changed, true); assert.equal(retry.changed, true);
  assert.deepEqual(await ui({ action: "ack_catalog_sync", ticket: failed.ack_ticket }), { discarded: true });
  catalogFails = false;
  await ui({ action: "catalog", offset: 0 });
  await ui({ action: "ack_catalog_sync", ticket: retry.ack_ticket });
  assert.equal((await ui({ action: "sync_catalog" })).changed, false);
  revision = "R3";
  const beforeMutation = await ui({ action: "sync_catalog" });
  await ui({ action: "disconnect" });
  assert.deepEqual(await ui({ action: "ack_catalog_sync", ticket: beforeMutation.ack_ticket }), { discarded: true });
});

test("late profile A revision checks are discarded and older servers request an explicit full refresh", async () => {
  const B = "33333333-3333-4333-8333-333333333333";
  let resolveA;
  const delayedA = new Promise(resolve => { resolveA = resolve; });
  const s = server({ ...baseRoutes,
    "GET /profiles": ok([{ id: P, name: "A", revision: 1 }, { id: B, name: "B", revision: 1 }]),
    [`GET /profiles/${B}/preferences`]: ok({}),
    [`GET /profiles/${P}/catalog/revision`]: () => delayedA,
    [`GET /profiles/${B}/catalog/revision`]: ok({ revision: "B:1" }),
  });
  const b = createBridge({ fetch: s.fetch, storage: memory(), objectUrl: () => "blob:x" });
  const ui = command => b.invoke("ui_command", { command });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  const stale = ui({ action: "sync_catalog" });
  const switched = ui({ action: "select_profile", id: B });
  resolveA(ok({ revision: "A:1" }));
  assert.deepEqual(await stale, { changed: false, discarded: true });
  await switched;
  assert.equal((await ui({ action: "sync_catalog" })).changed, true, "B has its own initial revision check");

  const legacy = make({ [`GET /profiles/${P}/catalog/revision`]: [404, { code: "ROUTE_NOT_FOUND" }] });
  await legacy.ui({ action: "boot" }); await legacy.ui({ action: "select_profile", id: P });
  assert.deepEqual(await legacy.ui({ action: "sync_catalog" }), { changed: true, full_refresh: true, legacy_revision_route: true, discarded: false });
});

test("season artwork uses the fixed dated API route", async () => {
  const { ui, calls } = make({ [`GET /profiles/${P}/series/42/seasons/3/artwork`]: [200, "", { size: 10 }] });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  assert.deepEqual(await ui({ action: "season_artwork", tv_id: 42, season: 3 }), { src: "blob:x" });
  assert.match(calls.at(-1).url, /\/profiles\/.+\/series\/42\/seasons\/3\/artwork$/);
  await assert.rejects(ui({ action: "season_artwork", tv_id: 0, season: 3 }), /Saison invalide/);
});

test("calendar boundaries and device-local cache bounds match the Rust client", async () => {
  const a = { mode: "auto", manual: "autumn" };
  for (const [y, m, d, want] of [[2026, 2, 28, "winter"], [2026, 3, 1, "spring"], [2026, 6, 1, "summer"], [2026, 9, 1, "autumn"], [2026, 10, 1, "halloween"], [2026, 10, 31, "halloween"], [2026, 11, 24, "autumn"], [2026, 11, 25, "christmas"], [2026, 12, 25, "christmas"], [2026, 12, 26, "winter"]]) assert.equal(resolveAppearance(a, y, m, d), want, `${m}/${d}`);
  assert.equal(resolveAppearance({ mode: "off", manual: "winter" }, 2026, 7, 1), null);
  assert.throws(() => resolveAppearance(a, 2026, 2, 29), /invalide/);
  assert.equal(resolveAppearance(a, 2028, 2, 29), "winter");
  const { ui } = make();
  assert.deepEqual((await ui({ action: "save_cache_settings", cache: { forward_mib: 16, back_mib: 0 } })).cache, { forward_mib: 16, back_mib: 0 });
  await assert.rejects(ui({ action: "save_cache_settings", cache: { forward_mib: 15, back_mib: 0 } }), /Tampon/);
  await assert.rejects(ui({ action: "save_cache_settings", cache: { forward_mib: 128, back_mib: 257 } }), /Tampon/);
});

test("watch clock: final-tenth credit is real time, never doubled by speed, never from seeks", () => {
  assert.equal(finalTenthStart(1_000_000), 900_000);
  const lease = { session_id: "s", generation: 1, media_id: M, media_revision: 1n };
  const clock = new WatchClock();
  const obs = (position, extra = {}) => ({ positionMs: position, durationMs: 1_000_000, speed: 1, playing: true, buffering: false, seek: false, ...extra });
  const first = clock.event(lease, obs(899_000), 0); assert.equal(first.final_elapsed_ms, 0); clock.acknowledge(first, 0);
  const second = clock.event(lease, obs(904_000), 5000); assert.equal(second.final_elapsed_ms, 4000); clock.acknowledge(second, 5000);
  const jump = clock.event(lease, obs(990_000, { seek: true }), 10_000); assert.equal(jump.final_elapsed_ms, 0);
  const fast = new WatchClock(); const f1 = fast.event(lease, obs(950_000, { speed: 2 }), 0); fast.acknowledge(f1, 0);
  assert.equal(fast.event(lease, obs(960_000, { speed: 2 }), 5000).final_elapsed_ms, 5000);
  assert.equal(new WatchClock().event(lease, obs(1, { speed: 3 }), 0), null);
  assert.equal(second.media_revision, 1n);
});
