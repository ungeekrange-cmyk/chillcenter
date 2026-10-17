"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { MseEngine } = require("../src/mse-engine.js");

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
  function Managed() { order.push("source"); }
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
