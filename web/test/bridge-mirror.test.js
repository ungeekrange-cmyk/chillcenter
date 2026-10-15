"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { createBridge, BASE } = require("../src/bridge.js");
const Store = require("../src/catalog-store.js");

const P = "11111111-1111-4111-8111-111111111111", SRV = "srv";
const uid = n => `aaaaaaaa-0000-4000-8000-${String(n).padStart(12, "0")}`;
const film = (n, o = {}) => ({ media: { id: uid(n), title: `Film ${String(n).padStart(2, "0")}`, path: `f/${n}.mkv`, category: "film", year: 2000 + n, genres: [], watched: false, rating: null, available: true, added_at: n, poster: null, series: null, season: null, episode: null, series_metadata: null, series_overview: "", ...o }, revision: 1 });

// A tiny server: profiles, library, preferences, snapshot API, catalog (counted), artwork (counted).
function world(initial) {
  const w = { items: initial, revision: "rev-1", calls: [], catalogCalls: 0, artworkCalls: 0, sagas: [{ id: "1", name: "Saga", media_ids: [uid(1)], poster: null, revision: 4 }], online: true };
  const json = (status, body) => ({ ok: status < 400, status, text: async () => (body === undefined ? "" : JSON.stringify(body)), blob: async () => ({ size: 10 }) });
  w.fetch = async (url, init = {}) => {
    const method = init.method || "GET", path = url.replace(BASE, "").split("?")[0];
    w.calls.push(method + " " + path);
    if (!w.online) throw new Error("offline");
    if (path === "/server") return json(200, { server_id: SRV, capabilities: ["catalog_pages", "playback_leases", "media_range"], tls: {} });
    if (path === "/profiles") return json(200, [{ id: P, name: "A", revision: 1 }]);
    if (path === "/admin/library") return json(200, { video_root: null, revision: 0 });
    if (path === `/profiles/${P}/preferences`) return json(200, { audio_language: "en", subtitle_language: "en", autoplay_seconds: 10 });
    if (path === `/profiles/${P}/catalog`) { w.catalogCalls += 1; return json(200, { items: [], has_more: false, series_counters: {} }); }
    if (path === `/profiles/${P}/catalog/snapshot` && method === "POST") {
      const known = JSON.parse(init.body).known_revision;
      return known === w.revision ? json(200, { revision: w.revision, unchanged: true }) : json(200, { revision: w.revision, snapshot_id: "s", total: w.items.length, items: w.items.slice(0, 100), sagas: w.sagas, playlists: [], series_revisions: {}, has_more: w.items.length > 100, next_offset: w.items.length > 100 ? 100 : null });
    }
    if (path.startsWith(`/profiles/${P}/catalog/snapshot/`)) { const off = Number(new URL("http://x" + url.slice(url.indexOf("?"))).searchParams.get("offset")); return json(200, { items: w.items.slice(off, off + 100), has_more: w.items.length > off + 100, next_offset: w.items.length > off + 100 ? off + 100 : null }); }
    if (path.endsWith("/artwork")) { w.artworkCalls += 1; return json(200, ""); }
    if (/\/media\/[^/]+\/(watched|rating)$/.test(path) && method === "PUT") return json(200, { saved: true });
    if (method === "DELETE" && path === `/profiles/${P}`) return { ok: true, status: 204, text: async () => "" };
    return json(404, { code: "NOT_FOUND" });
  };
  return w;
}
function client(w, backend, extra = {}) {
  const timers = [], events = []; let urls = 0;
  const memory = new Map(); const storage = { getItem: k => memory.get(k) ?? null, setItem: (k, v) => memory.set(k, v) };
  const bridge = createBridge({ fetch: w.fetch, storage, objectUrl: () => "blob:" + (++urls), revoke() {}, address: "pi", store: backend, notify: e => events.push(e), setTimer: f => { timers.push(f); return timers.length; }, clearTimer() {}, ...extra });
  const ui = c => bridge.invoke("ui_command", { command: c });
  const flush = async () => { while (timers.length) { const f = timers.shift(); f(); await bridge._mirror().syncing; } await new Promise(r => setTimeout(r, 0)); };
  return { bridge, ui, timers, events, flush };
}
const select = async c => { await c.ui({ action: "boot" }); await c.ui({ action: "select_profile", id: P }); };
const page = (c, o = {}) => c.ui({ action: "catalog", offset: 0, category: null, q: null, sort: "title_asc", grouped: false, ...o });

test("first visit: the server answers until the first sync ends, then everything is local and the UI is told", async () => {
  const w = world(Array.from({ length: 5 }, (_, i) => film(i + 1))), c = client(w, Store.createMemoryBackend());
  await select(c);
  await page(c); assert.equal(w.catalogCalls, 1, "no mirror yet: the server serves the first page");
  await c.flush();
  assert.deepEqual(c.events, [{ type: "catalog-updated", changed: 5, removed: 0, firstSync: true, collections: false }]);
  const local = await page(c); assert.equal(w.catalogCalls, 1, "now served locally"); assert.equal(local.items.length, 5); assert.equal(local.items[0].title, "Film 01");
});

test("next visit: instant local catalogue from IndexedDB, only a no-change check goes to the server", async () => {
  const w = world(Array.from({ length: 5 }, (_, i) => film(i + 1))), store = Store.createMemoryBackend();
  const first = client(w, store); await select(first); await first.flush();
  const second = client(w, store);
  await select(second);
  const before = w.catalogCalls; const r = await page(second);
  assert.equal(r.items.length, 5); assert.equal(w.catalogCalls, before, "never asked the server for a page");
  const snapshotCalls = () => w.calls.filter(x => x.endsWith("/catalog/snapshot")).length, n = snapshotCalls();
  await second.flush();
  assert.equal(snapshotCalls(), n + 1, "one cheap check"); assert.equal(w.calls.filter(x => x.includes("/snapshot/")).length, 0, "no page downloaded"); assert.deepEqual(second.events, []);
});

test("a change on the server is detected by the check and only the difference is published", async () => {
  const w = world([film(1), film(2), film(3)]), store = Store.createMemoryBackend();
  const c = client(w, store); await select(c); await c.flush(); c.events.length = 0;
  w.items = [film(1), film(2, { watched: true }), film(4)]; w.revision = "rev-2";
  await c.ui({ action: "refresh" }); await c.flush();
  assert.deepEqual(c.events, [{ type: "catalog-updated", changed: 2, removed: 1, firstSync: false, collections: false }]);
  const r = await page(c, { sort: "title_asc" }); assert.deepEqual(r.items.map(m => m.title), ["Film 01", "Film 02", "Film 04"]);
  assert.equal((await page(c, { watched: true })).items.length, 1);
});

test("refresh answers at once and never waits for the network check", async () => {
  const w = world([film(1)]), c = client(w, Store.createMemoryBackend()); await select(c); await c.flush();
  const before = w.calls.length;
  const v = await c.ui({ action: "refresh" });
  assert.equal(v.stage, "connected"); assert.equal(w.calls.slice(before).some(x => x.includes("/snapshot")), false, "the check is only scheduled");
  assert.equal(c.timers.length, 1);
});

test("marking watched updates the local catalogue immediately and reconciles with the server later", async () => {
  const w = world([film(1), film(2)]), c = client(w, Store.createMemoryBackend()); await select(c); await c.flush();
  await c.ui({ action: "set_watched", media_id: uid(1), watched: true });
  assert.equal((await page(c, { watched: true })).items.length, 1, "visible at once, without waiting for any sync");
  assert.equal(c.timers.length, 1, "a reconciling check is scheduled");
  await c.ui({ action: "set_rating", media_id: uid(2), rating: 4 });
  assert.equal((await page(c, { sort: "rating_desc" })).items[0].rating, 4);
});

test("series, sagas, playlists and sheets are answered locally; unknown ones fall back to the server", async () => {
  const w = world([film(1, { category: "serie", series: "S", season: 1, episode: 1 }), film(2, { category: "serie", series: "S", season: 1, episode: 2, watched: true })]), c = client(w, Store.createMemoryBackend());
  await select(c); await c.flush(); const n = w.calls.length;
  const s = await c.ui({ action: "series", name: "S" }); assert.deepEqual(s.counters, { total: 2, watched: 1, unwatched: 1 });
  assert.deepEqual((await c.ui({ action: "sagas" })).items.map(x => x.name), ["Saga"]);
  assert.equal((await c.ui({ action: "saga", id: "1" })).saga.name, "Saga");
  assert.deepEqual(await c.ui({ action: "playlists" }), { items: [] });
  const d = await c.ui({ action: "media_detail", media_id: uid(1) }); assert.equal(d.media.id, uid(1)); assert.equal(d.revision, 1);
  assert.equal(w.calls.length, n, "not one server call");
  await assert.rejects(c.ui({ action: "series", name: "Absente" }), e => e.status === 404, "a series the mirror lacks is asked from the server");
});

test("switching or deleting a profile discards its in-memory mirror; the stored copy goes with a deleted profile", async () => {
  const w = world([film(1)]), store = Store.createMemoryBackend(), c = client(w, store); await select(c); await c.flush();
  assert.notEqual(await store.load(Store.mirrorKey(SRV, P)), null);
  await c.ui({ action: "delete_profile", id: P, confirm_id: P });
  assert.equal(await store.load(Store.mirrorKey(SRV, P)), null); assert.equal(c.bridge._mirror().ready, false);
});

test("a failing server never breaks reads: the last good mirror keeps answering", async () => {
  const w = world([film(1), film(2)]), c = client(w, Store.createMemoryBackend()); await select(c); await c.flush();
  w.online = false; w.revision = "rev-9";
  c.timers.push(() => c.bridge._sync()); await c.flush();
  assert.equal((await page(c)).items.length, 2);
});

test("posters: fetched once per revision, then served from the cache; no mirror means the old direct path", async () => {
  const media = film(1); media.media.poster = "x";
  const w = world([media]), c = client(w, Store.createMemoryBackend()); await select(c); await c.flush();
  const a = await c.ui({ action: "artwork", media_id: uid(1) }), b = await c.ui({ action: "artwork", media_id: uid(1) });
  assert.equal(a.src, b.src); assert.equal(w.artworkCalls, 1);
  const plain = client(w, null); await select(plain); await plain.ui({ action: "artwork", media_id: uid(1) });
  assert.equal(w.artworkCalls, 2);
});

test("a server without the snapshot routes (older API) is never retried, and keeps answering through the plain API", async () => {
  const w = world([film(1)]); const orig = w.fetch;
  w.fetch = async (url, init = {}) => (url.includes("/catalog/snapshot") ? { ok: false, status: 404, text: async () => JSON.stringify({ code: "ROUTE_NOT_FOUND" }) } : orig(url, init));
  const c = client(w, Store.createMemoryBackend()); await select(c); await c.flush();
  assert.equal(c.bridge._mirror().unsupported, true); assert.equal(c.events.length, 0);
  const n = w.calls.length; c.bridge.wake(); await c.ui({ action: "refresh" }); assert.equal(c.timers.length, 0, "nothing is scheduled any more");
  await page(c); assert.equal(w.catalogCalls, 1, "the server answers");
  assert.ok(w.calls.length > n);
});

// ---- review 9: staleness after writes (C-1), writes during a sync (C-2), profile switch during a sync (C-3) ---------------------------------
test("C-1: after a playlist/saga/sheet write, reads go to the server until the mirror has caught up; the UI is told about collection-only changes", async () => {
  const w = world([film(1), film(2)]); w.playlistsCalls = 0; w.playlists = [];
  const orig = w.fetch; w.fetch = async (url, init = {}) => {
    const path = url.replace(BASE, "").split("?")[0], method = init.method || "GET";
    if (path === `/profiles/${P}/playlists` && method === "POST") { w.playlists = [{ id: uid(900), name: "Liste", media_ids: [uid(1)], poster: null, revision: 1 }]; w.revision = "rev-2"; return { ok: true, status: 201, text: async () => JSON.stringify(w.playlists[0]), blob: async () => ({}) }; }
    if (path === `/profiles/${P}/playlists` && method === "GET") { w.playlistsCalls += 1; return { ok: true, status: 200, text: async () => JSON.stringify({ items: w.playlists }), blob: async () => ({}) }; }
    const r = await orig(url, init);
    if (path === `/profiles/${P}/catalog/snapshot` && method === "POST" && w.revision === "rev-2" && JSON.parse(init.body).known_revision !== "rev-2") { const body = JSON.parse(await r.text()); body.playlists = w.playlists; return { ...r, text: async () => JSON.stringify(body) }; }
    return r;
  };
  const c = client(w, Store.createMemoryBackend()); await select(c); await c.flush(); c.events.length = 0;
  assert.deepEqual((await c.ui({ action: "playlists" })).items, [], "local, before any write");
  await c.ui({ action: "create_playlist", name: "Liste", media_id: uid(1) });
  const fresh = await c.ui({ action: "playlists" });
  assert.equal(fresh.items.length, 1, "the SERVER answers right after the write: never the old list"); assert.equal(w.playlistsCalls, 1);
  assert.equal(c.bridge._mirror().dirty, true);
  await c.flush();
  assert.equal(c.bridge._mirror().dirty, false, "the mirror caught up");
  assert.deepEqual(c.events.map(e => [e.changed, e.collections]), [[0, true]], "a change that touches only collections is announced");
  assert.equal((await c.ui({ action: "playlists" })).items.length, 1); assert.equal(w.playlistsCalls, 1, "local again");
});

test("C-2: a write made while a snapshot is downloading is not overwritten by that older snapshot, and one more pass follows", async () => {
  const w = world([film(1), film(2)]), c = client(w, Store.createMemoryBackend()); await select(c); await c.flush();
  w.revision = "rev-2"; w.items = [film(1), film(2), film(3)];
  const orig = w.fetch; let release; const gate = new Promise(r => { release = r; });
  w.fetch = async (url, init = {}) => { if (url.includes("/catalog/snapshot") && (init.method || "GET") === "POST") await gate; return orig(url, init); };
  const syncing = c.bridge._sync();                       // downloading the snapshot that does NOT contain the write below
  await new Promise(r => setTimeout(r, 0));
  await c.ui({ action: "set_watched", media_id: uid(1), watched: true });
  release(); await syncing;
  assert.equal((await page(c, { watched: true })).items.length, 1, "the write survived the older snapshot");
  assert.ok(c.timers.length >= 1 || c.bridge._mirror().resync === false, "a follow-up pass is scheduled");
  w.fetch = orig; await c.flush();
  assert.equal((await page(c)).items.length, 3, "the follow-up pass brought the new item");
});

test("C-3: switching profile during a sync neither drops the new profile's first sync nor lets the old one clear it", async () => {
  const P2 = "44444444-4444-4444-8444-444444444444";
  const w = world([film(1)]); const orig = w.fetch; let release; const gate = new Promise(r => { release = r; });
  w.fetch = async (url, init = {}) => {
    const path = url.replace(BASE, "").split("?")[0];
    if (path === "/profiles") return { ok: true, status: 200, text: async () => JSON.stringify([{ id: P, name: "A", revision: 1 }, { id: P2, name: "B", revision: 1 }]), blob: async () => ({}) };
    if (path === `/profiles/${P2}/preferences`) return orig(url.replace(P2, P), init);
    if (path.startsWith(`/profiles/${P2}/catalog/snapshot`)) return orig(url.replace(P2, P), init);
    if (path === `/profiles/${P}/catalog/snapshot` && (init.method || "GET") === "POST") await gate;
    return orig(url, init);
  };
  const c = client(w, Store.createMemoryBackend()); await c.ui({ action: "boot" }); await c.ui({ action: "select_profile", id: P });
  const first = c.bridge._sync();                          // profile A is downloading
  await new Promise(r => setTimeout(r, 0));
  await c.ui({ action: "select_profile", id: P2 });        // the user switches to B meanwhile
  release(); await first; await c.flush();
  const m = c.bridge._mirror();
  assert.equal(m.profile, P2); assert.equal(m.ready, true, "profile B got its own first synchronisation");
});

test("series names that are prototype keys never read a prototype property", async () => {
  const w = world([film(1, { category: "serie", series: "constructor", season: 1, episode: 1 })]), c = client(w, Store.createMemoryBackend()); await select(c); await c.flush();
  const v = await c.ui({ action: "series", name: "constructor" });
  assert.equal(v.revision, null); assert.equal(v.name, "constructor");
});
