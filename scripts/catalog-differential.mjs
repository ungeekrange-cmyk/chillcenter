// Differential check of web/src/catalog-query.js against a REAL server (read-only: GETs and one snapshot POST).
//   node scripts/catalog-differential.mjs <host> <port> <profile-id> [max-queries]
// Downloads the snapshot, then runs a matrix of catalogue queries both on the server and locally and compares the answers
// (items, order, has_more, series counters). Prints only ids/counts on mismatch, never titles.
import http from "node:http";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
const require = createRequire(import.meta.url);
const web = join(dirname(fileURLToPath(import.meta.url)), "..", "web", "src");
const Query = require(join(web, "catalog-query.js")), Json64 = require(join(web, "json64.js"));
const [host, port, profile, maxArg] = process.argv.slice(2);
if (!host || !port || !profile) { console.error("usage: catalog-differential.mjs <host> <port> <profile> [max]"); process.exit(2); }
const BASE = "/api/2026-10-14";
const call = (method, path, body) => new Promise((resolve, reject) => {
  const req = http.request({ host, port: Number(port), path: BASE + path, method, timeout: 60000, headers: body ? { "Content-Type": "application/json", Origin: `http://${host}${port === "80" ? "" : ":" + port}` } : {} }, res => {
    const parts = []; res.on("data", d => parts.push(d)); res.on("end", () => resolve({ status: res.statusCode, body: Buffer.concat(parts).toString() }));
  });
  req.on("error", reject); req.on("timeout", () => req.destroy(new Error("timeout")));
  if (body) req.write(JSON.stringify(body)); req.end();
});
const first = await call("POST", `/profiles/${profile}/catalog/snapshot`, {});
if (first.status !== 200) { console.error("snapshot", first.status); process.exit(2); }
const snap = Json64.parse(first.body);
const entries = [...snap.items];
for (let off = snap.next_offset; snap.has_more && off !== null && off !== undefined;) {
  const page = Json64.parse((await call("GET", `/profiles/${profile}/catalog/snapshot/${snap.snapshot_id}?offset=${off}`)).body);
  entries.push(...page.items); if (!page.has_more) break; off = page.next_offset;
}
console.log(`snapshot: ${entries.length}/${snap.total} items, ${snap.sagas.length} sagas, ${snap.playlists.length} playlists`);
const index = Query.prepare(entries);
const genres = [...new Set(entries.flatMap(e => e.media.genres || []))].slice(0, 3);
const years = [...new Set(entries.map(e => e.media.year).filter(Boolean))].slice(0, 2);
const words = ["a", "the", "é", "%", "_", "S0", " "]; // ASCII/non-ASCII/LIKE metacharacters
const sorts = [null, "title_asc", "title_desc", "year_asc", "year_desc", "added_desc", "rating_desc"];
const cases = [];
for (const grouped of [false, true]) for (const sort of sorts) cases.push({ grouped, sort, offset: 0 });
for (const category of ["film", "serie", "spectacle", "documentaire"]) for (const grouped of [false, true]) cases.push({ category, grouped, sort: "title_asc", offset: 0 }, { category, grouped, sort: "year_desc", offset: 48 });
for (const watched of [true, false]) for (const grouped of [false, true]) cases.push({ watched, grouped, sort: "title_asc", offset: 0 });
for (const available of [true, false]) cases.push({ available, sort: "title_asc", offset: 0 });
for (const year of [0, ...years]) for (const grouped of [false, true]) cases.push({ year, grouped, sort: "added_desc", offset: 0 });
for (const genre of genres) cases.push({ genre, grouped: true, sort: "title_asc", offset: 0 }, { genre, grouped: false, sort: "rating_desc", offset: 0 });
for (const q of words) cases.push({ q, grouped: true, sort: "title_asc", offset: 0 }, { q, grouped: false, sort: "year_asc", offset: 0 });
cases.push({ grouped: true, sort: "title_asc", offset: 96 }, { grouped: true, sort: "title_asc", offset: 2400 }, { grouped: false, sort: "title_desc", offset: 480 });
cases.push({ category: "serie", watched: false, genre: genres[0], grouped: true, sort: "title_asc", offset: 0, q: "a" });
const max = maxArg ? Number(maxArg) : cases.length;
const norm = v => JSON.stringify(v, (k, x) => (x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.keys(x).sort().map(key => [key, x[key]])) : x));
let bad = 0, ran = 0;
for (const c of cases.slice(0, max)) {
  const q = [["offset", c.offset], ["limit", 48]];
  for (const k of ["category", "q", "genre", "year", "watched", "sort", "grouped", "available"]) if (c[k] !== undefined && c[k] !== null) q.push([k, c[k]]);
  const res = await call("GET", `/profiles/${profile}/catalog?` + q.map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`).join("&"));
  if (res.status !== 200) { console.log("SERVER", res.status, JSON.stringify(c)); bad++; continue; }
  const server = Json64.parse(res.body);
  let local; try { local = Query.browse(index, { ...c, limit: 48 }); } catch (e) { console.log("LOCAL ERROR", e.message, JSON.stringify(c)); bad++; continue; }
  ran++;
  const sIds = server.items.map(m => m.id), lIds = local.items.map(m => m.id);
  const sameIds = sIds.join() === lIds.join(), sameMore = server.has_more === local.has_more, sameCounters = norm(server.series_counters) === norm(local.series_counters);
  const sameItems = sameIds && norm(server.items) === norm(local.items);
  if (!(sameIds && sameMore && sameCounters && sameItems)) {
    bad++;
    const firstDiff = sIds.findIndex((id, i) => id !== lIds[i]);
    console.log("DIFF", JSON.stringify(c), { server: sIds.length, local: lIds.length, sameIds, sameMore, sameCounters, sameItems, firstDiff, serverId: sIds[firstDiff], localId: lIds[firstDiff] });
    if (sameIds && !sameItems) { const i = server.items.findIndex((m, k) => norm(m) !== norm(local.items[k])); const a = server.items[i], b = local.items[i]; console.log("  item fields differing:", Object.keys({ ...a, ...b }).filter(k => norm(a[k]) !== norm(b[k]))); }
  }
}
console.log(`résultat : ${ran} requêtes comparées, ${bad} écart(s)`);
process.exit(bad ? 1 : 0);
