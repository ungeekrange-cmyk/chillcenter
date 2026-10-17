"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Store = require("../src/catalog-store.js");
const Sync = require("../src/catalog-sync.js");

const P = "p1", KEY = Store.mirrorKey("srv", P);
const item = (n, o = {}) => ({ media: { id: `m${n}`, title: `T${n}`, watched: false, rating: null, ...o }, revision: 1 });
const snapshot = (items, o = {}) => ({ revision: "rev-1", snapshot_id: "snap-1", total: items.length, items: items.slice(0, 100), sagas: [{ id: "s" }], playlists: [], series_revisions: { A: 3 }, has_more: items.length > 100, next_offset: items.length > 100 ? 100 : null, ...o });
// A fake server speaking the snapshot API; `script` lets a test inject errors per call.
function server(items, { revision = "rev-1", failures = [] } = {}) {
  const calls = []; let pending = [...failures];
  const request = async (method, path, o = {}) => {
    calls.push(method + " " + path + (o.query ? "?" + o.query.map(q => q.join("=")).join("&") : ""));
    const f = pending.find(x => x.when(method, path)); if (f) { pending = pending.filter(x => x !== f); throw Object.assign(new Error("boom"), { status: f.status }); }
    if (method === "POST") return o.body.known_revision === revision ? { revision, unchanged: true } : snapshot(items, { revision });
    const offset = Number(o.query.find(q => q[0] === "offset")[1]);
    return { revision, items: items.slice(offset, offset + 100), has_more: items.length > offset + 100, next_offset: items.length > offset + 100 ? offset + 100 : null };
  };
  return { request, calls };
}
// Existing fixtures model only the retained snapshot protocol. Delta behaviour
// has its own explicit wire fixtures below.
const run = (srv, store, local, extra = {}) => Sync.sync({ request: srv.request, store, key: KEY, profile: P, local, preferDelta: false, ...extra });
const entriesOf = async store => (await store.load(KEY)).entries.map(e => e.media.id).sort();

test("first sync downloads every page (100 per page), commits atomically and reports firstSync", async () => {
  const items = Array.from({ length: 250 }, (_, i) => item(i));
  const store = Store.createMemoryBackend(), srv = server(items);
  const r = await run(srv, store, null);
  assert.equal(r.firstSync, true); assert.equal(r.changed, 250); assert.equal(r.removed, 0); assert.equal(r.entries.length, 250);
  assert.equal(srv.calls.length, 3, "one POST then two pages");
  const saved = await store.load(KEY);
  assert.equal(saved.entries.length, 250); assert.equal(saved.manifest.revision, "rev-1"); assert.deepEqual(saved.doc.series_revisions, { A: 3 });
});

test("an unchanged revision costs one request and rewrites nothing but the check date", async () => {
  const items = [item(1), item(2)], store = Store.createMemoryBackend(), srv = server(items);
  await run(srv, store, null);
  const local = await store.load(KEY); const before = srv.calls.length;
  const r = await run(srv, store, local, { now: () => 12345 });
  assert.equal(r.unchanged, true); assert.equal(srv.calls.length, before + 1);
  assert.equal((await store.load(KEY)).manifest.checked_at, 12345);
});

test("only the difference is written: changed content (even personal state), new and removed items", async () => {
  const store = Store.createMemoryBackend();
  const v1 = [item(1), item(2), item(3)];
  await run(server(v1), store, null);
  const writes = []; const real = store.commit.bind(store); store.commit = async (k, c) => { writes.push({ up: c.upsert.map(e => e.media.id), rm: c.remove }); return real(k, c); };
  const v2 = [item(1), item(2, { watched: true }), item(4)];   // 2 changed personal state, 3 removed, 4 added
  const r = await run(server(v2, { revision: "rev-2" }), store, await store.load(KEY));
  assert.deepEqual(writes[0].up.sort(), ["m2", "m4"]); assert.deepEqual(writes[0].rm, ["m3"]);
  assert.equal(r.changed, 2); assert.equal(r.removed, 1); assert.equal(r.firstSync, false);
  assert.deepEqual(await entriesOf(store), ["m1", "m2", "m4"]);
  assert.equal((await store.load(KEY)).entries.find(e => e.media.id === "m2").media.watched, true);
});

test("diff ignores identical items and detects a revision-only change", () => {
  const a = [item(1), item(2)], b = [item(1), { ...item(2), revision: 2 }];
  const d = Sync.diff(a, b);
  assert.deepEqual(d.upsert.map(e => e.media.id), ["m2"]); assert.deepEqual(d.remove, []);
});

test("409 SYNC_SNAPSHOT_CHANGED and 429 busy are retried (bounded); other errors surface", async () => {
  const items = [item(1)], store = Store.createMemoryBackend();
  const fails = [{ when: m => m === "POST", status: 409 }, { when: m => m === "POST", status: 429 }];
  const r = await run(server(items, { failures: fails }), store, null);
  assert.equal(r.changed, 1);
  const always = Array.from({ length: 10 }, () => ({ when: m => m === "POST", status: 409 }));
  await assert.rejects(run(server(items, { failures: always }), Store.createMemoryBackend(), null), e => e.status === 409);
  await assert.rejects(run(server(items, { failures: [{ when: m => m === "POST", status: 404 }] }), Store.createMemoryBackend(), null), e => e.status === 404);
});

test("an expired snapshot (410) restarts the download once", async () => {
  const items = Array.from({ length: 150 }, (_, i) => item(i)), store = Store.createMemoryBackend();
  const r = await run(server(items, { failures: [{ when: (m, p) => m === "GET" && p.includes("/snapshot/"), status: 410 }] }), store, null);
  assert.equal(r.entries.length, 150);
});

test("a truncated snapshot is refused and the previous mirror stays intact", async () => {
  const store = Store.createMemoryBackend();
  await run(server([item(1), item(2)]), store, null);
  const lying = { request: async (m, p) => (m === "POST" ? snapshot([item(1), item(2), item(3)], { revision: "rev-2", total: 3, items: [item(1)], has_more: false }) : null) };
  await assert.rejects(Sync.sync({ request: lying.request, store, key: KEY, profile: P, local: await store.load(KEY), preferDelta: false }), /incomplet/);
  assert.deepEqual(await entriesOf(store), ["m1", "m2"]); assert.equal((await store.load(KEY)).manifest.revision, "rev-1");
});

test("a failing commit leaves the previous mirror untouched (atomic publish)", async () => {
  const store = Store.createMemoryBackend();
  await run(server([item(1)]), store, null);
  const real = store.commit.bind(store);
  store.commit = async () => { throw new Error("disk full"); };
  await assert.rejects(run(server([item(1), item(2)], { revision: "rev-2" }), store, await store.load(KEY)), /disk full/);
  store.commit = real;
  assert.deepEqual(await entriesOf(store), ["m1"]);
});

test("abort stops the download between pages", async () => {
  const items = Array.from({ length: 300 }, (_, i) => item(i)), ctl = new AbortController();
  const srv = server(items); const real = srv.request;
  srv.request = async (m, p, o) => { const r = await real(m, p, o); if (m === "GET") ctl.abort(); return r; };
  await assert.rejects(run(srv, Store.createMemoryBackend(), null, { signal: ctl.signal }), e => e.code === "ABORTED");
});

function deltaServer(pages, { revision = "delta-2", failures = [] } = {}) {
  const calls = []; let pending = [...failures];
  const request = async (method, path, o = {}) => {
    calls.push(method + " " + path + (o.query ? "?" + o.query.map(q => q.join("=")).join("&") : ""));
    const failure = pending.find(x => x.when(method, path)); if (failure) { pending = pending.filter(x => x !== failure); throw Object.assign(new Error("boom"), { status: failure.status, code: failure.code }); }
    if (method === "POST") return pages[0];
    return pages[Number(o.query.find(q => q[0] === "offset")[1]) / 100];
  };
  return { request, calls };
}
test("changes delta merges one upsert and one delete without retransferring the snapshot", async () => {
  const store = Store.createMemoryBackend();
  await run(server([item(1), item(2)]), store, null);
  const first = { revision: "delta-2", unchanged: false, reset: false, snapshot_id: "d", total: 1, items: [item(3)], deleted_ids: ["m1"], sagas: [], playlists: [], series_revisions: {}, season_metadata: {}, has_more: false, next_offset: null };
  const srv = deltaServer([first]); const result = await Sync.sync({ request: srv.request, store, key: KEY, profile: P, local: await store.load(KEY) });
  assert.deepEqual(await entriesOf(store), ["m2", "m3"]); assert.equal(result.changed, 1); assert.equal(result.removed, 1);
  assert.deepEqual(srv.calls, ["POST /profiles/p1/catalog/changes"]);
});
test("delta aux-only season metadata publishes a new document while keeping entries", async () => {
  const store = Store.createMemoryBackend(); await run(server([item(1)]), store, null);
  const first = { revision: "delta-2", unchanged: false, reset: false, snapshot_id: "d", total: 0, items: [], deleted_ids: [], sagas: [], playlists: [], series_revisions: {}, season_metadata: { "tmdb:tv:9:season:1": { overview: "x" } }, has_more: false };
  await Sync.sync({ request: deltaServer([first]).request, store, key: KEY, profile: P, local: await store.load(KEY) });
  const saved = await store.load(KEY); assert.deepEqual(await entriesOf(store), ["m1"]); assert.equal(saved.doc.season_metadata["tmdb:tv:9:season:1"].overview, "x");
});
test("delta reset atomically replaces the mirror", async () => {
  const store = Store.createMemoryBackend(); await run(server([item(1), item(2)]), store, null);
  const first = { revision: "delta-2", unchanged: false, reset: true, snapshot_id: "d", total: 1, items: [item(3)], deleted_ids: [], sagas: [], playlists: [], series_revisions: {}, season_metadata: {}, has_more: false };
  await Sync.sync({ request: deltaServer([first]).request, store, key: KEY, profile: P, local: await store.load(KEY) });
  assert.deepEqual(await entriesOf(store), ["m3"]);
});
test("failed or expired delta pages never publish a partial mirror", async () => {
  const store = Store.createMemoryBackend(); await run(server([item(1)]), store, null);
  const first = { revision: "delta-2", unchanged: false, reset: false, snapshot_id: "d", total: 101, items: [item(2)], deleted_ids: [], sagas: [], playlists: [], series_revisions: {}, season_metadata: {}, has_more: true, next_offset: 100 };
  const srv = deltaServer([first, null], { failures: [{ when: m => m === "GET", status: 410 }] });
  await assert.rejects(Sync.sync({ request: srv.request, store, key: KEY, profile: P, local: await store.load(KEY) }));
  assert.deepEqual(await entriesOf(store), ["m1"]);
});
test("unchanged delta costs exactly one request; profile 404 does not fall back", async () => {
  const store = Store.createMemoryBackend();
  const unchanged = deltaServer([{ revision: "known", unchanged: true }]);
  const local = { manifest: { revision: "known", total: 0 }, entries: [], doc: {} };
  await Sync.sync({ request: unchanged.request, store, key: KEY, profile: P, local }); assert.equal(unchanged.calls.length, 1);
  const missing = deltaServer([], { failures: [{ when: m => m === "POST", status: 404, code: "PROFILE_NOT_FOUND" }] });
  await assert.rejects(Sync.sync({ request: missing.request, store, key: KEY, profile: P, local }), e => e.code === "PROFILE_NOT_FOUND");
  assert.equal(missing.calls.length, 1);
});
test("delta byte budget counts UTF-8 bytes rather than JavaScript UTF-16 units", async () => {
  const huge = item(1, { title: "€".repeat(11 * 1024 * 1024) }); // 33 MiB UTF-8, only 11 Mi code units
  const first = { revision: "delta-2", unchanged: false, reset: true, snapshot_id: "d", total: 1, items: [huge], deleted_ids: [], sagas: [], playlists: [], series_revisions: {}, season_metadata: {}, has_more: false };
  await assert.rejects(Sync.sync({ request: deltaServer([first]).request, store: Store.createMemoryBackend(), key: KEY, profile: P, local: null }), e => e.code === "SYNC_CATALOG_TOO_LARGE");
});
test("a ROUTE_NOT_FOUND fallback refuses an oversized legacy snapshot before publishing", async () => {
  const store = Store.createMemoryBackend(); await run(server([item(1)]), store, null);
  const request = async (method, path) => {
    if (path.endsWith("/catalog/changes")) throw Object.assign(new Error("old server"), { status: 404, code: "ROUTE_NOT_FOUND" });
    assert.equal(method, "POST");
    return snapshot(Array.from({ length: 10001 }, (_, i) => item(i)), { revision: "too-big", total: 10001 });
  };
  await assert.rejects(Sync.sync({ request, store, key: KEY, profile: P, local: await store.load(KEY) }), e => e.code === "SYNC_INVALID");
  assert.deepEqual(await entriesOf(store), ["m1"]);
});

test("drop removes a mirror and its posters, and leaves other mirrors alone", async () => {
  const store = Store.createMemoryBackend();
  await run(server([item(1)]), store, null);
  await store.putBlob(KEY + "|media|m1", { blob: { size: 10 }, rev: 1 }); await store.putBlob("other|x|media|m1", { blob: { size: 10 }, rev: 1 });
  await store.drop(KEY);
  assert.equal(await store.load(KEY), null); assert.equal(await store.getBlob(KEY + "|media|m1"), null);
  assert.notEqual(await store.getBlob("other|x|media|m1"), null);
});

test("poster cache trims the least recently used blobs down to the cap", async () => {
  const store = Store.createMemoryBackend();
  await store.putBlob("a", { blob: { size: 40 }, rev: 1, bytes: 40 }); await new Promise(r => setTimeout(r, 3));
  await store.putBlob("b", { blob: { size: 40 }, rev: 1, bytes: 40 }); await new Promise(r => setTimeout(r, 3));
  await store.putBlob("c", { blob: { size: 40 }, rev: 1, bytes: 40 }); await new Promise(r => setTimeout(r, 3));
  await store.getBlob("a");   // reads refresh recency at most once an hour (no rewrite of the blob at every display): the oldest write goes first
  assert.equal(await store.trimBlobs(80), 1);
  assert.equal(await store.getBlob("a"), null); assert.notEqual(await store.getBlob("b"), null); assert.notEqual(await store.getBlob("c"), null);
});

test("openBestBackend falls back to memory when IndexedDB is missing or refuses to open", async () => {
  assert.equal((await Store.openBestBackend(null)).kind, "memory");
  const broken = { open() { const r = {}; setTimeout(() => r.onerror && r.onerror(), 0); return r; } };
  assert.equal((await Store.openBestBackend(broken)).kind, "memory");
});
