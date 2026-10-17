"use strict";
// Background synchronisation of the catalogue mirror (ADR-0019). Prefer the additive changes API:
//   POST /profiles/{p}/catalog/changes {known_revision}  -> {unchanged:true} or delta pages
//   GET  /profiles/{p}/catalog/changes/{id}?offset=N     -> next page of 100
// Snapshot download remains a strictly validated compatibility fallback only for 404 ROUTE_NOT_FOUND.
(function (root) {
  const MAX_ATTEMPTS = 4;
  const MAX_ITEMS = 10000, MAX_BYTES = 32 * 1024 * 1024;
  const unsupportedChanges = new WeakSet(); // request is scoped to one bridge/server connection
  const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

  // Same revision AND same content: personal state (watched, rating) lives inside `media` and can change on another device
  // without the media revision moving, so the revision alone is not a safe change detector.
  const signature = e => e.revision + "\u0000" + JSON.stringify(e.media);

  function diff(previous, next) {
    const before = new Map(previous.map(e => [e.media.id, signature(e)]));
    const upsert = [];
    const seen = new Set();
    for (const e of next) {
      seen.add(e.media.id);
      if (before.get(e.media.id) !== signature(e)) upsert.push(e);
    }
    const remove = [];
    for (const id of before.keys()) if (!seen.has(id)) remove.push(id);
    return { upsert, remove };
  }

  const aborted = signal => {
    if (signal && signal.aborted) throw Object.assign(new Error("Synchronisation annulée."), { code: "ABORTED" });
  };
  const routeMissing = e => e && e.status === 404 && e.code === "ROUTE_NOT_FOUND";
  function bad(message, code) { return Object.assign(new Error(message), { code }); }
  function pageBytes(page, bytes) {
    const next = bytes + new TextEncoder().encode(JSON.stringify(page)).byteLength;
    if (next > MAX_BYTES) throw bad("Catalogue trop volumineux.", "SYNC_CATALOG_TOO_LARGE");
    return next;
  }

  async function changesOnce(request, profile, knownRevision, { signal, onPage } = {}) {
    aborted(signal);
    const path = `/profiles/${profile}/catalog/changes`;
    const first = await request("POST", path, { body: { known_revision: knownRevision || null }, signal });
    if (first.unchanged) {
      if (knownRevision !== null && knownRevision !== undefined && first.revision !== knownRevision) throw bad("Révision de catalogue incohérente.", "SYNC_REVISION");
      return { unchanged: true, revision: first.revision };
    }
    if (!first.snapshot_id || !Number.isInteger(first.total) || first.total < 0 || first.total > MAX_ITEMS || !Array.isArray(first.items) || first.items.length > 100 || !Array.isArray(first.deleted_ids) || first.deleted_ids.length > MAX_ITEMS) throw bad("Catalogue invalide.", "SYNC_INVALID");
    const revision = first.revision, reset = first.reset === true, entries = [...first.items], deleted = [...first.deleted_ids];
    const ids = new Set(entries.map(e => e && e.media && e.media.id));
    if (ids.has(undefined) || ids.size !== entries.length) throw bad("Page de catalogue invalide.", "SYNC_INCOMPLETE");
    let bytes = pageBytes(first, 0), page = first, lastOffset = -1;
    while (page.has_more === true) {
      aborted(signal);
      const offset = page.next_offset;
      if (!Number.isInteger(offset) || offset !== entries.length || offset <= lastOffset || offset >= first.total) throw bad("Page de catalogue incohérente.", "SYNC_INCOMPLETE");
      lastOffset = offset;
      try { page = await request("GET", `${path}/${first.snapshot_id}`, { query: [["offset", String(offset)]], signal }); }
      catch (e) { if (e.status === 404 || e.status === 410) e.expiredSnapshot = true; throw e; }
      if (page.revision !== revision || !Array.isArray(page.items) || !page.items.length || page.items.length > 100 || page.reset === true || (page.deleted_ids && page.deleted_ids.length)) throw bad("Page de catalogue invalide.", "SYNC_INCOMPLETE");
      for (const entry of page.items) { const id = entry && entry.media && entry.media.id; if (!id || ids.has(id)) throw bad("Page de catalogue incohérente.", "SYNC_INCOMPLETE"); ids.add(id); }
      bytes = pageBytes(page, bytes); entries.push(...page.items);
      if (entries.length > MAX_ITEMS) throw bad("Catalogue trop volumineux.", "SYNC_CATALOG_TOO_LARGE");
      if (onPage) onPage(entries.length, first.total);
    }
    if (entries.length !== first.total) throw bad("Copie du catalogue incomplète.", "SYNC_INCOMPLETE");
    return { unchanged: false, reset, revision, entries, deleted_ids: deleted, sagas: first.sagas, playlists: first.playlists, series_revisions: first.series_revisions, season_metadata: first.season_metadata, total: first.total };
  }

  async function changes(request, profile, knownRevision, options) {
    try { return await changesOnce(request, profile, knownRevision, options); }
    catch (e) { if (!e.expiredSnapshot) throw e; return changesOnce(request, profile, knownRevision, options); }
  }

  async function downloadOnce(request, profile, knownRevision, { signal, onPage } = {}) {
    let first;
    for (let attempt = 1; ; attempt++) {
      aborted(signal);
      try {
        first = await request("POST", `/profiles/${profile}/catalog/snapshot`, { body: { known_revision: knownRevision || null }, signal });
        break;
      } catch (e) {
        const retryable = e.status === 409 || e.status === 429 || e.status === 503;
        if (!retryable || attempt >= MAX_ATTEMPTS) throw e;
        await sleep(e.status === 409 ? 100 : 400 * attempt);
      }
    }
    if (first.unchanged) return { unchanged: true, revision: first.revision };
    if (!first.snapshot_id || !Number.isInteger(first.total) || first.total < 0 || first.total > MAX_ITEMS || !Array.isArray(first.items) || first.items.length > 100) throw bad("Catalogue invalide.", "SYNC_INVALID");
    const entries = [...first.items];
    const ids = new Set(entries.map(e => e && e.media && e.media.id));
    if (ids.has(undefined) || ids.size !== entries.length) throw bad("Page de catalogue invalide.", "SYNC_INCOMPLETE");
    let bytes = pageBytes(first, 0), lastOffset = -1;
    let { has_more: more, next_offset: offset } = first;
    while (more) {
      aborted(signal);
      if (!Number.isInteger(offset) || offset !== entries.length || offset <= lastOffset || offset >= first.total) throw bad("Page de catalogue incohérente.", "SYNC_INCOMPLETE");
      lastOffset = offset;
      let page;
      try { page = await request("GET", `/profiles/${profile}/catalog/snapshot/${first.snapshot_id}`, { query: [["offset", String(offset)]], signal }); }
      catch (e) { if (e.status === 404 || e.status === 410) e.expiredSnapshot = true; throw e; }   // only a PAGE can mean "snapshot gone"
      if (page.revision !== first.revision || !Array.isArray(page.items) || !page.items.length || page.items.length > 100) throw bad("Page de catalogue invalide.", "SYNC_INCOMPLETE");
      for (const entry of page.items) { const id = entry && entry.media && entry.media.id; if (!id || ids.has(id)) throw bad("Page de catalogue incohérente.", "SYNC_INCOMPLETE"); ids.add(id); }
      bytes = pageBytes(page, bytes);
      entries.push(...page.items);
      if (entries.length > MAX_ITEMS) throw bad("Catalogue trop volumineux.", "SYNC_CATALOG_TOO_LARGE");
      if (onPage) onPage(entries.length, first.total);
      more = page.has_more; offset = page.next_offset;
    }
    if (entries.length !== first.total) throw Object.assign(new Error("Instantané incomplet."), { code: "SYNC_INCOMPLETE" });
    return { unchanged: false, reset: true, revision: first.revision, entries, deleted_ids: [], sagas: first.sagas, playlists: first.playlists, series_revisions: first.series_revisions, season_metadata: first.season_metadata, total: first.total };
  }

  // An expired or unknown snapshot (404/410, e.g. the server restarted between two pages) restarts the download once.
  async function download(request, profile, knownRevision, options) {
    try { return await downloadOnce(request, profile, knownRevision, options); }
    catch (e) {
      if (!e.expiredSnapshot) throw e;   // a 404 on the first call means the profile is gone: surface it
      return downloadOnce(request, profile, knownRevision, options);
    }
  }

  // One synchronisation round for one mirror. `local` = what the store (or memory) holds now, or null.
  // Returns { unchanged, changed, removed, firstSync, revision, entries, doc } ; `entries` is the full new list.
  async function sync({ request, store, key, profile, local, signal, onPage, now = () => Date.now(), preferDelta = true }) {
    const known = local && local.manifest ? local.manifest.revision : null;
    let result;
    if (preferDelta && !unsupportedChanges.has(request)) {
      try { result = await changes(request, profile, known, { signal, onPage }); }
      catch (e) { if (!routeMissing(e)) throw e; unsupportedChanges.add(request); result = await download(request, profile, known, { signal, onPage }); }
    } else result = await download(request, profile, known, { signal, onPage });
    if (result.unchanged) {
      if (local && local.manifest) await store.commit(key, { manifest: { ...local.manifest, checked_at: now() }, upsert: [], remove: [] });
      return { unchanged: true, changed: 0, removed: 0, firstSync: false, revision: result.revision };
    }
    aborted(signal);
    const previous = local ? local.entries : [];
    const next = result.reset ? result.entries : (() => { const map = new Map(previous.map(e => [e.media.id, e])); for (const id of result.deleted_ids) map.delete(id); for (const e of result.entries) map.set(e.media.id, e); return [...map.values()]; })();
    const { upsert, remove } = result.reset ? diff(previous, next) : { upsert: result.entries, remove: result.deleted_ids };
    const doc = { sagas: result.sagas, playlists: result.playlists, series_revisions: result.series_revisions, season_metadata: result.season_metadata };
    aborted(signal);
    await store.commit(key, { manifest: { revision: result.revision, total: next.length, synced_at: now(), checked_at: now() }, upsert, remove, doc });
    return { unchanged: false, changed: upsert.length, removed: remove.length, firstSync: !local, revision: result.revision, entries: next, doc };
  }

  const api = { sync, diff, download, changes, signature };
  if (typeof module !== "undefined") module.exports = api; else root.ChillCatalogSync = api;
})(typeof window !== "undefined" ? window : globalThis);
