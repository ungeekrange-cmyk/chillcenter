"use strict";
// Local catalogue queries over a mirrored snapshot (ADR-0019). Port of `Store::browse_catalog`
// (crates/chill-domain/src/catalog_browser.rs), `Store::series` and the collections of the sync document:
// same filters, same grouping of series, same orderings, same counters. SQLite semantics that matter here:
//   * COLLATE NOCASE and LIKE only fold ASCII letters (not é/É),
//   * ORDER BY ... ASC puts NULL first and DESC puts NULL last, `x IS NULL` sorts non-null first,
//   * the tie-break "legacy" id is the relative file path (verified against the real server).
(function (root) {
  const SORTS = ["title_asc", "title_desc", "year_asc", "year_desc", "added_desc", "rating_desc"];
  const CATEGORIES = ["film", "serie", "spectacle", "documentaire"];
  const MAX_PAGE = 100;

  const foldAscii = s => String(s).replace(/[A-Z]/g, c => c.toLowerCase());
  const cmpText = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

  function seriesLabel(m) { return m.series ?? m.title; }
  // Keep this deliberately separate from foldAscii: this is the Rust scanner's
  // conservative identity rule, while browse/search retain SQLite's ASCII rules.
  function normalized(value) {
    return String(value).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  }
  function titleKey(value) { return normalized(value).trim().split(/\s+/).filter(Boolean).join(" "); }
  function seriesRoot(m) {
    const pieces = String(m.path ?? "").split("/").filter(Boolean);
    let parents = pieces.slice(0, -1);
    const leaf = (parents[parents.length - 1] ?? "").replace(/[_.]/g, " ");
    const key = titleKey(leaf);
    if (/^(season|saison)\s/.test(key) || /^s\d+$/.test(key)) parents = parents.slice(0, -1);
    return parents.map(titleKey).join("\x1f");
  }
  function seriesGroupKey(m) {
    if (m.category !== "serie") return null;
    return m.tmdb_id !== null && m.tmdb_id !== undefined ? `tmdb:tv:${m.tmdb_id}` : `source:${titleKey(seriesLabel(m))}\x1f${seriesRoot(m)}`;
  }

  // Derived, query-friendly rows. Cheap (2.5k items): rebuilt after every sync, never persisted.
  function prepare(entries) {
    const rows = entries.map(e => {
      const m = e.media;
      const meta = m.series_metadata;
      return {
        m, id: m.id, revision: e.revision, category: m.category, seriesLabel: m.category === "serie" ? seriesLabel(m) : null, seriesGroup: seriesGroupKey(m),
        sortYear: (meta && meta.year !== null && meta.year !== undefined) ? meta.year : (m.year ?? null),
        genres: (meta && Array.isArray(meta.genres)) ? meta.genres : (Array.isArray(m.genres) ? m.genres : []),
        seen: m.watched ? 1 : 0, rating: m.rating ?? null, added: m.added_at ?? null,
        title: m.title, lowerTitle: foldAscii(m.title), season: m.season ?? null,
        legacy: m.path ?? m.id,   // the server's tie-break (`legacy` id) is the relative path of the file
      };
    });
    rows.forEach(r => { r.lowerSeriesLabel = r.seriesLabel === null ? null : foldAscii(r.seriesLabel); });
    const bySeries = new Map(); // all rows of a series, unfiltered (counters are computed before filters)
    for (const r of rows) {
      if (r.category !== "serie") continue;
      let s = bySeries.get(r.seriesGroup);
      if (!s) { s = { allSeen: 1, seasons: new Set(), episodes: 0, unwatched: 0, representative: r }; bySeries.set(r.seriesGroup, s); }
      s.allSeen = Math.min(s.allSeen, r.seen);
      s.seasons.add(r.season ?? -1);
      s.episodes += 1; s.unwatched += 1 - r.seen;
      if (r.legacy < s.representative.legacy) s.representative = r;
    }
    const byId = new Map(rows.map(r => [r.id, r]));
    return { rows, bySeries, byId, memo: new Map() };   // memo: sorted result lists by filter set (the UI pages through 50+ pages)
  }

  function validate(q) {
    const limit = q.limit ? q.limit : 48;
    if (!Number.isInteger(limit) || limit < 1 || limit > MAX_PAGE) throw new Error("Filtre ou page catalogue invalide.");
    if (q.sort && !SORTS.includes(q.sort)) throw new Error("Tri inconnu.");
    if (q.category && !CATEGORIES.includes(q.category)) throw new Error("Filtre ou page catalogue invalide.");
    if (q.year !== null && q.year !== undefined && q.year !== 0 && !(q.year >= 1850 && q.year <= 2200)) throw new Error("Filtre ou page catalogue invalide.");
    if (q.q && new TextEncoder().encode(q.q).length > 500) throw new Error("Filtre ou page catalogue invalide.");
    if (q.genre && new TextEncoder().encode(q.genre).length > 200) throw new Error("Filtre ou page catalogue invalide.");
    return limit;
  }

  function browse(index, q) {
    const limit = validate(q);
    const offset = Math.max(0, Number(q.offset) || 0);
    const grouped = Boolean(q.grouped);
    const needle = q.q ? foldAscii(q.q) : "";
    const hasCategory = Boolean(q.category), hasGenre = Boolean(q.genre);
    const hasYear = q.year !== null && q.year !== undefined;
    const hasAvail = q.available !== null && q.available !== undefined;
    const hasWatched = q.watched !== null && q.watched !== undefined;
    const wantSeen = hasWatched ? (q.watched ? 1 : 0) : null;
    const wantAvail = hasAvail ? Boolean(q.available) : null;

    // eligible rows (per-row filters), then one representative per series (grouped) or per media.
    const sig = JSON.stringify([q.category ?? null, needle, q.genre ?? null, hasYear ? q.year : null, wantSeen, wantAvail, q.sort ?? null, grouped]);
    let list = index.memo.get(sig);
    if (!list) { list = computeList(); index.memo.set(sig, list); if (index.memo.size > 8) index.memo.delete(index.memo.keys().next().value); }
    return slice(list);

    function computeList() {
    const reps = new Map(); // partition key -> row with the smallest legacy id among eligible rows
    for (const r of index.rows) {
      if (hasCategory && r.category !== q.category) continue;
      if (needle && !(r.lowerTitle.includes(needle) || (r.lowerSeriesLabel !== null && r.lowerSeriesLabel.includes(needle)))) continue;
      if (hasGenre && !r.genres.includes(q.genre)) continue;
      if (hasYear && !(r.sortYear === q.year || (q.year === 0 && r.sortYear === null))) continue;
      if (hasAvail && r.m.available !== wantAvail) continue;
      const series = grouped && r.category === "serie";
      const visibleSeen = series ? index.bySeries.get(r.seriesGroup).allSeen : r.seen;
      if (hasWatched && visibleSeen !== wantSeen) continue;
      const key = series ? "series:" + r.seriesGroup : "id:" + r.id;
      const current = reps.get(key);
      if (!current || r.legacy < current.legacy) reps.set(key, r);
    }
    const list = [...reps.values()];
    const label = r => (grouped && r.category === "serie" ? r.seriesLabel : r.title);
    const lab = r => foldAscii(label(r));
    const tie = (a, b) => cmpText(lab(a), lab(b)) || cmpText(a.legacy, b.legacy);
    const nullFirstAsc = (a, b) => (a === null) - (b === null); // `x IS NULL` ascending: non-null first
    let order;
    switch (q.sort || null) {
      case null: order = (a, b) => cmpText(a.legacy, b.legacy); break;
      case "title_asc": order = tie; break;
      case "title_desc": order = (a, b) => cmpText(lab(b), lab(a)) || cmpText(a.legacy, b.legacy); break;
      case "year_asc": order = (a, b) => nullFirstAsc(a.sortYear, b.sortYear) || (a.sortYear ?? 0) - (b.sortYear ?? 0) || tie(a, b); break;
      case "year_desc": order = (a, b) => nullFirstAsc(a.sortYear, b.sortYear) || (b.sortYear ?? 0) - (a.sortYear ?? 0) || tie(a, b); break;
      case "added_desc": order = (a, b) => descNullLast(a.added, b.added) || tie(a, b); break;
      case "rating_desc": order = (a, b) => nullFirstAsc(a.rating, b.rating) || (b.rating ?? 0) - (a.rating ?? 0) || tie(a, b); break;
      default: throw new Error("Tri inconnu.");
    }
    list.sort(order);
    return list;
    }

    function slice(list) {
    const page = list.slice(offset, offset + limit + 1);
    const hasMore = page.length > limit;
    if (hasMore) page.pop();
    const counters = {};
    const items = page.map(r => {
      const m = { ...r.m };
      if (grouped && r.category === "serie") {
        const s = index.bySeries.get(r.seriesGroup);
        m.title = m.series ?? m.title;
        m.watched = Boolean(s.allSeen);
        counters[m.id] = { series_key: r.m.tmdb_id !== null && r.m.tmdb_id !== undefined ? `tmdb:tv:${r.m.tmdb_id}` : `media:${r.id}`, seasons: s.seasons.size, episodes: s.episodes, unwatched: s.unwatched };
        if (m.series_metadata) { m.overview = m.series_metadata.overview; m.genres = m.series_metadata.genres; m.year = m.series_metadata.year; }
        else m.overview = m.series_overview;
      }
      return m;
    });
    return { items, has_more: hasMore, series_counters: counters };
    }
  }
  // DESC ordering with NULL last (SQLite: NULL is the smallest value).
  function descNullLast(a, b) {
    const an = a === null || a === undefined, bn = b === null || b === undefined;
    if (an && bn) return 0;
    if (an) return 1;
    if (bn) return -1;
    return b - a;
  }

  // `GET /profiles/{p}/series/{name}`: episodes ordered by (season, episode, title), counters, first member's metadata.
  function series(index, name, revision, selector = "title") {
    const keyedGroup = selector === "key" && name.startsWith("media:") ? index.byId.get(name.slice("media:".length))?.seriesGroup : name;
    const members = index.rows
      .filter(r => r.category === "serie" && (selector === "key" ? r.seriesGroup === keyedGroup : r.seriesLabel === name))
      .map(r => r.m)
      .sort((a, b) => ((a.season ?? 2147483647) - (b.season ?? 2147483647)) || ((a.episode ?? 2147483647) - (b.episode ?? 2147483647)) || cmpText(a.title, b.title));
    if (!members.length) return null;
    const watched = members.filter(m => m.watched).length;
    return { name, revision: revision ?? null, metadata: members[0].series_metadata ?? null, episodes: members, counters: { total: members.length, watched, unwatched: members.length - watched } };
  }

  function common(a, b) {
    const have = new Set((a ?? []).map(v => String(v).trim().toLowerCase()).filter(Boolean));
    const seen = new Set();
    return (b ?? []).map(v => String(v).trim()).filter(v => { const k = v.toLowerCase(); return have.has(k) && !seen.has(k) && (seen.add(k), true); });
  }
  // Local mirror equivalent of Store::recommendations: entities, never episodes.
  function recommendations(index, mediaId) {
    const sourceRow = index.byId.get(mediaId);
    if (!sourceRow) return [];
    const enrich = r => {
      const m = { ...r.m };
      if (m.category === "serie") {
        const meta = m.series_metadata;
        if (meta) { m.genres = meta.genres ?? []; m.cast = meta.cast ?? []; m.directors = meta.directors ?? []; m.year = meta.year ?? null; m.overview = meta.overview ?? ""; }
      }
      return m;
    };
    const source = enrich(sourceRow);
    const sourceGroup = sourceRow.seriesGroup;
    const scored = [];
    for (const r of index.rows) {
      const group = r.category === "serie" ? r.seriesGroup : `video:${r.id}`;
      if ((r.category === "serie" && index.bySeries.get(group).representative !== r) || !r.m.available || r.id === mediaId || (sourceGroup && group === sourceGroup)) continue;
      if (r.category === "serie" && (!r.m.series || !r.m.series.trim())) continue;
      if (r.category !== "serie" && (r.m.season !== null && r.m.season !== undefined || r.m.episode !== null && r.m.episode !== undefined)) continue;
      const m = enrich(r); const matches = []; let score = 0;
      for (const [label, values] of [["Genres", common(source.genres, m.genres)], ["Réalisation", common(source.directors, m.directors)], ["Distribution", common(source.cast, m.cast)]]) if (values.length) { score += values.length; matches.push(`${label} : ${values.join(", ")}`); }
      if (!score) continue;
      if (m.category === "serie") { const s = index.bySeries.get(group); m.title = r.seriesLabel; m.season = null; m.episode = null; m.watched = Boolean(s.allSeen); m.rating = null; }
      scored.push({ score, media: m, series_key: m.category === "serie" ? (m.tmdb_id !== null && m.tmdb_id !== undefined ? `tmdb:tv:${m.tmdb_id}` : `media:${r.id}`) : null, matches });
    }
    return scored.sort((a, b) => b.score - a.score || cmpText(a.media.title.toLowerCase(), b.media.title.toLowerCase()) || cmpText(a.media.id, b.media.id)).slice(0, 60).map(({ score, ...item }) => item);
  }

  const api = { prepare, browse, series, recommendations, foldAscii, SORTS, CATEGORIES };
  if (typeof module !== "undefined") module.exports = api; else root.ChillCatalogQuery = api;
})(typeof window !== "undefined" ? window : globalThis);
