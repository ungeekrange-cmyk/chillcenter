"use strict";
// Background synchronisation of the catalogue mirror (ADR-0019), over the existing snapshot API:
//   POST /profiles/{p}/catalog/snapshot {known_revision}  -> {unchanged:true}            (15 ms on the Pi)
//                                                         -> first page + snapshot_id      (full snapshot otherwise)
//   GET  /profiles/{p}/catalog/snapshot/{id}?offset=N     -> next page of 100
// The transfer is the server's whole snapshot when anything changed, but only the DIFFERENCE is written locally and
// published to the UI. Errors follow the Mac client: 409 SYNC_SNAPSHOT_CHANGED, 410/404 expired snapshot, 429 busy.
(function (root) {
  const MAX_ATTEMPTS = 4;
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

  async function downloadOnce(request, profile, knownRevision, { signal, onPage } = {}) {
    let first;
    for (let attempt = 1; ; attempt++) {
      if (signal && signal.aborted) throw Object.assign(new Error("Synchronisation annulée."), { code: "ABORTED" });
      try {
        first = await request("POST", `/profiles/${profile}/catalog/snapshot`, { body: { known_revision: knownRevision || null } });
        break;
      } catch (e) {
        const retryable = e.status === 409 || e.status === 429 || e.status === 503;
        if (!retryable || attempt >= MAX_ATTEMPTS) throw e;
        await sleep(e.status === 409 ? 100 : 400 * attempt);
      }
    }
    if (first.unchanged) return { unchanged: true, revision: first.revision };
    const entries = [...first.items];
    let { has_more: more, next_offset: offset } = first;
    while (more) {
      if (signal && signal.aborted) throw Object.assign(new Error("Synchronisation annulée."), { code: "ABORTED" });
      let page;
      try { page = await request("GET", `/profiles/${profile}/catalog/snapshot/${first.snapshot_id}`, { query: [["offset", String(offset)]] }); }
      catch (e) { if (e.status === 404 || e.status === 410) e.expiredSnapshot = true; throw e; }   // only a PAGE can mean "snapshot gone"
      entries.push(...page.items);
      if (onPage) onPage(entries.length, first.total);
      more = page.has_more; offset = page.next_offset;
    }
    if (entries.length !== first.total) throw Object.assign(new Error("Instantané incomplet."), { code: "SYNC_INCOMPLETE" });
    return { unchanged: false, revision: first.revision, entries, sagas: first.sagas, playlists: first.playlists, series_revisions: first.series_revisions, total: first.total };
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
  async function sync({ request, store, key, profile, local, signal, onPage, now = () => Date.now() }) {
    const known = local && local.manifest ? local.manifest.revision : null;
    const result = await download(request, profile, known, { signal, onPage });
    if (result.unchanged) {
      if (local && local.manifest) await store.commit(key, { manifest: { ...local.manifest, checked_at: now() }, upsert: [], remove: [] });
      return { unchanged: true, changed: 0, removed: 0, firstSync: false, revision: result.revision };
    }
    const previous = local ? local.entries : [];
    const { upsert, remove } = diff(previous, result.entries);
    const doc = { sagas: result.sagas, playlists: result.playlists, series_revisions: result.series_revisions };
    await store.commit(key, { manifest: { revision: result.revision, total: result.total, synced_at: now(), checked_at: now() }, upsert, remove, doc });
    return { unchanged: false, changed: upsert.length, removed: remove.length, firstSync: !local, revision: result.revision, entries: result.entries, doc };
  }

  const api = { sync, diff, download, signature };
  if (typeof module !== "undefined") module.exports = api; else root.ChillCatalogSync = api;
})(typeof window !== "undefined" ? window : globalThis);
