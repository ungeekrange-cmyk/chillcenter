// Browser-level playback proof for the built web client. It creates only
// disposable ffmpeg fixtures: no Pi, NAS, server or user video is contacted.
// Chromium's MediaSource is deliberately exposed as ManagedMediaSource to
// exercise our fallback registration; this is not a claim of native iOS MMS.
const { chromium } = require(process.env.CHILL_PREVIEW_NODE_MODULES ? `${process.env.CHILL_PREVIEW_NODE_MODULES}/playwright` : "playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const fsp = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const origin = "https://playback.fixture";
const scripts = ["json64.js", "watch-clock.js", "engines.js", "mp4.js", "audio-core.js", "audio-register.js", "mkv.js", "avi.js", "soft-video.js", "mpeg4-register.js", "range-reader.js", "mse-engine.js", "player-web.js", "mobile-playback.js"];
const profile = "11111111-1111-4111-8111-111111111111";

function makeFixtures(dir) {
  const common = ["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=160x96:rate=24:duration=4", "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100:duration=4"];
  const run = args => {
    const result = spawnSync("ffmpeg", [...common, ...args], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
  };
  const mkv = path.join(dir, "fixture-h264-aac.mkv");
  const avi = path.join(dir, "fixture-xvid-mp3.avi");
  run(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "24", "-c:a", "aac", "-shortest", mkv]);
  run(["-c:v", "mpeg4", "-vtag", "XVID", "-bf", "0", "-g", "24", "-c:a", "libmp3lame", "-b:a", "128k", "-shortest", avi]);
  return { mkv: fs.readFileSync(mkv), avi: fs.readFileSync(avi) };
}

function contentType(file) {
  return ({ ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".html": "text/html" })[path.extname(file)] || "application/octet-stream";
}

async function routeFixture(page, build, clips) {
  await page.route("**/*", async route => {
    const request = route.request(), url = new URL(request.url());
    if (url.origin !== origin) return route.abort();
    if (url.pathname.startsWith("/api/2026-10-14/media/") && url.pathname.endsWith("/stream")) {
      const id = url.pathname.split("/")[4];
      const bytes = clips[id];
      if (!bytes) return route.fulfill({ status: 404, body: "missing fixture" });
      const match = /^bytes=(\d+)-(\d+)$/.exec(request.headers().range || "");
      assert(match, "RangeReader must request a byte range");
      const start = Number(match[1]), end = Math.min(Number(match[2]), bytes.length - 1);
      return route.fulfill({ status: 206, headers: { "content-range": `bytes ${start}-${end}/${bytes.length}`, etag: '"fixture"', "content-type": "application/octet-stream" }, body: bytes.subarray(start, end + 1) });
    }
    if (url.pathname === "/" || url.pathname === "/harness.html") {
      return route.fulfill({ contentType: "text/html", body: "<!doctype html><html lang=\"en\"><head><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Playback fixture</title></head><body><section id=\"player-panel\"><div id=\"player-stage\"><div id=\"native-video\"></div></div></section></body></html>" });
    }
    const file = path.resolve(build, decodeURIComponent(url.pathname.slice(1)));
    if (!file.startsWith(`${build}${path.sep}`)) return route.abort();
    try { return route.fulfill({ contentType: contentType(file), body: await fsp.readFile(file) }); }
    catch { return route.fulfill({ status: 404, body: "missing asset" }); }
  });
}

async function loadRuntime(page) {
  await page.goto(`${origin}/harness.html`);
  await page.addStyleTag({ url: `${origin}/web.css` });
  await page.evaluate(() => {
    // Test only the fallback wiring. The native Chromium constructor still
    // does all decoding and source-buffer work below. Hide native MP3 MSE
    // support too: this exercises the iPhone-compatible AVI MP3 -> AAC
    // worker path instead of Chromium's ordinary MPEG audio fast path.
    const NativeMediaSource = window.MediaSource;
    function FixtureManagedMediaSource() { return new NativeMediaSource(); }
    FixtureManagedMediaSource.prototype = NativeMediaSource.prototype;
    FixtureManagedMediaSource.isTypeSupported = type => /audio\/(mpeg|mp4; codecs="mp4a\.6B")/i.test(type) ? false : NativeMediaSource.isTypeSupported(type);
    Object.defineProperty(window, "ManagedMediaSource", { configurable: true, value: FixtureManagedMediaSource });
    Object.defineProperty(window, "MediaSource", { configurable: true, value: undefined });
    Object.defineProperty(window, "EventSource", { configurable: true, value: undefined });
    window.__allowFixturePlay = false;
    window.__fixtureNativePlay = HTMLMediaElement.prototype.play;
    HTMLMediaElement.prototype.play = function fixturePlay() {
      if (!window.__allowFixturePlay) return Promise.reject(new DOMException("gesture required", "NotAllowedError"));
      return window.__fixtureNativePlay.call(this);
    };
  });
  for (const file of scripts) await page.addScriptTag({ url: `${origin}/${file}` });
  await page.evaluate(() => {
    // The lease media id comes from the start command; the fixture request
    // only needs to satisfy the client-side control protocol.
    window.__fixtureAcquireCount = 0;
    window.__fixtureCtx = { profile: "11111111-1111-4111-8111-111111111111" };
    window.__fixtureCtx.request = async (method, apiPath) => {
      if (method === "GET" && /\/playback-info$/.test(apiPath)) return { media: { title: "Synthetic clip" }, speed: 1, preferences: { audio_language: "en", subtitle_language: "en" } };
      if (method === "POST" && /\/playback\/acquire$/.test(apiPath)) {
        const id = window.__fixtureRequestedId;
        window.__fixtureAcquireCount++;
        return { lease: { session_id: "fixture-session", generation: 1, media_id: id, media_revision: 1 }, personal: { position_ms: 0 } };
      }
      if (method === "POST" && /\/(release|progress|heartbeat)$/.test(apiPath)) return {};
      throw new Error(`Unexpected fixture API request: ${method} ${apiPath}`);
    };
  });
}

async function command(page, command) {
  return page.evaluate(async command => {
    if (command.action === "start") window.__fixtureRequestedId = command.media_id;
    return ChillWebPlayer.command(command, window.__fixtureCtx);
  }, command);
}

async function waitForStatus(page, predicate, label) {
  let view;
  for (let attempt = 0; attempt < 150; attempt++) {
    view = await command(page, { action: "status" });
    if (predicate(view)) return view;
    await page.waitForTimeout(100);
  }
  throw new Error(`${label}: ${JSON.stringify(view)}`);
}

(async () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "chill-web-mobile-playback-"));
  const build = path.join(temp, "dist"), fixtures = makeFixtures(temp);
  const buildResult = spawnSync(process.execPath, [path.join(root, "scripts/build-web.mjs"), build], { cwd: root, encoding: "utf8" });
  assert.equal(buildResult.status, 0, buildResult.stderr || buildResult.stdout);
  const browser = await chromium.launch({ executablePath: process.env.CHILL_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true, args: ["--disable-background-networking", "--no-first-run", "--autoplay-policy=no-user-gesture-required"] });
  try {
    const page = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
    page.setDefaultTimeout(15000);
    const errors = []; page.on("pageerror", error => errors.push(error.message));
    await routeFixture(page, build, { mkv: fixtures.mkv, avi: fixtures.avi });
    await loadRuntime(page);

    await command(page, { action: "start", media_id: "mkv", context: { kind: "standalone", id: null } });
    try { await waitForStatus(page, view => view.stage === "player" && view.native.loaded, "MMS fallback loads the MKV"); }
    catch (error) { console.error("MKV diagnostics", await command(page, { action: "status" })); throw error; }
    const blocked = await command(page, { action: "status" });
    assert.equal(blocked.native.play_blocked, true, "the initial scripted play is visible as a recoverable block");
    await page.waitForSelector("#web-play-retry:not([hidden])");
    const beforeRetry = await page.evaluate(() => {
      window.__fixtureVideo = document.querySelector("#native-video video");
      return { acquires: window.__fixtureAcquireCount, video: Boolean(window.__fixtureVideo) };
    });
    assert.deepEqual(beforeRetry, { acquires: 1, video: true });
    await page.evaluate(() => document.addEventListener("click", () => { window.__allowFixturePlay = true; }, { capture: true, once: true }));
    await page.locator("#web-play-retry").click();
    await waitForStatus(page, view => view.stage === "player" && view.position > 0.3, "trusted retry starts the buffered MKV");
    const resumed = await command(page, { action: "status" });
    assert.equal(resumed.native.play_blocked, false);
    assert.equal(await page.locator("#native-video video").count(), 1);
    assert.equal(await page.evaluate(() => window.__fixtureVideo === document.querySelector("#native-video video")), true, "retry keeps the same buffered media element");
    assert.equal(await page.evaluate(() => window.__fixtureAcquireCount), 1, "retry does not acquire or reopen the stream");
    await command(page, { action: "stop" });

    await command(page, { action: "start", media_id: "avi", context: { kind: "standalone", id: null } });
    await waitForStatus(page, view => view.stage === "player" && view.native.loaded, "AVI opens after the MKV is stopped");
    // Playback permission remains granted from the trusted retry. AVI exercises
    // the Xvid software decoder and the forced MP3 -> AAC fallback (the
    // native MP3 MSE capability was deliberately hidden above).
    await waitForStatus(page, view => view.stage === "player" && view.position > 0.3, "AVI advances");
    assert.equal(await page.locator("#native-video canvas").count(), 1, "AVI uses the software video canvas");
    await page.waitForTimeout(150);
    const canvas = await page.locator("#native-video canvas").evaluate(el => ({ width: el.width, height: el.height }));
    assert(canvas.width >= 160 && canvas.height >= 96, `unexpected AVI canvas ${JSON.stringify(canvas)}`);
    const hasPicture = await page.locator("#native-video canvas").evaluate(el => {
      const pixels = el.getContext("2d").getImageData(0, 0, el.width, el.height).data;
      for (let i = 0; i < pixels.length; i += 4) if (pixels[i] || pixels[i + 1] || pixels[i + 2]) return true;
      return false;
    });
    assert.equal(hasPicture, true, "AVI canvas contains decoded RGB pixels, not only dimensions");
    await command(page, { action: "stop" });
    assert.deepEqual(errors, []);
    console.log("PASS built web playback: ManagedMediaSource fallback wiring, blocked-play retry without reopening, H.264/AAC MKV and Xvid/MP3 AVI with forced local MP3->AAC fallback in Chromium synthetic fixtures. Native iPhone Safari/ManagedMediaSource still requires device validation.");
  } finally {
    await browser.close();
    fs.rmSync(temp, { recursive: true, force: true });
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
