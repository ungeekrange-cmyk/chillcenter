"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { WebPlayer, pickTrack, uuidV4 } = require("../src/player-web.js");
const { UnsupportedEngine } = require("../src/engines.js");

const P = "11111111-1111-4111-8111-111111111111", M = "22222222-2222-4222-8222-222222222222", N = "33333333-3333-4333-8333-333333333333";
const lease = (extra = {}) => ({ session_id: "sess", generation: 3, media_id: M, media_revision: 18446744073709551615n, remaining_ms: 30000, ...extra });
const info = { media: { id: M, title: "Film" }, preferences: { audio_language: "en", subtitle_language: "en", autoplay_seconds: 10 }, speed: 1 };

class FakeEngine {
  constructor(tracks) { this.s = { loaded: true, paused: true, buffering: false, ended: false, position: 0, duration: 1000, speed: 1, volume: 100, tracks }; this.log = []; }
  async open(o) { this.log.push(["open", o.startAt]); this.s.position = o.startAt; }
  play() { this.s.paused = false; } pause() { this.s.paused = true; } seek(p) { this.s.position = p; } setSpeed(x) { this.s.speed = x; } setVolume() {}
  selectAudio(id) { this.log.push(["audio", id]); } selectSubtitle(id) { this.log.push(["sub", id]); } externalSubtitles() { return []; }
  snapshot() { return { ...this.s }; } close() { this.closed = true; }
}
function harness({ acquire, tracks = [], context } = {}) {
  const calls = [], engine = new FakeEngine(tracks), intervals = []; let t = 0;
  const request = async (method, path, o = {}) => {
    calls.push({ method, path, body: o.body });
    if (path.endsWith("/playback-info")) return info;
    if (path.endsWith("/playback/acquire")) { if (acquire) return acquire(); return { lease: lease(), personal: { position_ms: 12000 } }; }
    if (path.endsWith("/playback/heartbeat")) return { remaining_ms: 30000 };
    if (path.endsWith("/playback/next")) return { media: { id: N, title: "Suite" }, new_season: false, available: true };
    return { saved: true };
  };
  const player = new WebPlayer({ request, engineFactory: () => engine, storage: { getItem: () => "dev", setItem() {} }, now: () => t, setInterval: (f, ms) => { intervals.push(f); return intervals.length; }, clearInterval() {}, EventSource: null, uuid: () => "dev" });
  const ctx = { profile: P };
  return { player, calls, engine, ctx, tick: ms => { t += ms; }, intervals, cmd: c => player.command(c, ctx) };
}

test("start acquires a lease with device and context, resumes at the saved position", async () => {
  const h = harness();
  const view = await h.cmd({ action: "start", media_id: M, context: { kind: "standalone", id: null } });
  const acquire = h.calls.find(c => c.path.endsWith("/acquire"));
  assert.deepEqual(acquire.body, { media_id: M, device_id: "dev", context: { kind: "standalone", id: null } });
  assert.deepEqual(h.engine.log[0], ["open", 12]);
  assert.equal(view.stage, "player"); assert.equal(view.native.paused, false); assert.equal(view.title, "Film");
});

test("progress and heartbeat re-emit the exact uint64 revision; final tenth credits real time", async () => {
  const h = harness();
  await h.cmd({ action: "start", media_id: M, context: { kind: "standalone", id: null } });
  h.engine.s.position = 899; h.engine.s.duration = 1000;
  await h.player.report(false, false); h.tick(5000); h.engine.s.position = 904; await h.player.report(false, false);
  const progress = h.calls.filter(c => c.path.endsWith("/progress")).map(c => c.body);
  assert.equal(progress[1].final_elapsed_ms, 4000); assert.equal(progress[1].media_revision, 18446744073709551615n);
  assert.deepEqual(Object.keys(progress[1]).sort(), ["buffering", "duration_ms", "final_elapsed_ms", "generation", "media_id", "media_revision", "playing", "position_ms", "seek", "sequence", "session_id", "speed"]);
  await h.player.heartbeat();
  assert.deepEqual(Object.keys(h.calls.find(c => c.path.endsWith("/heartbeat")).body), ["session_id", "generation", "media_id", "media_revision"]);
});

test("seek and speed break credit; speed is saved per lease", async () => {
  const h = harness();
  await h.cmd({ action: "start", media_id: M, context: { kind: "series", id: "x" } });
  await h.cmd({ action: "jump", seconds: 30 });
  assert.equal(h.engine.s.position, 42);
  assert.equal(h.calls.filter(c => c.path.endsWith("/progress")).at(-1).body.seek, true);
  await h.cmd({ action: "speed", speed: 1.5 });
  assert.equal(h.calls.at(-1).body.speed, 1.5);
  await assert.rejects(h.cmd({ action: "speed", speed: 3 }), /Vitesse invalide/);
});

test("end of video records progress, releases the lease, then resolves the next episode", async () => {
  const h = harness();
  await h.cmd({ action: "start", media_id: M, context: { kind: "series", id: "x" } });
  h.engine.s.ended = true; h.engine.s.position = 1000;
  const view = await h.cmd({ action: "status" });
  const paths = h.calls.map(c => c.path.split("/playback/")[1]).filter(Boolean);
  assert.deepEqual(paths.slice(-3), ["progress", "release", "next"]);
  assert.equal(view.next.media.id, N); assert.equal(view.autoplay_seconds, 10); assert.equal(view.native.ended, true);
  const solo = harness(); await solo.cmd({ action: "start", media_id: M, context: { kind: "standalone", id: null } });
  solo.engine.s.ended = true; assert.equal((await solo.cmd({ action: "status" })).next.media, null);
});

test("a second device gets a transfer offer instead of a lease", async () => {
  const h = harness({ acquire: () => { throw Object.assign(new Error("x"), { status: 409, code: "PLAYBACK_ACTIVE" }); } });
  const orig = h.player.d.request;
  h.player.d.request = async (m, p, o) => p.endsWith("/transfer/propose") ? { token: "tok", expires_in_ms: 20000, active: lease() } : orig(m, p, o);
  const view = await h.cmd({ action: "start", media_id: M, context: { kind: "standalone", id: null } });
  assert.equal(view.stage, "transfer"); assert.equal(view.confirmed, false); assert.equal(view.remaining_ms, 20000);
  assert.equal(h.calls.some(c => c.path.endsWith("/heartbeat")), false);
  assert.equal((await h.cmd({ action: "refuse_transfer" })).stage, "closed");
});

test("language policy: preferred, else original audio with a notice; subtitles off when absent", () => {
  const tracks = [{ id: 1, kind: "Audio", language: "fr", original: true }, { id: 2, kind: "Audio", language: "en" }, { id: 3, kind: "Subtitle", language: "fr" }];
  assert.deepEqual(pickTrack(tracks, "en", "Audio"), { id: 2, fallback: false });
  assert.deepEqual(pickTrack(tracks, "de", "Audio"), { id: 1, fallback: true });
  assert.deepEqual(pickTrack(tracks, "en", "Subtitle"), { id: null, fallback: false });
  assert.deepEqual(pickTrack([], "en", "Audio"), { id: null, fallback: false });
  assert.deepEqual(pickTrack([{ id: 7, kind: "Audio", language: "und" }], "en", "Audio"), { id: 7, fallback: false }, "unknown language: no false alarm");
});

test("without a real engine the player fails plainly and releases the lease", async () => {
  const h = harness();
  h.player.d.engineFactory = () => new UnsupportedEngine();
  const view = await h.cmd({ action: "start", media_id: M, context: { kind: "standalone", id: null } });
  assert.equal(view.stage, "closed"); assert.match(view.error, /moteur de lecture/);
  assert.equal(h.calls.some(c => c.path.endsWith("/release")), true);
});

test("device ids work in an insecure context (no crypto.randomUUID), and stay valid v4 UUIDs", () => {
  const insecure = { getRandomValues: a => { for (let i = 0; i < a.length; i++) a[i] = (i * 37 + 11) & 0xff; return a; } };
  const id = uuidV4(insecure);
  assert.match(id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.equal(uuidV4({ randomUUID: () => "from-native" }), "from-native");
  assert.match(uuidV4(require("node:crypto").webcrypto), /^[0-9a-f-]{36}$/);
});
