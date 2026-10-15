"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Q = require("../src/catalog-query.js");

let n = 0;
const media = (o = {}) => { n += 1; return { id: `id-${String(n).padStart(3, "0")}`, path: `p/${String(n).padStart(3, "0")}.mkv`, title: `T${n}`, category: "film", year: null, genres: [], watched: false, rating: null, added_at: null, available: true, series: null, season: null, episode: null, series_metadata: null, series_overview: "", overview: "", ...o }; };
const entry = (o, revision = 1) => ({ media: media(o), revision });
const ids = page => page.items.map(m => m.title);
const run = (entries, q) => Q.browse(Q.prepare(entries), { limit: 48, offset: 0, ...q });

test("sorting follows SQLite: NOCASE folds ASCII only, NULL years last in both directions, ties by file path", () => {
  const e = [entry({ title: "b", year: 2001 }), entry({ title: "A", year: null }), entry({ title: "é", year: 1999 }), entry({ title: "É", year: 2005 }), entry({ title: "a", year: 2001 })];
  assert.deepEqual(ids(run(e, { sort: "title_asc" })), ["A", "a", "b", "É", "é"], "A and a tie once folded: file path decides; É sorts before é (not folded)");
  assert.deepEqual(ids(run(e, { sort: "title_desc" })), ["é", "É", "b", "A", "a"], "ties keep the ascending path order");
  assert.deepEqual(ids(run(e, { sort: "year_asc" })), ["é", "a", "b", "É", "A"], "NULL year last");
  assert.deepEqual(ids(run(e, { sort: "year_desc" })), ["É", "a", "b", "é", "A"], "NULL year last in descending order too");
});

test("search is an ASCII case-insensitive substring over title and series name; LIKE wildcards are literal", () => {
  const e = [entry({ title: "Dune" }), entry({ title: "Dune 2" }), entry({ title: "100%" }), entry({ title: "a_b" }), entry({ title: "axb" }), entry({ title: "Élan" }), entry({ title: "Ep 1", category: "serie", series: "Dune Saga" })];
  assert.deepEqual(ids(run(e, { q: "DUNE", sort: "title_asc" })), ["Dune", "Dune 2", "Ep 1"]);
  assert.deepEqual(ids(run(e, { q: "%" })), ["100%"]);
  assert.deepEqual(ids(run(e, { q: "_" })), ["a_b"]);
  assert.deepEqual(ids(run(e, { q: "\\" })), []);
  assert.deepEqual(ids(run(e, { q: "élan" })), [], "É is not folded to é (SQLite LIKE is ASCII-only)");
  assert.deepEqual(ids(run(e, { q: "Élan" })), ["Élan"]);
});

test("filters: category, genre (exact), year with 0 meaning unknown, watched, available", () => {
  const e = [entry({ title: "a", category: "film", genres: ["Action"], year: 2000, watched: true }), entry({ title: "b", category: "documentaire", genres: ["Action", "Drame"], year: null }), entry({ title: "c", genres: ["action"], available: false })];
  assert.deepEqual(ids(run(e, { category: "documentaire" })), ["b"]);
  assert.deepEqual(ids(run(e, { genre: "Action", sort: "title_asc" })), ["a", "b"]);
  assert.deepEqual(ids(run(e, { year: 0 })), ["b", "c"]);
  assert.deepEqual(ids(run(e, { year: 2000 })), ["a"]);
  assert.deepEqual(ids(run(e, { watched: true })), ["a"]);
  assert.deepEqual(ids(run(e, { watched: false, sort: "title_asc" })), ["b", "c"]);
  assert.deepEqual(ids(run(e, { available: false })), ["c"]);
  assert.throws(() => run(e, { category: "nope" }), /invalide/);
  assert.throws(() => run(e, { sort: "random" }), /Tri inconnu/);
  assert.throws(() => run(e, { year: 1700 }), /invalide/);
  assert.throws(() => run(e, { limit: 101 }), /invalide/);
});

test("grouped series: one card per series, counters computed before filters, representative = smallest path", () => {
  const meta = { year: 2010, genres: ["Comédie"], overview: "Résumé de série" };
  const ep = (title, season, watched, p) => entry({ title, category: "serie", series: "One", season, episode: 1, watched, series_metadata: meta, path: p, genres: ["x"] });
  const e = [ep("Pilote", 1, true, "s/b"), ep("Finale", 2, false, "s/a"), entry({ title: "Film", category: "film" })];
  const page = run(e, { grouped: true, sort: "title_asc" });
  assert.deepEqual(ids(page), ["Film", "One"]);
  const card = page.items.find(m => m.title === "One");
  assert.equal(card.path, "s/a", "representative: smallest path");
  assert.equal(card.watched, false, "all_seen = MIN over the whole series");
  assert.deepEqual(page.series_counters[card.id], { seasons: 2, episodes: 2, unwatched: 1 });
  assert.equal(card.overview, "Résumé de série"); assert.deepEqual(card.genres, ["Comédie"]); assert.equal(card.year, 2010);
  // a search matching ONE episode still returns the series card with FULL counters
  const found = run(e, { grouped: true, q: "Pilote" });
  assert.deepEqual(ids(found), ["One"]); assert.equal(found.series_counters[found.items[0].id].episodes, 2);
  // watched filter on a series uses the whole-series state, not the matching episode
  assert.deepEqual(ids(run(e, { grouped: true, watched: true })), []);
  assert.deepEqual(ids(run(e, { grouped: true, watched: false, sort: "title_asc" })), ["Film", "One"]);
  // ungrouped: every episode is its own card, no counters
  const flat = run(e, { grouped: false });
  assert.equal(flat.items.length, 3); assert.deepEqual(flat.series_counters, {});
});

test("paging: has_more from limit+1, offsets, default limit 48; items are copies (the mirror is never mutated)", () => {
  const e = Array.from({ length: 120 }, (_, i) => entry({ title: `F${String(i).padStart(3, "0")}` }));
  const idx = Q.prepare(e);
  const first = Q.browse(idx, { sort: "title_asc" });
  assert.equal(first.items.length, 48); assert.equal(first.has_more, true);
  const last = Q.browse(idx, { sort: "title_asc", offset: 96, limit: 48 });
  assert.equal(last.items.length, 24); assert.equal(last.has_more, false);
  assert.equal(Q.browse(idx, { sort: "title_asc", offset: 500 }).items.length, 0);
  first.items[0].title = "mutated";
  assert.notEqual(Q.browse(idx, { sort: "title_asc" }).items[0].title, "mutated");
});

test("series view: episodes ordered by season, episode, title; counters; unknown series is null", () => {
  const e = [entry({ title: "z", category: "serie", series: "S", season: 2, episode: 1 }), entry({ title: "b", category: "serie", series: "S", season: 1, episode: 2, watched: true }), entry({ title: "a", category: "serie", series: "S", season: 1, episode: 1 }), entry({ title: "Solo", category: "serie" })];
  const view = Q.series(Q.prepare(e), "S", 7);
  assert.deepEqual(view.episodes.map(m => m.title), ["a", "b", "z"]);
  assert.deepEqual(view.counters, { total: 3, watched: 1, unwatched: 2 }); assert.equal(view.revision, 7);
  assert.equal(Q.series(Q.prepare(e), "Solo", 1).episodes.length, 1, "a series without a name falls back to the title");
  assert.equal(Q.series(Q.prepare(e), "Nope", 1), null);
});
