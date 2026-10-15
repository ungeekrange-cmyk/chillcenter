"use strict";
// Poster cache (ADR-0019): memory (object URLs) -> IndexedDB blobs -> network, with a small network concurrency so that a page of
// 48 cards does not flood the Pi, and the most recently requested poster served first (the user scrolls forward).
// A poster is valid while its revision matches; nothing is ever fetched twice for the same key and revision.
(function (root) {
  const MAX_BYTES = 8 * 1024 * 1024;

  function createArtworkCache({ store, fetchBlob, objectUrl, revoke = () => {}, concurrency = 4, memoryLimit = 400, diskLimit = 150 * 1024 * 1024, trimEvery = 40 }) {
    const memory = new Map();     // key -> { url, rev }, Map insertion order = recency
    const inflight = new Map();   // key -> Promise<string>
    const waiting = [];           // pending network tasks, newest last (served first)
    let running = 0, stored = 0, closed = false;

    const remember = (key, rev, url) => {
      const old = memory.get(key); if (old) { memory.delete(key); if (old.url !== url) revoke(old.url); }
      memory.set(key, { url, rev });
      while (memory.size > memoryLimit) { const [k, v] = memory.entries().next().value; memory.delete(k); revoke(v.url); }
      return url;
    };
    const pump = () => {
      while (running < concurrency && waiting.length) {
        const task = waiting.pop(); running += 1;
        task().finally(() => { running -= 1; pump(); });
      }
    };
    async function load(key, rev, url) {
      const rec = await store.getBlob(key).catch(() => null);
      if (rec && rec.rev === rev) return remember(key, rev, objectUrl(rec.blob));
      return new Promise((resolve, reject) => {
        waiting.push(async () => {
          if (closed) { reject(Object.assign(new Error("Cache fermé."), { code: "CLOSED" })); return; }
          try {
            const blob = await fetchBlob(url);
            if (closed) throw Object.assign(new Error("Cache fermé."), { code: "CLOSED" });   // disconnected while downloading: keep nothing
            if (!blob || blob.size > MAX_BYTES) throw Object.assign(new Error("Affiche non prise en charge."), { code: "INVALID_OPERATION" });
            try { await store.putBlob(key, { blob, rev, bytes: blob.size }); if (++stored % trimEvery === 0) await store.trimBlobs(diskLimit); } catch { /* quota: the poster still displays */ }
            resolve(remember(key, rev, objectUrl(blob)));
          } catch (e) { reject(e); }
        });
        pump();
      });
    }
    return {
      get(key, rev, url) {
        const hit = memory.get(key);
        if (hit && hit.rev === rev) { memory.delete(key); memory.set(key, hit); return Promise.resolve(hit.url); }
        const pending = inflight.get(key + "\u0000" + rev);
        if (pending) return pending;
        const p = load(key, rev, url).finally(() => inflight.delete(key + "\u0000" + rev));
        inflight.set(key + "\u0000" + rev, p);
        return p;
      },
      clear() { for (const v of memory.values()) revoke(v.url); memory.clear(); waiting.length = 0; },
      close() { closed = true; this.clear(); },
      stats: () => ({ memory: memory.size, running, waiting: waiting.length }),
    };
  }

  const api = { createArtworkCache, MAX_BYTES };
  if (typeof module !== "undefined") module.exports = api; else root.ChillArtworkCache = api;
})(typeof window !== "undefined" ? window : globalThis);
