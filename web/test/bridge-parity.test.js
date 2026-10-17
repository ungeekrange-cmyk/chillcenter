"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { createBridge, BASE } = require("../src/bridge.js");

const P = "11111111-1111-4111-8111-111111111111";
const M = "22222222-2222-4222-8222-222222222222";
function client() {
  const calls = [];
  const fetch = async (url, init = {}) => {
    const path = url.replace(BASE, ""); calls.push({ method: init.method || "GET", path, body: init.body && JSON.parse(init.body) });
    const base = path.split("?")[0]; let body = {};
    if (base === "/server") body = { server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"] };
    else if (base === "/profiles") body = [{ id: P, name: "A", revision: 1 }];
    else if (base === "/admin/library") body = { video_root: null, additional_video_roots: [], revision: 2 };
    else if (base.endsWith("/preferences")) body = {};
    return { ok: true, status: 200, text: async () => JSON.stringify(body) };
  };
  const storage = { getItem: () => null, setItem() {} };
  const bridge = createBridge({ fetch, storage, objectUrl: () => "blob:x" });
  return { calls, bridge, ui: command => bridge.invoke("ui_command", { command }) };
}
async function ready(c) { await c.ui({ action: "boot" }); await c.ui({ action: "select_profile", id: P }); c.calls.length = 0; }

test("local language, initial preferences, and multi-library bodies preserve current UI contracts", async () => {
  const c = client(); await ready(c);
  assert.deepEqual(await c.ui({ action: "save_language", language: "auto" }), { language: "auto" });
  await c.ui({ action: "create_profile", name: "B", initial_preferences: { autoplay_seconds: 20 } });
  await c.ui({ action: "save_library", video_root: "/one", additional_video_roots: ["/two"], expected_revision: 2 });
  assert.deepEqual(c.calls.find(x => x.method === "POST" && x.path === "/profiles").body, { name: "B", initial_preferences: { autoplay_seconds: 20 } });
  assert.deepEqual(c.calls.find(x => x.method === "PUT" && x.path === "/admin/library").body, { video_root: "/one", additional_video_roots: ["/two"], expected_revision: 2 });
});

test("TMDB, identity, and saga-member actions use dated REST routes", async () => {
  const c = client(); await ready(c);
  await c.ui({ action: "tmdb_status" });
  await c.ui({ action: "tmdb_configure", credential_type: "api_key", credential: "secret", auto_enrich: true });
  await c.ui({ action: "tmdb_candidates", media_id: M, query: "Dune" });
  await c.ui({ action: "tmdb_select", media_id: M, expected_revision: 3, tmdb_id: 42, kind: "movie", scope: "media" });
  await c.ui({ action: "unavailable", offset: 0, limit: 48 });
  await c.ui({ action: "confirm_reassociation", source_id: M, target_id: P, expected_source_revision: 1, expected_target_revision: 2, expected_epoch: 3, confirmed: true });
  await c.ui({ action: "saga_member_artwork", id: "saga", expected_revision: 4, media_id: M });
  assert.deepEqual(c.calls.map(x => x.method + " " + x.path), [
    "GET /admin/tmdb", "PUT /admin/tmdb", `GET /admin/profiles/${P}/media/${M}/tmdb/candidates?query=Dune`, `POST /admin/profiles/${P}/media/${M}/tmdb`, "GET /admin/unavailable?offset=0&limit=48", "POST /admin/reassociations", `PUT /profiles/${P}/sagas/saga/poster/member`
  ]);
});

test("browser artwork and archive adapters preserve confirmation and bounded file seams", async () => {
  const files = [{ name: "cover.png", type: "image/png", size: 4 }, { name: "backup.zip", type: "application/zip", size: 4 }], calls = [];
  const c = client();
  // Recreate with explicit chooser and binary-aware fetch so no native path crosses the bridge.
  const fetch = async (url, init = {}) => { const path = url.replace(BASE, ""); calls.push({ method: init.method || "GET", path, body: init.body }); const bare = path.split("?")[0]; let json = {};
    if (bare === "/server") json = { server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"] };
    else if (bare === "/profiles") json = [{ id: P, name: "A", revision: 1 }]; else if (bare === "/admin/library") json = { revision: 1 };
    else if (bare.endsWith("/preferences")) json = {}; else if (bare === "/admin/imports") json = { preview_id: M, counts: {}, profiles: [], roots: [], apply_supported: true };
    return { ok: true, status: 200, text: async () => JSON.stringify(json), blob: async () => ({ size: 4 }) }; };
  const b = createBridge({ fetch, storage: { getItem: () => null, setItem() {} }, chooseFile: async () => files.shift(), objectUrl: () => "blob:x" }); const ui = command => b.invoke("ui_command", { command });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P });
  await b.invoke("artwork_command", { target: { kind: "media", media_id: M, expected_revision: 1 } });
  const preview = await b.invoke("admin_command", { command: { action: "import" } }); await new Promise(resolve => setImmediate(resolve));
  const ready = await b.invoke("admin_command", { command: { action: "status", id: preview.id } });
  await assert.rejects(b.invoke("admin_command", { command: { action: "apply", id: preview.id, confirmed: false } }), e => e.code === "ADMIN_CONFIRMATION");
  assert.equal(calls.some(x => x.method === "PUT" && x.path === `/profiles/${P}/media/${M}/poster?expected_revision=1`), true);
  assert.equal(ready.stage, "preview");
});

test("missing profile preferences default to English and 20 seconds", async () => {
  const c = client(); await ready(c); await c.ui({ action: "create_profile", name: "B" });
  assert.deepEqual(c.calls.find(x => x.method === "POST" && x.path === "/profiles").body.initial_preferences, { audio_language: "en", subtitle_language: "en", autoplay_seconds: 20 });
});

test("archive apply sends the exact remote preview route and confirmed plan body", async () => {
  const file = { name: "backup.zip", type: "application/zip", size: 4 }, calls = [];
  const fetch = async (url, init = {}) => { const path = url.replace(BASE, ""); calls.push({ method: init.method || "GET", path, body: typeof init.body === "string" ? JSON.parse(init.body) : init.body }); const bare = path.split("?")[0]; let body = {};
    if (bare === "/server") body = { server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"] }; else if (bare === "/profiles") body = [{ id: P, name: "A", revision: 1 }]; else if (bare === "/admin/library" || bare.endsWith("/preferences")) body = {};
    else if (bare === "/admin/imports") body = { preview_id: M, counts: {}, profiles: [], roots: [], apply_supported: true }; else if (bare.endsWith("/plan")) body = { plan_id: P }; else if (bare.endsWith("/apply")) body = { media: 1, profiles: 1, epoch: 2, already_imported: false };
    return { ok: true, status: 200, text: async () => JSON.stringify(body) }; };
  const b = createBridge({ fetch, storage: { getItem: () => null, setItem() {} }, chooseFile: async () => file }); const ui = command => b.invoke("ui_command", { command });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P }); const start = await b.invoke("admin_command", { command: { action: "import" } }); let status; for (let i = 0; i < 20; i++) { status = await b.invoke("admin_command", { command: { action: "status", id: start.id } }); if (status.stage !== "working") break; await new Promise(r => setTimeout(r, 1)); } assert.equal(status.stage, "preview", status.error);
  await b.invoke("admin_command", { command: { action: "plan", id: start.id, mapping: { profile_map: {}, root_prefix: "", conflict_policy: "keep_destination" } } });
  await b.invoke("admin_command", { command: { action: "apply", id: start.id, confirmed: true } });
  assert.deepEqual(calls.at(-1), { method: "POST", path: `/admin/imports/${M}/apply`, body: { plan_id: P, confirmed: true } });
});

test("cancelled upload aborts and deletes a late preview", async () => {
  let release, signal; const pending = new Promise(r => { release = r; }), calls = [];
  const fetch = async (url, init = {}) => { const path = url.replace(BASE, ""); calls.push({ method: init.method || "GET", path }); const bare = path.split("?")[0];
    if (bare === "/server") return { ok: true, status: 200, text: async () => JSON.stringify({ server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"] }) };
    if (bare === "/profiles") return { ok: true, status: 200, text: async () => JSON.stringify([{ id: P, name: "A", revision: 1 }]) };
    if (bare === "/admin/library" || bare.endsWith("/preferences")) return { ok: true, status: 200, text: async () => "{}" };
    if (bare === "/admin/imports" && init.method === "POST") { signal = init.signal; return pending; }
    return { ok: true, status: 204, text: async () => "" }; };
  const b = createBridge({ fetch, storage: { getItem: () => null, setItem() {} }, chooseFile: async () => ({ name: "x.zip", type: "application/zip", size: 1 }) }), ui = c => b.invoke("ui_command", { command: c });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P }); const start = await b.invoke("admin_command", { command: { action: "import" } }); await b.invoke("admin_command", { command: { action: "cancel", id: start.id } }); assert.equal(signal.aborted, true);
  release({ ok: true, status: 200, text: async () => JSON.stringify({ preview_id: M }) }); await new Promise(r => setImmediate(r));
  assert.equal(calls.some(x => x.method === "DELETE" && x.path === `/admin/imports/${M}`), true);
});

test("profile switch during artwork choosing sends no upload", async () => {
  let select; const calls = []; const chooseFile = () => new Promise(resolve => { select = resolve; }); const fetch = async (url, init = {}) => { const path = url.replace(BASE, ""); calls.push({ method: init.method || "GET", path }); const bare = path.split("?")[0]; const body = bare === "/server" ? { server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"] } : bare === "/profiles" ? [{ id: P, name: "A", revision: 1 }, { id: M, name: "B", revision: 1 }] : {}; return { ok: true, status: 200, text: async () => JSON.stringify(body) }; };
  const b = createBridge({ fetch, storage: { getItem: () => null, setItem() {} }, chooseFile }), ui = c => b.invoke("ui_command", { command: c }); await ui({ action: "boot" }); await ui({ action: "select_profile", id: P }); const upload = b.invoke("artwork_command", { target: { kind: "media", media_id: M, expected_revision: 1 } }); await ui({ action: "select_profile", id: M }); select({ name: "x.png", type: "image/png", size: 1 }); assert.deepEqual(await upload, { discarded: true }); assert.equal(calls.some(x => x.method === "PUT"), false);
});

test("export save delegates direct download without GET or blob buffering", async () => {
  const downloads = [], calls = []; const fetch = async (url, init = {}) => { const path = url.replace(BASE, ""); calls.push({ method: init.method || "GET", path }); const bare = path.split("?")[0]; const body = bare === "/server" ? { server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"] } : bare === "/profiles" ? [{ id: P, name: "A" }] : bare === "/admin/exports" ? { export_id: M, state: "ready" } : {}; return { ok: true, status: 200, text: async () => JSON.stringify(body) }; };
  const b = createBridge({ fetch, storage: { getItem: () => null, setItem() {} }, downloadFile: x => downloads.push(x) }), ui = c => b.invoke("ui_command", { command: c }); await ui({ action: "boot" }); const job = await b.invoke("admin_command", { command: { action: "export" } }); await b.invoke("admin_command", { command: { action: "save", id: job.id } }); assert.deepEqual(downloads, [{ href: `${BASE}/admin/exports/${M}`, name: "chill-center-export.zip" }]); assert.equal(calls.some(x => x.method === "GET" && x.path.includes(`/exports/${M}`)), false);
});

test("series fallback explicitly distinguishes opaque keys from legacy titles", async () => {
  const c = client(); await ready(c);
  await c.ui({ action: "series", name: "tmdb:tv:42" });
  await c.ui({ action: "series", name: "Legacy title" });
  await c.ui({ action: "edit_series", series: "tmdb:tv:42", expected_revision: 1, patch: { name: "New" } });
  await c.ui({ action: "edit_series", series: "Legacy title", expected_revision: 1, patch: { name: "New" } });
  assert.deepEqual(c.calls.map(x => `${x.method} ${x.path}`), [
    `GET /profiles/${P}/series/tmdb%3Atv%3A42?selector=key`,
    `GET /profiles/${P}/series/Legacy%20title?selector=title`,
    `PUT /profiles/${P}/series/tmdb%3Atv%3A42?selector=key`,
    `PUT /profiles/${P}/series/Legacy%20title?selector=title`
  ]);
});

test("export cancellation calls its explicit server cancellation route", async () => {
  const c = client(); await ready(c);
  const job = await c.bridge.invoke("admin_command", { command: { action: "export" } });
  await c.bridge.invoke("admin_command", { command: { action: "cancel", id: job.id } });
  assert.equal(c.calls.at(-1).path, `/admin/exports/${job.id}/cancel`);
  assert.equal(c.calls.at(-1).method, "DELETE");
});

test("late import apply after profile switch is reported stale without publishing to the new mirror", async () => {
  let release; const applied = new Promise(resolve => { release = resolve; }); const fetch = async (url, init = {}) => { const path = url.replace(BASE, ""); const bare = path.split("?")[0]; let body = {};
    if (bare === "/server") body = { server_id: "srv", capabilities: ["catalog_pages", "playback_leases", "media_range"] };
    else if (bare === "/profiles") body = [{ id: P, name: "A", revision: 1 }, { id: M, name: "B", revision: 1 }];
    else if (bare === "/admin/imports") body = { preview_id: M }; else if (bare.endsWith("/plan")) body = { plan_id: P };
    else if (bare.endsWith("/apply")) return applied;
    return { ok: true, status: 200, text: async () => JSON.stringify(body) }; };
  const b = createBridge({ fetch, storage: { getItem: () => null, setItem() {} }, chooseFile: async () => ({ name: "x.zip", type: "application/zip", size: 1 }) }), ui = c => b.invoke("ui_command", { command: c });
  await ui({ action: "boot" }); await ui({ action: "select_profile", id: P }); const job = await b.invoke("admin_command", { command: { action: "import" } });
  for (let i = 0; i < 20; i++) { if ((await b.invoke("admin_command", { command: { action: "status", id: job.id } })).stage === "preview") break; await new Promise(r => setTimeout(r, 1)); }
  await b.invoke("admin_command", { command: { action: "plan", id: job.id, mapping: {} } }); const apply = b.invoke("admin_command", { command: { action: "apply", id: job.id, confirmed: true } });
  await ui({ action: "select_profile", id: M }); release({ ok: true, status: 200, text: async () => "{}" }); await assert.rejects(apply, e => e.code === "ADMIN_STALE");
});
