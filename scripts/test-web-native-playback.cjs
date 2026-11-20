// Browser regression for the native MediaSource path.  It serves only local,
// generated fixture bytes; it never contacts the Pi, NAS, or a user profile.
const { chromium } = require(process.env.CHILL_PREVIEW_NODE_MODULES ? `${process.env.CHILL_PREVIEW_NODE_MODULES}/playwright` : "playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const fsp = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const origin = "https://native-playback.fixture";
const chrome = process.env.CHILL_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const scripts = ["mp4.js", "audio-core.js", "audio-register.js", "mkv.js", "range-reader.js", "mse-engine.js"];
const trace = (milestone, detail = {}) => {
  if (process.env.CHILL_NATIVE_PLAYBACK_TRACE === "1") console.error(JSON.stringify({ test: "native-playback", milestone, ...detail }));
};
const within = (promise, label, milliseconds = 10000) => new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error(`${label} timed out after ${milliseconds} ms`)), milliseconds);
  Promise.resolve(promise).then(value => { clearTimeout(timer); resolve(value); }, error => { clearTimeout(timer); reject(error); });
});

function run(args, label) {
  const result = spawnSync("ffmpeg", ["-v", "error", "-y", ...args], { encoding: "utf8", timeout: 30000 });
  assert.equal(result.status, 0, `${label}: ${result.stderr || result.error || "ffmpeg failed"}`);
}

function fixtures(dir) {
  const video = ["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=24:duration=18"];
  const audio = ["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=18"];
  const mp4 = path.join(dir, "h264-aac-11025.mp4");
  run([...video, "-f", "lavfi", "-i", "sine=frequency=660:sample_rate=11025:duration=18", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "48", "-c:a", "aac", "-ar", "11025", "-shortest", "-movflags", "+faststart", mp4], "H.264/AAC 11.025 kHz fixture");
  const hevc = path.join(dir, "hevc-bframes.mkv");
  run([...video, ...audio, "-c:v", "libx265", "-pix_fmt", "yuv420p", "-x265-params", "open-gop=1:keyint=120:min-keyint=120:bframes=8", "-c:a", "aac", "-shortest", hevc], "HEVC/AAC long-GOP fixture");
  return { mp4: fs.readFileSync(mp4), hevc: fs.readFileSync(hevc) };
}

function contentType(file) {
  return path.extname(file) === ".js" ? "text/javascript" : "application/octet-stream";
}

async function route(page, build, clips) {
  await page.route("**/*", async handler => {
    const request = handler.request(), url = new URL(request.url());
    if (url.origin !== origin) return handler.abort();
    if (url.pathname.startsWith("/api/2026-10-14/media/") && url.pathname.endsWith("/stream")) {
      const id = url.pathname.split("/")[4], bytes = clips[id];
      if (!bytes) return handler.fulfill({ status: 404, body: "missing fixture" });
      const match = /^bytes=(\d+)-(\d+)$/.exec(request.headers().range || "");
      assert(match, "the engine must use ranged reads");
      const start = Number(match[1]), end = Math.min(Number(match[2]), bytes.length - 1);
      return handler.fulfill({ status: 206, headers: { "content-range": `bytes ${start}-${end}/${bytes.length}`, etag: '"native-fixture"', "content-type": "application/octet-stream" }, body: bytes.subarray(start, end + 1) });
    }
    if (url.pathname === "/") return handler.fulfill({ contentType: "text/html", body: "<!doctype html><meta charset=utf-8><main id=host></main>" });
    const file = path.resolve(build, decodeURIComponent(url.pathname.slice(1)));
    if (!file.startsWith(`${build}${path.sep}`)) return handler.abort();
    try { return handler.fulfill({ contentType: contentType(file), body: await fsp.readFile(file) }); }
    catch { return handler.fulfill({ status: 404, body: "missing asset" }); }
  });
}

const waitFor = async (page, predicate, label, milliseconds = 5000, argument) => {
  const until = Date.now() + milliseconds;
  let state;
  do { state = await page.evaluate(predicate, argument); if (state.ok) return state; await page.waitForTimeout(80); } while (Date.now() < until);
  throw new Error(`${label}: ${JSON.stringify(state)}`);
};

async function load(page) {
  await page.goto(`${origin}/`);
  await page.evaluate(() => {
    // Back-cache zero is a supported preference. It makes every out-of-buffer
    // seek exercise eviction, including the dependency-safe random access
    // point chosen by the MSE engine.
    localStorage.setItem("chill.settings", JSON.stringify({ playback_cache: { back_mib: 0, forward_mib: 16 } }));
    window.ChillEngines = { factories: [] };
  });
  for (const script of scripts) await page.addScriptTag({ url: `${origin}/${script}` });
  await page.evaluate(() => {
    const Factory = window.ChillEngines.factories[0];
    if (!Factory) throw new Error("MSE engine did not register");
    window.__nativeEngine = Factory(document.querySelector("#host"));
  });
}

async function openAndPlay(page, id, startAt) {
  await page.evaluate(async ({ id, startAt }) => {
    await window.__nativeEngine.open({ mediaId: id, lease: { session_id: "fixture", generation: 1 }, startAt, speed: 1 });
    await window.__nativeEngine.play();
  }, { id, startAt });
  return waitFor(page, startAt => {
    const e = window.__nativeEngine, v = e.video;
    return { ok: Boolean(v && !e.errorText && !v.paused && !v.seeking && v.currentTime > startAt + 0.3), current: v?.currentTime, error: e.errorText, paused: v?.paused, seeking: v?.seeking, buffered: e?.streams?.map(s => Array.from({ length: s.sb.buffered.length }, (_, i) => [s.sb.buffered.start(i), s.sb.buffered.end(i)])) };
  }, `${id} playback did not advance`, 5000, startAt);
}

async function assertNative(page, label) {
  const state = await page.evaluate(() => {
    const e = window.__nativeEngine, v = e.video, q = v.getVideoPlaybackQuality?.();
    return { error: e.errorText, canvas: document.querySelectorAll("canvas").length, video: document.querySelectorAll("video").length, time: v.currentTime, decoded: q?.totalVideoFrames ?? 0, buffered: e.streams.map(s => Array.from({ length: s.sb.buffered.length }, (_, i) => [s.sb.buffered.start(i), s.sb.buffered.end(i)])) };
  });
  assert.equal(state.error, null, `${label}: ${JSON.stringify(state)}`);
  assert.deepEqual({ canvas: state.canvas, video: state.video }, { canvas: 0, video: 1 }, `${label}: expected one native video`);
  assert.ok(state.decoded > 0, `${label}: browser decoded no video frame: ${JSON.stringify(state)}`);
}

async function seekAndAssert(page, seconds, label) {
  const before = await page.evaluate(() => window.__nativeEngine.video.getVideoPlaybackQuality?.().totalVideoFrames ?? 0);
  await page.evaluate(seconds => window.__nativeEngine.seek(seconds), seconds);
  await waitFor(page, ({ seconds, before }) => {
    const e = window.__nativeEngine, v = e.video;
    const decoded = v.getVideoPlaybackQuality?.().totalVideoFrames ?? 0;
    return {
      ok: !e.errorText && !v.paused && !v.seeking && v.currentTime > seconds + 0.25 && decoded > before,
      time: v.currentTime,
      decoded,
      decodedBefore: before,
      error: e.errorText,
      paused: v.paused,
      seeking: v.seeking,
      buffered: e.streams.map(s => Array.from({ length: s.sb.buffered.length }, (_, i) => [s.sb.buffered.start(i), s.sb.buffered.end(i)])),
    };
  }, `${label} seek did not advance`, 5000, { seconds, before });
  await assertNative(page, `${label} seek ${seconds}`);
}

(async () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "chill-web-native-playback-"));
  const build = path.join(temp, "dist");
  try {
    trace("temporary-workspace-created");
    const clips = fixtures(temp);
    trace("fixtures-generated", { bytes: { mp4: clips.mp4.length, hevc: clips.hevc.length } });
    const built = spawnSync(process.execPath, [path.join(root, "scripts/build-web.mjs"), build], { cwd: root, encoding: "utf8" });
    assert.equal(built.status, 0, built.stderr || built.stdout);
    trace("web-built");
    const browser = await chromium.launch({ executablePath: chrome, headless: true, args: ["--disable-background-networking", "--no-first-run", "--autoplay-policy=no-user-gesture-required"] });
    try {
      trace("browser-launched", { executable: chrome });
      browser.on("disconnected", () => trace("browser-disconnected"));
      const page = await within(browser.newPage(), "browser.newPage");
      trace("page-created");
      page.setDefaultTimeout(20000);
      const errors = []; page.on("pageerror", e => errors.push(e.message));
      await route(page, build, clips); await load(page);
      trace("harness-loaded");

      await openAndPlay(page, "mp4", 4); await assertNative(page, "H.264/AAC 11.025 kHz resume");
      for (const seconds of [7.5, 12.5, 4]) {
        await seekAndAssert(page, seconds, "H.264/AAC 11.025 kHz");
      }
      trace("h264-verified");
      await page.evaluate(() => window.__nativeEngine.pause());
      const paused = await page.evaluate(() => ({ paused: window.__nativeEngine.video.paused, time: window.__nativeEngine.video.currentTime }));
      assert.equal(paused.paused, true); assert.ok(paused.time > 0.3);
      await page.evaluate(() => window.__nativeEngine.close());

      await load(page);
      const hevcMime = await page.evaluate(() => MediaSource.isTypeSupported('video/mp4; codecs="hvc1.1.6.L93.B0"'));
      console.log(JSON.stringify({ test: "HEVC MediaSource", supported: hevcMime }));
      assert.equal(hevcMime, true, "This Chrome must expose HEVC MediaSource for the HEVC regression");
      await openAndPlay(page, "hevc", 4); await assertNative(page, "HEVC/AAC resume");
      for (const seconds of [7.5, 12.5, 4]) {
        await seekAndAssert(page, seconds, "HEVC/AAC");
      }
      trace("hevc-verified");
      assert.deepEqual(errors, []);
      console.log("PASS native browser playback: H.264/AAC 11.025 kHz and HEVC/AAC long-GOP resume plus dependency-safe seeks.");
    } finally { await browser.close(); }
  } finally { fs.rmSync(temp, { recursive: true, force: true }); }
})().catch(error => { console.error(error); process.exitCode = 1; });
