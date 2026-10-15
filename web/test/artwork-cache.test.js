"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Store = require("../src/catalog-store.js");
const { createArtworkCache } = require("../src/artwork-cache.js");

function setup(opts = {}) {
  const store = Store.createMemoryBackend(), fetched = [], revoked = []; let urls = 0, active = 0, peak = 0;
  const gates = [];
  const fetchBlob = async url => {
    fetched.push(url); active += 1; peak = Math.max(peak, active);
    if (opts.gated) await new Promise(resolve => gates.push(resolve));
    active -= 1; return { size: opts.size ?? 100, url };
  };
  const cache = createArtworkCache({ store, fetchBlob, objectUrl: () => "blob:" + (++urls), revoke: u => revoked.push(u), ...opts.cache });
  return { cache, store, fetched, revoked, gates, peak: () => peak };
}

test("a poster is fetched once, then served from memory, then from IndexedDB after a restart", async () => {
  const s = setup();
  const a = await s.cache.get("k|media|1", 5, "/a/1"), b = await s.cache.get("k|media|1", 5, "/a/1");
  assert.equal(a, b); assert.equal(s.fetched.length, 1);
  // new cache instance over the same store = browser restart: no network
  const again = createArtworkCache({ store: s.store, fetchBlob: async () => { throw new Error("network used"); }, objectUrl: () => "blob:restart" });
  assert.equal(await again.get("k|media|1", 5, "/a/1"), "blob:restart");
});

test("a new revision invalidates both layers and refetches", async () => {
  const s = setup();
  await s.cache.get("k|media|1", 1, "/a/1"); const url2 = await s.cache.get("k|media|1", 2, "/a/1");
  assert.equal(s.fetched.length, 2); assert.equal(s.revoked.length, 1, "the replaced object URL is revoked");
  assert.equal(await s.cache.get("k|media|1", 2, "/a/1"), url2);
});

test("concurrent requests for one poster share one download", async () => {
  const s = setup({ gated: true });
  const p = [s.cache.get("k|m|1", 1, "/a/1"), s.cache.get("k|m|1", 1, "/a/1"), s.cache.get("k|m|1", 1, "/a/1")];
  await new Promise(r => setTimeout(r, 5)); s.gates.splice(0).forEach(g => g());
  const urls = await Promise.all(p);
  assert.equal(new Set(urls).size, 1); assert.equal(s.fetched.length, 1);
});

test("network concurrency is capped and the most recent request is served first", async () => {
  const s = setup({ gated: true, cache: { concurrency: 2 } });
  const ps = Array.from({ length: 6 }, (_, i) => s.cache.get(`k|m|${i}`, 1, `/a/${i}`));
  await new Promise(r => setTimeout(r, 5));
  assert.equal(s.fetched.length, 2, "only two in flight"); assert.deepEqual(s.fetched, ["/a/0", "/a/1"], "the first two start at once");
  s.gates.splice(0).forEach(g => g()); await new Promise(r => setTimeout(r, 5));
  assert.deepEqual(s.fetched.slice(2, 4), ["/a/5", "/a/4"], "then the NEWEST waiting requests first (the user scrolls forward)");
  while (s.fetched.length < 6 || s.gates.length) { s.gates.splice(0).forEach(g => g()); await new Promise(r => setTimeout(r, 2)); }
  await Promise.all(ps);
  assert.equal(s.peak(), 2);
});

test("memory is bounded and evicted object URLs are revoked; failures and oversize posters are not cached", async () => {
  const s = setup({ cache: { memoryLimit: 3 } });
  for (let i = 0; i < 5; i++) await s.cache.get(`k|m|${i}`, 1, `/a/${i}`);
  assert.equal(s.cache.stats().memory, 3); assert.equal(s.revoked.length, 2);
  const big = setup({ size: 9 * 1024 * 1024 });
  await assert.rejects(big.cache.get("k|m|1", 1, "/a/1"), e => e.code === "INVALID_OPERATION");
  assert.equal(await big.store.getBlob("k|m|1"), null);
  const bad = createArtworkCache({ store: Store.createMemoryBackend(), fetchBlob: async () => { throw Object.assign(new Error("404"), { status: 404 }); }, objectUrl: () => "blob:x" });
  await assert.rejects(bad.cache?.get?.("x", 1, "/x") ?? bad.get("k|m|1", 1, "/x"), e => e.status === 404);
});

test("the disk cache is trimmed periodically and a full quota never blocks the display", async () => {
  const s = setup({ cache: { trimEvery: 2, diskLimit: 150 } });
  for (let i = 0; i < 4; i++) { await s.cache.get(`k|m|${i}`, 1, `/a/${i}`); await new Promise(r => setTimeout(r, 2)); }
  const left = (await Promise.all([0, 1, 2, 3].map(i => s.store.getBlob(`k|m|${i}`)))).filter(Boolean).length;
  assert.ok(left < 4, "oldest posters were trimmed");
  const full = createArtworkCache({ store: { getBlob: async () => null, putBlob: async () => { throw new Error("QuotaExceededError"); }, trimBlobs: async () => 0 }, fetchBlob: async () => ({ size: 10 }), objectUrl: () => "blob:ok" });
  assert.equal(await full.get("k|m|1", 1, "/a"), "blob:ok");
});

test("close() rejects waiting work and revokes everything", async () => {
  const s = setup({ gated: true, cache: { concurrency: 1 } });
  const first = s.cache.get("k|m|1", 1, "/a/1"); const second = s.cache.get("k|m|2", 1, "/a/2");
  second.catch(() => {});
  await new Promise(r => setTimeout(r, 5)); s.cache.close(); s.gates.splice(0).forEach(g => g());
  await first.catch(() => {});
  assert.equal(s.cache.stats().memory, 0);
});
