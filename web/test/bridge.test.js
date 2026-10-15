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
    calls.push({ url, method: init.method || "GET", body: init.body, headers: init.headers });
    const key = (init.method || "GET") + " " + url.replace(BASE, "").split("?")[0];
    const r = routes[key];
    if (!r) return { ok: false, status: 404, text: async () => '{"code":"NOT_FOUND"}' };
    const [status, body, extra] = typeof r === "function" ? r(url, init) : r;
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

test("artwork becomes a blob URL once; unsupported commands fail explicitly", async () => {
  const { ui, calls } = make({ [`GET /profiles/${P}/media/${M}/artwork`]: [200, "", { size: 10 }] });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  assert.deepEqual(await ui({ action: "artwork", media_id: M }), { src: "blob:x" });
  await ui({ action: "artwork", media_id: M });
  assert.equal(calls.filter(c => c.url.endsWith("/artwork")).length, 1);
  await assert.rejects(ui({ action: "tmdb_status" }), e => e.code === "WEB_UNSUPPORTED");
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
