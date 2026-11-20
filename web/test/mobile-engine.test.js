"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { MseEngine, initialSubtitle } = require("../src/mse-engine.js");

function browserRoot({ mediaSource, managedMediaSource } = {}) {
  const root = {
    MediaSource: mediaSource,
    ManagedMediaSource: managedMediaSource,
    ChillEngines: { factories: [] }, ChillRangeReader: { RangeReader: class {} },
    fetch() {}, document: {}, URL: {}, localStorage: {}, navigator: {},
    requestAnimationFrame() {}, cancelAnimationFrame() {},
  };
  const source = fs.readFileSync(path.join(__dirname, "../src/mse-engine.js"), "utf8");
  vm.runInNewContext(source, { window: root, globalThis: root });
  return root;
}

function engineWith(video) {
  return new MseEngine({ replaceChildren() {} }, {
    storage: { getItem: () => null }, navigator: {}, URL: { createObjectURL: () => "blob:fixture", revokeObjectURL() {} },
  });
}

test("registers the established MediaSource first and uses ManagedMediaSource only as fallback", () => {
  function Mse() {} function Managed() {}
  const classic = browserRoot({ mediaSource: Mse, managedMediaSource: Managed });
  const fallback = browserRoot({ managedMediaSource: Managed });
  assert.equal(classic.ChillEngines.factories.length, 1);
  assert.equal(fallback.ChillEngines.factories.length, 1);
  assert.equal(classic.ChillEngines.factories[0]({}).d.MediaSource, Mse);
  assert.equal(classic.ChillEngines.factories[0]({}).d.managedMediaSource, false);
  assert.equal(fallback.ChillEngines.factories[0]({}).d.MediaSource, Managed);
  assert.equal(fallback.ChillEngines.factories[0]({}).d.managedMediaSource, true);
});

test("ManagedMediaSource disables remote playback before attaching its object URL", () => {
  const order = [];
  function Managed() { order.push("source"); this.addEventListener = () => {}; }
  const video = { set disableRemotePlayback(value) { order.push(`remote:${value}`); }, set src(value) { order.push(`src:${value}`); } };
  const engine = engineWith(video);
  engine.video = video;
  engine.d.managedMediaSource = true;
  engine.d.URL.createObjectURL = () => { order.push("url"); return "blob:managed"; };
  engine.attachMediaSource(Managed);
  assert.deepEqual(order, ["remote:true", "source", "url", "src:blob:managed"]);
});

test("classic MediaSource preserves its existing attachment behavior", () => {
  const order = [];
  function Mse() { order.push("source"); }
  const video = { set disableRemotePlayback(value) { order.push(`remote:${value}`); }, set src(value) { order.push(`src:${value}`); } };
  const engine = engineWith(video);
  engine.video = video;
  engine.d.URL.createObjectURL = () => { order.push("url"); return "blob:classic"; };
  engine.attachMediaSource(Mse);
  assert.deepEqual(order, ["source", "url", "src:blob:classic"]);
});

test("a blocked mobile play is reported, and a later touch retry clears it without dropping media", async () => {
  let calls = 0;
  const video = {
    readyState: 4, paused: true, ended: false, currentTime: 12, playbackRate: 1, volume: 1,
    play: () => ++calls === 1 ? Promise.reject(Object.assign(new Error("gesture"), { name: "NotAllowedError" })) : Promise.resolve(),
  };
  const engine = engineWith(video);
  engine.video = video; engine.movie = { duration: 120 }; engine.audio = [];
  assert.equal(await engine.play(), false);
  assert.equal(engine.snapshot().play_blocked, true);
  assert.match(engine.snapshot().notice, /Tap Play/, "English is the safe default before a locale is known");
  engine.d.document = { documentElement: { lang: "fr-FR" } };
  assert.match(engine.snapshot().notice, /Touchez Lecture/, "French follows the document locale");
  assert.equal(await engine.play(), true);
  assert.equal(engine.snapshot().play_blocked, false);
  assert.equal(engine.video, video, "retry retains the buffered media element");
});

test("a pause or newer retry ignores a stale play rejection", async () => {
  const pending = [];
  const video = {
    readyState: 4, paused: true, ended: false, currentTime: 0, playbackRate: 1, volume: 1,
    play: () => new Promise((resolve, reject) => pending.push({ resolve, reject })), pause() {},
  };
  const engine = engineWith(video);
  engine.video = video; engine.movie = { duration: 120 }; engine.audio = [];
  const first = engine.play();
  engine.pause();
  pending[0].reject(Object.assign(new Error("aborted by pause"), { name: "NotAllowedError" }));
  assert.equal(await first, false);
  assert.equal(engine.snapshot().play_blocked, false);

  const stale = engine.play();
  const current = engine.play();
  pending[2].resolve();
  assert.equal(await current, true);
  pending[1].reject(Object.assign(new Error("old"), { name: "NotAllowedError" }));
  assert.equal(await stale, false);
  assert.equal(engine.snapshot().play_blocked, false);
});

test("non-autoplay failures remain a real error without offering autoplay recovery", async () => {
  const video = {
    readyState: 4, paused: true, ended: false, currentTime: 0, playbackRate: 1, volume: 1,
    play: () => Promise.reject(Object.assign(new Error("codec"), { name: "NotSupportedError" })),
  };
  const engine = engineWith(video);
  engine.video = video; engine.movie = { duration: 120 }; engine.audio = [];
  assert.equal(await engine.play(), false);
  assert.equal(engine.snapshot().play_blocked, false);
  assert.match(engine.snapshot().error, /could not start playback/i);
});

test("a current AbortError is recoverable, while an AbortError made stale by pause is silent", async () => {
  let reject;
  const video = {
    readyState: 4, paused: true, ended: false, currentTime: 0, playbackRate: 1, volume: 1,
    play: () => new Promise((resolve, fail) => { reject = fail; }), pause() {},
  };
  const engine = engineWith(video);
  engine.video = video; engine.movie = { duration: 120 }; engine.audio = [];
  const current = engine.play();
  reject(Object.assign(new Error("interrupted"), { name: "AbortError" }));
  assert.equal(await current, false);
  assert.equal(engine.snapshot().play_blocked, true);
  assert.match(engine.snapshot().notice, /interrupted/i);
  engine.d.document = { documentElement: { lang: "fr" } };
  assert.match(engine.snapshot().notice, /interrompue/i);

  let rejectStale;
  const staleVideo = { ...video, play: () => new Promise((resolve, fail) => { rejectStale = fail; }) };
  const staleEngine = engineWith(staleVideo);
  staleEngine.video = staleVideo; staleEngine.movie = { duration: 120 }; staleEngine.audio = [];
  const stale = staleEngine.play();
  staleEngine.pause();
  rejectStale(Object.assign(new Error("old interruption"), { name: "AbortError" }));
  assert.equal(await stale, false);
  assert.equal(staleEngine.snapshot().play_blocked, false);
});

test("a late rejected play after close is contained and does not revive an error", async () => {
  let reject;
  const video = {
    readyState: 0, paused: true, ended: false, currentTime: 0, playbackRate: 1, volume: 1,
    play: () => new Promise((_, fail) => { reject = fail; }), pause() {}, removeAttribute() {}, load() {},
  };
  const host = { replaceChildren() {} };
  const engine = new MseEngine(host, { storage: { getItem: () => null }, navigator: {}, URL: { revokeObjectURL() {} } });
  engine.video = video;
  const attempt = engine.play();
  engine.close();
  reject(Object.assign(new Error("late"), { name: "NotAllowedError" }));
  assert.equal(await attempt, false);
  assert.equal(engine.playBlocked, undefined);
});


test("startup primes a short buffer rather than awaiting the full cache", async () => {
  const engine = engineWith({});
  engine.video = { currentTime: 0 };
  engine.ms = { readyState: "open", endOfStream() {} };
  engine.cache = { forward: 90, back: 60 };
  let end = 0, reads = 0;
  const sb = { buffered: { get length() { return end > 0 ? 1 : 0; }, start: () => 0, end: () => end } };
  engine.streams = [{ track: {}, sb, done: false }];
  engine.src = { next: async () => { reads++; return { data: new Uint8Array() }; } };
  engine.append = async () => { end += 2; };
  engine.evict = async () => {};
  await engine.pump(3);
  assert.equal(reads, 2, "first picture needs only a few seconds, not 45 network round trips");
  assert.equal(end, 4);
  await engine.pump();
  assert.equal(end, 90, "background buffering can still fill the configured cache");
});

test("eviction retains the current video GOP even with zero back cache", async () => {
  const engine = engineWith({});
  engine.video = { currentTime: 5000 };
  const video = { buffered: { length: 1, start: () => 4995.542 } };
  const audio = { buffered: { length: 1, start: () => 4995.5 } };
  engine.streams = [
    { track: { kind: "video" }, sb: video, keyframes: [4995.542] },
    { track: { kind: "audio" }, sb: audio },
  ];
  const removed = [];
  engine.removeRange = async (sb, start, end) => removed.push({ sb, start, end });
  await engine.evict(video, 0);
  assert.deepEqual(removed, [], "removing to 5000 also discards dependent pictures up to the next keyframe");
  await engine.evict(audio, 0);
  assert.equal(removed[0].end, 5000, "audio has independent samples");
  engine.streams[0].keyframes.push(5001.042);
  engine.video.currentTime = 5003;
  await engine.evict(video, 0);
  assert.equal(removed[1].end, 5001.042, "remove only before a known random-access point");
  assert.deepEqual(engine.streams[0].keyframes, [5001.042]);
});

test("a seek clears random-access points from the old timeline", async () => {
  const engine = engineWith({});
  engine.src = { seek: async () => {} };
  engine.streams = [{ keyframes: [10, 20], done: true }];
  await engine.rewind(100);
  assert.deepEqual(engine.streams[0].keyframes, []);
  assert.equal(engine.streams[0].done, false);
});

test("Safari's managed buffer can stop bulk fetching without starving the first picture", async () => {
  const engine = engineWith({});
  engine.video = { currentTime: 0 };
  engine.ms = { readyState: "open", streaming: false, endOfStream() {} };
  engine.d.managedMediaSource = true;
  engine.cache = { forward: 90, back: 60 };
  let end = 0;
  engine.streams = [{ track: {}, sb: { buffered: { get length() { return end ? 1 : 0; }, start: () => 0, end: () => end } } }];
  engine.src = { next: async () => ({ data: new Uint8Array() }) };
  engine.append = async () => { end += 2; };
  engine.evict = async () => {};
  await engine.pump();
  assert.equal(end, 4);
  engine.ms.streaming = true;
  await engine.pump();
  assert.equal(end, 90);
});

test("a late sidecar response cannot restore subtitles after Off, even when fetch ignores abort", async () => {
  const engine = engineWith({});
  const external = { id: "sub-aa", title: "sample.srt", language: "fr" };
  engine.external = [external]; engine.src = { setSubtitle() {} };
  engine.subtitleBase = "/subtitles"; engine.subtitleQuery = "session=fixture&generation=7";
  let resolve;
  engine.d.fetch = () => new Promise(r => { resolve = r; });
  let selected = null, added = 0;
  engine.captions = { select: t => { selected = t; }, add: rows => { added += rows.length; } };
  const pending = engine.selectExternalSubtitle(external.id);
  await engine.selectSubtitle(null);
  resolve({ ok: true, headers: { get: () => null }, text: async () => "1\n00:00:01,000 --> 00:00:02,000\nTardif" });
  await pending;
  assert.equal(selected, null); assert.equal(added, 0); assert.equal(engine.activeExternal, null);
});

test("an out-of-buffer seek reanchors the native clock after clearing old ranges and retains external cues", async () => {
  const engine = engineWith({});
  engine.video = { currentTime: 1 }; engine.src = {};
  engine.activeExternal = { id: "sub-aa" };
  engine.streams = [{ sb: {} }]; engine.cache = { forward: 90 };
  let cleared = 0, pumpedAt;
  engine.captions = { clear: () => { cleared++; } };
  engine.removeRange = async () => { engine.video.currentTime = 8.7; };
  engine.rewind = async () => {};
  engine.pump = async () => { pumpedAt = engine.video.currentTime; };
  await engine.reposition(1);
  assert.equal(pumpedAt, 1); assert.equal(cleared, 0);
  assert.equal(engine.repositioning, false);
});

test("native seek corrections during a buffer reset cannot replace the user's requested target", async () => {
  const engine = engineWith({}), events = new Map();
  engine.video = { currentTime: 1, addEventListener: (name, fn) => events.set(name, fn) };
  engine.started = true; engine.streams = [{ sb: {} }];
  engine.cache = { forward: 90 }; engine.isBuffered = () => false;
  engine.bind();
  let adjusted = false, pumpedAt;
  engine.removeRange = async () => {
    if (!adjusted) { adjusted = true; engine.video.currentTime = 8.7; events.get("seeking")(); }
  };
  engine.rewind = async () => {};
  engine.pump = async () => { pumpedAt = engine.video.currentTime; };
  await engine.reposition(1); await engine.repositionJob;
  assert.equal(pumpedAt, 1);
});

test("a newer explicit user seek still replaces a reset already queued", async () => {
  const engine = engineWith({});
  engine.video = { currentTime: 0 }; engine.movie = { duration: 100 };
  engine.started = true; engine.streams = [{ sb: {} }]; engine.isBuffered = () => false;
  engine.removeRange = async () => {}; engine.rewind = async () => {};
  let pumpedAt;
  engine.pump = async () => { pumpedAt = engine.video.currentTime; };
  engine.seek(1); engine.seek(12);
  await engine.repositionJob;
  assert.equal(pumpedAt, 12); assert.equal(engine.repositioning, false);
});


test("initial captions retain a lone untagged track without guessing its language", () => {
  const unknown = { id: 3, language: "und", flagDefault: 1 };
  const english = { id: 4, language: "en", flagDefault: 1 };
  const french = { id: 5, language: "fr-FR", flagDefault: 0 };
  assert.equal(initialSubtitle([unknown], "fr"), unknown);
  assert.equal(initialSubtitle([unknown], ""), null, "Off stays off");
  assert.equal(initialSubtitle([unknown], "off"), null);
  assert.equal(initialSubtitle([english], "fr"), null, "known mismatch stays off");
  assert.equal(initialSubtitle([{ ...unknown, flagDefault: 0 }], "fr").id, 3, "container default does not erase the profile subtitle preference");
  assert.equal(initialSubtitle([unknown, { ...unknown, id: 8 }], "fr"), null, "ambiguous tracks stay off");
  assert.equal(initialSubtitle([unknown, french], "fr"), french, "exact language wins");
});
