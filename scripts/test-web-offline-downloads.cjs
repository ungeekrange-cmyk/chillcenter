// An isolated same-origin fixture. No access to the Pi or household profiles.
"use strict";
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs/promises");
const { mkdtempSync, readFileSync, rmSync } = require("node:fs");
const os = require("node:os");
const http = require("node:http");
const path = require("node:path");
const { chromium } = require(process.env.CHILL_PREVIEW_NODE_MODULES
  ? path.join(process.env.CHILL_PREVIEW_NODE_MODULES, "playwright") : "playwright");

const repo = path.join(__dirname, "..");
const dist = path.join(repo, "web", "dist");
const id = n => "00000000-0000-4000-8000-" + String(n).padStart(12, "0");
const serverId = id(1), profile = id(2), mediaId = id(3), episode = id(4);
const base = "/api/2026-10-14";
const downloads = base + "/profiles/" + profile + "/downloads";
const subtitleId = id(8);
const caption = Buffer.from("1\n00:00:00,000 --> 00:00:05,000\nOffline caption\n");
const version = '"fixture-1"';
const poster = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGNgAAIAAAUAAXpeqz8AAAAASUVORK5CYII=", "base64");
const chrome = process.env.CHILL_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const fixture = mkdtempSync(path.join(os.tmpdir(), "chill-offline-browser-"));
let clip;
try {
  const file = path.join(fixture, "fixture.mkv");
  const result = spawnSync("ffmpeg", ["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=160x100:rate=24:duration=8",
    "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=8",
    "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "24", "-c:a", "eac3", "-shortest", file]);
  assert.equal(result.status, 0, result.stderr?.toString());
  clip = readFileSync(file);
} finally { rmSync(fixture, { recursive: true }); }
const items = [
  { media: { id: mediaId, title: "Local movie", category: "film", available: true, poster: "fixture" }, bytes: clip.length, version },
  { media: { id: episode, title: "First episode", series: "Local series", category: "serie", season: 1, episode: 1, available: true }, series_key: "series:fixture", bytes: clip.length, version },
];
let disconnected = false;
const attempts = { bytes: 0, leases: 0, progress: 0 };
function json(res, value, status = 200) {
  res.writeHead(status, { "Content-Type": "application/json" });
  res.end(JSON.stringify(value));
}
const mime = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".png": "image/png", ".json": "application/json", ".webmanifest": "application/manifest+json" };
const server = http.createServer(async (req, res) => {
  if (disconnected) { req.socket.destroy(); return; }
  const url = new URL(req.url, "http://fixture.invalid");
  const p = url.pathname;
  if (!p.startsWith(base)) {
    const file = path.resolve(dist, p === "/" ? "index.html" : p.slice(1));
    if (!file.startsWith(dist + path.sep)) return json(res, {}, 404);
    try { const data = await fs.readFile(file); res.setHeader("Content-Type", mime[path.extname(file)] || "application/octet-stream"); return res.end(data); }
    catch { return json(res, {}, 404); }
  }
  if (p === base + "/server") return json(res, { server_id: serverId, capabilities: ["catalog_pages", "playback_leases", "media_range", "offline_downloads"] });
  if (p === base + "/profiles") return json(res, [{ id: profile, name: "Offline fixture", revision: 1 }]);
  if (p === base + "/admin/library") return json(res, { video_root: null });
  if (p.endsWith("/media/" + mediaId + "/artwork")) {
    res.writeHead(200, { "Content-Type": "image/png", "Content-Length": poster.length });
    return res.end(poster);
  }
  if (p.endsWith("/preferences")) return json(res, { audio_language: "en", subtitle_language: "en" });
  if (p.endsWith("/playback/acquire")) { attempts.leases++; return json(res, {}, 500); }
  if (p === downloads + "/plan") return json(res, { items: items.filter(item => item.media.id === url.searchParams.get("media_id")), skipped: 0, next_offset: null });
  if (p.startsWith(downloads + "/")) {
    const [idValue, rest, subtitle] = p.slice(downloads.length + 1).split("/");
    const item = items.find(row => row.media.id === idValue);
    if (!item) return json(res, {}, 404);
    if (rest === "session" && req.method === "POST") {
      return json(res, { version, bytes: item.bytes, token: id(9), preferences: { audio_language: "en", subtitle_language: "en" },
        personal: { watched: false, position_ms: 0 }, progress_revision: "0",
        subtitles: [{ id: subtitleId, title: "English.srt", language: "en", bytes: caption.length }] });
    }
    if (rest === "progress" && req.method === "PUT") { attempts.progress++; return json(res, { revision: "1" }); }
    if (req.headers["x-chill-download"] !== id(9)) return json(res, {}, 409);
    if (rest === "session" && req.method === "DELETE") { res.writeHead(204).end(); return; }
    if (rest === "subtitles" && subtitle === subtitleId) { res.writeHead(200, { "Content-Length": caption.length }); return res.end(caption); }
    if (rest === "content") {
      const range = /^bytes=(\d+)-(\d+)$/.exec(req.headers.range || "");
      if (!range || req.headers["if-match"] !== version) return json(res, {}, 416);
      const start = +range[1], end = +range[2], data = clip.subarray(start, end + 1);
      attempts.bytes += data.length;
      res.writeHead(206, { "Content-Range": "bytes " + start + "-" + end + "/" + item.bytes, "Content-Length": data.length });
      return res.end(data);
    }
  }
  if (p.endsWith("/catalog")) return json(res, { items: [], total: 0, next_offset: null });
  if (p.endsWith("/playback-progress")) return json(res, { items: [] });
  if (p.endsWith("/home")) return json(res, {});
  return json(res, { code: "ROUTE_NOT_FOUND" }, 404);
});

(async () => {
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const origin = "http://127.0.0.1:" + server.address().port;
  const browser = await chromium.launch({ executablePath: chrome, headless: true, args: ["--autoplay-policy=no-user-gesture-required", "--disable-background-networking"] });
  try {
    const context = await browser.newContext({ locale: "fr-FR" });
    const page = await context.newPage();
    page.on("pageerror", error => console.error(error.message));
    await page.addInitScript(({ serverId, profile }) => {
      if (!localStorage.getItem("chill.settings")) localStorage.setItem("chill.settings", JSON.stringify({ last_profile: { [serverId]: profile }, player_presentation: { fullscreen_on_start: false } }));
    }, { serverId, profile });
    await page.goto(origin);
    await page.waitForFunction(() => Boolean(navigator.serviceWorker.controller), null, { timeout: 15000 });
    await page.waitForFunction(() => document.querySelector("#profile-name")?.textContent === "Offline fixture");
    const run = (action, fields) => page.evaluate(({ action, fields }) => window.__TAURI__.core.invoke("downloads_command", { command: { action, ...fields } }), { action, fields });
    for (const item of items) {
      await run("start", { media_id: item.media.id, scope: "video" });
      const until = Date.now() + 15000;
      for (;;) {
        const current = await run("list");
        if (!current.busy && current.items.some(row => row.media.id === item.media.id)) break;
        assert(Date.now() < until, JSON.stringify(current));
        await page.waitForTimeout(100);
      }
    }
    assert.equal(attempts.bytes, clip.length * 2);
    const before = await run("list");
    assert.equal(before.items.length, 2);
    assert.equal(before.enabled, true);
    assert.equal(before.total, 2 * (clip.length + caption.length) + poster.length);
    await run("keep", { media_id: episode, enabled: true });

    disconnected = true;
    await page.close();
    const offline = await context.newPage();
    await offline.goto(origin, { waitUntil: "domcontentloaded" });
    await offline.locator("#downloads-panel").waitFor();
    await offline.locator("#downloads-panel").getByText("Local movie").waitFor();
    await offline.locator("#downloads-panel .download-poster").waitFor();
    assert.equal(await offline.locator("#downloads-panel > .downloads-list > details").count(), 1);
    await offline.locator("#downloads-panel article").filter({ hasText: "Local movie" }).getByRole("button", { name: "Lire", exact: true }).click();
    await offline.waitForFunction(() => {
      const video = document.querySelector("#native-video video");
      return video?.currentTime > 0.5 && Array.from(video.textTracks).some(track => track.activeCues?.length);
    }, null, { timeout: 12000 });
    assert.equal(attempts.leases, 0);
    const deadline = Date.now() + 14000;
    let after;
    do {
      after = await offline.evaluate(() => window.__TAURI__.core.invoke("downloads_command", { command: { action: "list" } }));
      if (!after.items.some(item => item.media.id === mediaId)) break;
      await offline.waitForTimeout(150);
    } while (Date.now() < deadline);
    assert.equal(after.items.length, 1, "default deletes only the completed movie");
    disconnected = false;
    await offline.evaluate(() => dispatchEvent(new Event("online")));
    const syncDeadline = Date.now() + 8000;
    while (!attempts.progress) {
      assert(Date.now() < syncDeadline, "offline progress did not synchronize");
      await offline.waitForTimeout(150);
    }
    await offline.locator("#show-downloads").click();
    await offline.locator("#downloads-panel input[type=checkbox]").uncheck();
    await offline.waitForFunction(() => !document.querySelector("#download-settings input").checked);
    await offline.evaluate(async ({ serverId, profile, episode }) => {
      const root = await navigator.storage.getDirectory();
      const directory = await root.getDirectoryHandle("chillcenter-downloads");
      const prefix = [serverId, profile, episode].join("-") + "-";
      for await (const key of directory.keys()) if (key.startsWith(prefix)) await directory.removeEntry(key, { recursive: true });
      await window.refreshDownloads();
    }, { serverId, profile, episode });
    await offline.locator("#show-downloads").waitFor({ state: "hidden" });
    await offline.locator("#admin-toggle").click();
    await offline.locator("#download-settings").waitFor();
    assert.equal(await offline.locator("#download-settings input").isChecked(), false);
    console.log("PASS Chromium download, cold reopen offline, E-AC-3, sidecar and poster, auto-delete, progress sync, external removal, synchronized switches");
  } finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
})().catch(error => { console.error(error); process.exitCode = 1; });
