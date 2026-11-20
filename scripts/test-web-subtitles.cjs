// Real Chromium regression for browser-native captions. All media and subtitle
// files are temporary synthetic fixtures; no Pi, NAS or household profile is used.
const assert = require("node:assert/strict");
const fs = require("node:fs"), fsp = require("node:fs/promises"), http = require("node:http"), os = require("node:os"), path = require("node:path");
const { spawnSync } = require("node:child_process");
const { chromium } = require(process.env.CHILL_PREVIEW_NODE_MODULES ? path.join(process.env.CHILL_PREVIEW_NODE_MODULES, "playwright") : "playwright");
const root = path.resolve(__dirname, "..");
const chrome = process.env.CHILL_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const profile = "fixture";
const lease = { session_id: "subtitle-session", generation: 7 };
const runtime = ["mp4.js", "audio-core.js", "audio-register.js", "subtitles.js", "mkv.js", "range-reader.js", "mse-engine.js"];

const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(check, label, milliseconds = 8000) { const end = Date.now() + milliseconds; let state; while (Date.now() < end) { state = await check(); if (state?.ok) return state; await wait(80); } throw new Error(`${label}: ${JSON.stringify(state)}`); }
function run(args, label) { const result = spawnSync("ffmpeg", ["-v", "error", "-y", ...args], { encoding: "utf8", timeout: 30000 }); assert.equal(result.status, 0, `${label}: ${result.stderr || result.error || "ffmpeg failed"}`); }
function fixture(dir) {
  const fr = path.join(dir, "fr.srt"), en = path.join(dir, "en.srt"), ass = path.join(dir, "other.ass"), mkv = path.join(dir, "subtitles.mkv");
  fs.writeFileSync(fr, "1\n00:00:01,000 --> 00:00:02,000\nFR DEBUT\n\n2\n00:00:03,000 --> 00:00:09,000\nFR LONGUE 3 A 9\n\n3\n00:00:12,000 --> 00:00:14,000\nFR DOUZE\n\n4\n00:00:17,000 --> 00:00:19,000\nFR DIXSEPT\n");
  fs.writeFileSync(en, "1\n00:00:03,000 --> 00:00:09,000\nEN LONG 3 TO 9\n\n2\n00:00:12,000 --> 00:00:14,000\nEN TWELVE\n");
  fs.writeFileSync(ass, "[Script Info]\nTitle: Fixture\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H64000000,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:03.00,0:00:09.00,Default,,0,0,0,,ASS OTHER\n");
  run(["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=24:duration=20", "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=20", "-i", fr, "-i", en, "-i", ass, "-map", "0:v", "-map", "1:a", "-map", "2", "-map", "3", "-map", "4", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "120", "-c:a", "aac", "-c:s", "copy", "-metadata:s:s:0", "language=fra", "-metadata:s:s:0", "title=French", "-metadata:s:s:1", "language=eng", "-metadata:s:s:1", "title=English", "-metadata:s:s:2", "language=und", "-metadata:s:s:2", "title=ASS", mkv], "MKV H.264/AAC + SRT/SRT/ASS");
  return fs.readFileSync(mkv);
}
function type(file) { return path.extname(file) === ".js" ? "text/javascript" : "application/octet-stream"; }

(async () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "chill-web-subtitles-")), build = path.join(temp, "dist");
  let server, browser;
  try {
    const clip = fixture(temp);
    const built = spawnSync(process.execPath, [path.join(root, "scripts/build-web.mjs"), build], { cwd: root, encoding: "utf8" });
    assert.equal(built.status, 0, built.stderr || built.stdout);
    const requests = [], external = {
      "sub-a1": "1\n00:00:03,000 --> 00:00:09,000\nEXTERNE &lt;img src=x onerror=window.__captionXss=1&gt;\n\n2\n00:00:12,000 --> 00:00:14,000\nEXTERNE DOUZE\n",
      "sub-b2": "WEBVTT\n\n00:00:03.000 --> 00:00:09.000\nVTT EXTERNE\n\n00:00:12.000 --> 00:00:14.000\nVTT DOUZE\n",
      "sub-c3": "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:03.00,0:00:09.00,Default,,0,0,0,,ASS EXTERNE\n",
      "sub-d4": "1\n00:00:03,000 --> 00:00:09,000\nREPONSE TARDIVE\n",
      "sub-e5": "not a subtitle file",
    };
    server = http.createServer(async (req, res) => {
      const url = new URL(req.url, "http://fixture.invalid"), base = `/api/2026-10-14/profiles/${profile}/media/movie/subtitles`;
      if (url.pathname === "/api/2026-10-14/media/movie/stream") {
        const match = /^bytes=(\d+)-(\d+)$/.exec(req.headers.range || ""); assert(match, "engine must range-read fixture");
        const start = Number(match[1]), end = Math.min(Number(match[2]), clip.length - 1);
        res.writeHead(206, { "content-range": `bytes ${start}-${end}/${clip.length}`, etag: '"subtitle-fixture"', "content-type": "application/octet-stream" }); return res.end(clip.subarray(start, end + 1));
      }
      if (url.pathname === base || url.pathname.startsWith(base + "/")) {
        requests.push({ path: url.pathname, query: url.searchParams.toString(), cache: req.headers["cache-control"], site: req.headers["sec-fetch-site"] });
        assert.equal(url.searchParams.get("session"), lease.session_id); assert.equal(url.searchParams.get("generation"), String(lease.generation));
        assert.match(String(req.headers["cache-control"] || ""), /no-cache/);
        assert.equal(req.headers["sec-fetch-site"], "same-origin");
        if (url.pathname === base) { res.setHeader("content-type", "application/json"); return res.end(JSON.stringify({ items: [{ id: "sub-a1", title: "external.srt", language: "fr" }, { id: "sub-b2", title: "external.vtt", language: "en" }, { id: "sub-c3", title: "external.ass", language: "fr" }, { id: "sub-d4", title: "late.srt", language: "en" }, { id: "sub-e5", title: "bad.ass", language: "fr" }] })); }
        const id = url.pathname.slice(base.length + 1); if (id === "sub-d4") await wait(500);
        res.setHeader("content-type", "text/plain"); res.setHeader("content-length", Buffer.byteLength(external[id] || "")); return res.end(external[id] || "");
      }
      if (url.pathname === "/") { res.setHeader("content-type", "text/html"); return res.end("<!doctype html><html lang=fr><main id=host></main></html>"); }
      const file = path.resolve(build, decodeURIComponent(url.pathname.slice(1))); if (!file.startsWith(build + path.sep)) { res.statusCode = 404; return res.end(); }
      try { res.setHeader("content-type", type(file)); res.end(await fsp.readFile(file)); } catch { res.statusCode = 404; res.end(); }
    });
    await new Promise(resolve => server.listen(0, "127.0.0.1", resolve)); const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ executablePath: chrome, headless: true, args: ["--disable-background-networking", "--no-first-run", "--autoplay-policy=no-user-gesture-required"] });
    const page = await browser.newPage(); page.setDefaultTimeout(20000); const errors = []; page.on("pageerror", error => errors.push(error.message));
    await page.goto(origin + "/"); await page.evaluate(() => { localStorage.setItem("chill.settings", JSON.stringify({ playback_cache: { back_mib: 0, forward_mib: 16 } })); window.ChillEngines = { factories: [] }; });
    for (const script of runtime) await page.addScriptTag({ url: `${origin}/${script}` });
    await page.evaluate(async ({ lease, profile }) => { const factory = ChillEngines.factories[0]; if (!factory) throw Error("MSE factory absent"); window.__engine = factory(document.querySelector("#host")); await window.__engine.open({ mediaId: "movie", lease, startAt: 5.2, speed: 1, subtitleLanguage: "fr", profileId: profile }); await window.__engine.play(); window.__captionState = () => { const e = window.__engine, v = e.video, t = e.captions?.textTrack, cues = Array.from(t?.activeCues || []).map(c => c.getCueAsHTML().textContent); return { time: v?.currentTime ?? null, paused: Boolean(v?.paused), seeking: Boolean(v?.seeking), error: e.errorText || null, activeSubtitle: e.activeSubtitle?.id ?? null, mode: t?.mode || null, cues, records: { count: e.captions?.records.length ?? 0, first: e.captions?.records[0]?.text ?? null, last: e.captions?.records.at(-1)?.text ?? null }, epoch: e.epoch, captionEpoch: e.captionEpoch, repositionTarget: e.repositionTarget ?? null, repositioning: Boolean(e.repositioning) }; }; }, { lease, profile });
    const french = await until(() => page.evaluate(() => { const state = window.__captionState(); return { ...state, ok: Boolean(!state.error && state.time > 5.4 && state.mode === "showing" && state.cues.some(text => text.includes("FR LONGUE"))), tracks: window.__engine.subtitleTracks?.map(t => ({ id: t.id, language: t.language })) }; }), "French embedded caption at resumed frame");
    assert(french.tracks.some(track => track.id === 3 && track.language === "fr"), "first text track must retain Matroska id 3");
    await page.evaluate(() => window.__engine.selectSubtitle(4));
    await until(() => page.evaluate(() => { const state = window.__captionState(); return { ...state, ok: Boolean(state.time > 5.2 && state.cues.some(text => text.includes("EN LONG"))) }; }), "English selection preserves time and updates caption");
    await page.evaluate(() => window.__engine.selectSubtitle(5));
    await until(() => page.evaluate(() => { const cue = window.__engine.captions.textTrack?.activeCues?.[0]; return { ok: cue?.getCueAsHTML().textContent.includes("ASS OTHER") }; }), "embedded ASS caption");
    await page.evaluate(() => window.__engine.selectSubtitle(null));
    assert.deepEqual(await page.evaluate(() => ({ tracks: Array.from(window.__engine.video.textTracks).filter(t => t.mode === "showing").length, cues: window.__engine.captions.records.length })), { tracks: 0, cues: 0 }, "Off removes owned native track and cues");
    await page.evaluate(() => window.__engine.selectSubtitle(3));
    await until(() => page.evaluate(() => { const cue = window.__engine.captions.textTrack?.activeCues?.[0]; return { ok: cue?.getCueAsHTML().textContent.includes("FR LONGUE") }; }), "French caption immediately returns");
    for (const [at, text] of [[1, "FR DEBUT"], [12, "FR DOUZE"], [17, "FR DIXSEPT"]]) {
      const beforeState = await page.evaluate(() => { const v = window.__engine.video; return { time: v.currentTime, paused: v.paused, ended: v.ended, duration: v.duration, frames: v.getVideoPlaybackQuality().totalVideoFrames }; });
      const before = beforeState.frames;
      await page.evaluate(at => window.__engine.seek(at), at);
      await until(() => page.evaluate(({ at, before, text, beforeState }) => { const e = window.__engine, v = e.video, cue = window.__engine.captions.textTrack?.activeCues?.[0]; return { ok: !e.errorText && !v.seeking && v.currentTime > at + .15 && v.currentTime < at + 1.5 && v.getVideoPlaybackQuality().totalVideoFrames > before && cue?.getCueAsHTML().textContent.includes(text), beforeState, time: v.currentTime, cue: cue?.getCueAsHTML().textContent, error: e.errorText, paused: v.paused, ended: v.ended, seeking: v.seeking, duration: v.duration, movie: e.movie.duration, repositionTarget: e.repositionTarget ?? null, repositioning: Boolean(e.repositioning), buffered: e.streams.map(s => Array.from({ length: s.sb.buffered.length }, (_, i) => [s.sb.buffered.start(i), s.sb.buffered.end(i)])) }; }, { at, before, text, beforeState }), `caption after seek ${at}`);
    }
    await until(() => page.evaluate(() => ({ ok: window.__engine.externalSubtitles().length === 5 })), "external subtitle list");
    const externalSeek = async (at, text, label) => {
      const before = await page.evaluate(() => window.__engine.video.getVideoPlaybackQuality().totalVideoFrames);
      await page.evaluate(at => window.__engine.seek(at), at);
      await until(() => page.evaluate(({ at, before, text }) => { const e = window.__engine, v = e.video, cue = window.__engine.captions.textTrack?.activeCues?.[0]; return { ok: Boolean(!v.seeking && v.currentTime > at + .15 && v.currentTime < at + 1.5 && v.getVideoPlaybackQuality().totalVideoFrames > before && cue?.getCueAsHTML().textContent.includes(text)), time: v.currentTime, cue: cue?.getCueAsHTML().textContent, repositionTarget: e.repositionTarget ?? null, repositioning: Boolean(e.repositioning) }; }, { at, before, text }), label);
    };
    await externalSeek(4, "FR LONGUE", "seek before external subtitle");
    await page.evaluate(() => window.__engine.selectExternalSubtitle("sub-a1"));
    await until(() => page.evaluate(() => { const e = window.__engine, state = window.__captionState(); return { ...state, external: e.activeExternal?.id ?? null, tracks: e.snapshot().tracks, hasImage: Boolean(e.video.querySelector("img")), xss: Boolean(window.__captionXss), ok: Boolean(e.snapshot().tracks.some(t => t.id === "sub-a1" && t.selected) && state.cues.some(text => text.includes("EXTERNE <img")) && !e.video.querySelector("img") && !window.__captionXss) }; }), "safe external SRT");
    await externalSeek(5, "EXTERNE <img", "external SRT forward seek"); await externalSeek(7, "EXTERNE <img", "external SRT forward seek again"); await externalSeek(4, "EXTERNE <img", "external SRT backward seek");
    await page.evaluate(() => window.__engine.selectExternalSubtitle("sub-b2"));
    await until(() => page.evaluate(() => { const cue = window.__engine.captions.textTrack?.activeCues?.[0]; return { ok: cue?.getCueAsHTML().textContent.includes("VTT EXTERNE") }; }), "valid external VTT");
    await page.evaluate(() => window.__engine.selectExternalSubtitle("sub-c3"));
    await until(() => page.evaluate(() => { const cue = window.__engine.captions.textTrack?.activeCues?.[0]; return { ok: cue?.getCueAsHTML().textContent.includes("ASS EXTERNE") }; }), "valid external ASS");
    await page.evaluate(() => { window.__lateTask = window.__engine.selectExternalSubtitle("sub-d4"); return true; }); await wait(80); await page.evaluate(() => window.__engine.selectSubtitle(null)); await page.evaluate(() => window.__lateTask);
    assert.equal(await page.evaluate(() => window.__engine.activeExternal), null, "late external response revived caption after Off");
    const frames = await page.evaluate(() => window.__engine.video.getVideoPlaybackQuality().totalVideoFrames); await page.evaluate(() => window.__engine.selectExternalSubtitle("sub-e5"));
    await until(() => page.evaluate(frames => ({ ok: window.__engine.video.getVideoPlaybackQuality().totalVideoFrames > frames && Boolean(window.__engine.captionNotice) }), frames), "malformed external subtitle leaves video playing");
    assert(requests.length >= 4); assert.deepEqual(errors, []); assert.equal(await page.evaluate(() => document.querySelectorAll(".chill-caption-overlay").length), 0, "native video track, not DOM overlay");
    console.log("PASS web subtitles: embedded SRT/ASS native tracks, seek/reposition captions, external same-origin lease-bound files, stale/error handling and literal cue safety.");
  } finally { await browser?.close(); if (server) await new Promise(resolve => server.close(resolve)); fs.rmSync(temp, { recursive: true, force: true }); }
})().catch(error => { console.error(error); process.exitCode = 1; });
