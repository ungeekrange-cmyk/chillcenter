"use strict";
// Local persistence of the catalogue mirror and of poster blobs (ADR-0019). Two interchangeable backends:
//   * IndexedDB (works over plain HTTP, unlike Cache Storage and Service Workers),
//   * memory (private browsing, no IndexedDB, tests).
// A mirror is keyed by "<server id>|<profile id>"; nothing here ever touches the network.
//
// Backend contract (all async):
//   load(key)                        -> { manifest, entries, doc } | null     entries = [{ media, revision }]
//   commit(key, { manifest, upsert, remove, doc })                            one atomic transaction
//   drop(key)                                                                  mirror + posters of that key
//   getBlob(k) / putBlob(k, { blob, rev, bytes }) / trimBlobs(maxBytes)       poster cache
(function (root) {
  const DB_NAME = "chill-center-web", DB_VERSION = 1, TOUCH_MS = 3600 * 1000;

  function createMemoryBackend() {
    const mirrors = new Map(), blobs = new Map();
    return {
      kind: "memory",
      async load(key) {
        const m = mirrors.get(key); if (!m) return null;
        return { manifest: { ...m.manifest }, entries: [...m.items.values()].map(e => ({ media: e.media, revision: e.revision })), doc: m.doc };
      },
      async commit(key, { manifest, upsert = [], remove = [], doc }) {
        // Build the next state first, publish it last: a throw leaves the previous mirror untouched (atomicity).
        const previous = mirrors.get(key);
        const items = new Map(previous ? previous.items : []);
        for (const id of remove) items.delete(id);
        for (const e of upsert) items.set(e.media.id, { media: e.media, revision: e.revision });
        mirrors.set(key, { manifest: { ...manifest }, items, doc: doc ?? (previous && previous.doc) ?? null });
      },
      async drop(key) {
        mirrors.delete(key);
        for (const k of [...blobs.keys()]) if (k.startsWith(key + "|")) blobs.delete(k);
      },
      async getBlob(k) { const r = blobs.get(k); if (!r) return null; if (Date.now() - r.at > TOUCH_MS) r.at = Date.now(); return { blob: r.blob, rev: r.rev }; },
      async putBlob(k, { blob, rev, bytes }) { blobs.set(k, { blob, rev, bytes: bytes ?? blob.size ?? 0, at: Date.now() }); },
      async trimBlobs(maxBytes) {
        let total = 0; for (const r of blobs.values()) total += r.bytes;
        if (total <= maxBytes) return 0;
        let removed = 0;
        for (const [k, r] of [...blobs.entries()].sort((a, b) => a[1].at - b[1].at)) { if (total <= maxBytes) break; blobs.delete(k); total -= r.bytes; removed += 1; }
        return removed;
      },
    };
  }

  const req = r => new Promise((resolve, reject) => { r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error || new Error("IndexedDB")); });
  const done = tx => new Promise((resolve, reject) => { tx.oncomplete = () => resolve(); tx.onerror = () => reject(tx.error || new Error("IndexedDB")); tx.onabort = () => reject(tx.error || new Error("IndexedDB annulé")); });

  function createIdbBackend(idb) {
    let opened = null;
    const open = () => opened || (opened = new Promise((resolve, reject) => {
      const r = idb.open(DB_NAME, DB_VERSION);
      r.onupgradeneeded = () => {
        const db = r.result;
        db.createObjectStore("manifest", { keyPath: "key" });
        db.createObjectStore("docs", { keyPath: "key" });
        db.createObjectStore("items", { keyPath: "k" }).createIndex("by_key", "key");
        db.createObjectStore("art", { keyPath: "k" }).createIndex("by_at", "at");
      };
      r.onsuccess = () => { r.result.onversionchange = () => r.result.close(); resolve(r.result); };
      r.onerror = () => { opened = null; reject(r.error || new Error("IndexedDB indisponible")); };
      r.onblocked = () => reject(new Error("IndexedDB bloqué"));
    }));
    return {
      kind: "indexeddb",
      async load(key) {
        const db = await open();
        const tx = db.transaction(["manifest", "items", "docs"], "readonly");
        const [manifest, rows, doc] = await Promise.all([req(tx.objectStore("manifest").get(key)), req(tx.objectStore("items").index("by_key").getAll(IDBKeyRange.only(key))), req(tx.objectStore("docs").get(key))]);
        if (!manifest) return null;
        return { manifest: { ...manifest }, entries: rows.map(r => ({ media: r.media, revision: r.revision })), doc: doc ? doc.doc : null };
      },
      async commit(key, { manifest, upsert = [], remove = [], doc }) {
        const db = await open();
        const tx = db.transaction(["manifest", "items", "docs"], "readwrite");
        const items = tx.objectStore("items");
        for (const id of remove) items.delete(key + "|" + id);
        for (const e of upsert) items.put({ k: key + "|" + e.media.id, key, media: e.media, revision: e.revision });
        if (doc !== undefined) tx.objectStore("docs").put({ key, doc });
        tx.objectStore("manifest").put({ ...manifest, key });
        await done(tx);
      },
      async drop(key) {
        const db = await open();
        const tx = db.transaction(["manifest", "items", "docs", "art"], "readwrite");
        tx.objectStore("manifest").delete(key); tx.objectStore("docs").delete(key);
        const range = IDBKeyRange.bound(key + "|", key + "|￿");
        tx.objectStore("items").delete(range); tx.objectStore("art").delete(range);
        await done(tx);
      },
      async getBlob(k) {
        const db = await open();
        const tx = db.transaction("art", "readwrite"), store = tx.objectStore("art");
        const rec = await req(store.get(k));
        if (!rec) return null;
        if (Date.now() - rec.at > TOUCH_MS) { rec.at = Date.now(); store.put(rec); }   // recency is kept to the hour: no rewrite of the blob at every display
        return { blob: rec.blob, rev: rec.rev };
      },
      async putBlob(k, { blob, rev, bytes }) {
        const db = await open();
        const tx = db.transaction("art", "readwrite");
        tx.objectStore("art").put({ k, blob, rev, bytes: bytes ?? blob.size ?? 0, at: Date.now() });
        await done(tx);
      },
      async trimBlobs(maxBytes) {
        const db = await open();
        const tx = db.transaction("art", "readwrite"), store = tx.objectStore("art");
        const all = await req(store.index("by_at").getAll());   // oldest access first
        let total = all.reduce((n, r) => n + r.bytes, 0), removed = 0;
        for (const r of all) { if (total <= maxBytes) break; store.delete(r.k); total -= r.bytes; removed += 1; }
        await done(tx);
        return removed;
      },
    };
  }

  // IndexedDB can exist and still refuse to work (private windows, blocked site data): probe once, fall back to memory.
  async function openBestBackend(idb) {
    if (!idb) return createMemoryBackend();
    try {
      const backend = createIdbBackend(idb);
      await backend.load("probe|probe");
      return backend;
    } catch { return createMemoryBackend(); }
  }

  const api = { createMemoryBackend, createIdbBackend, openBestBackend, mirrorKey: (server, profile) => `${server}|${profile}` };
  if (typeof module !== "undefined") module.exports = api; else root.ChillCatalogStore = api;
})(typeof window !== "undefined" ? window : globalThis);
