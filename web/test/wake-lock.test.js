"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { ScreenWakeLock, WebPlayer } = require("../src/player-web.js");
const flush = () => new Promise(resolve => setImmediate(resolve));
function api() {
  const grants = [];
  return { grants, async request(type) {
    assert.equal(type, "screen");
    const lock = new EventTarget(); lock.releases = 0;
    lock.release = async () => { ++lock.releases; lock.dispatchEvent(new Event("release")); };
    grants.push(lock); return lock;
  } };
}
function harness() {
  const wakeLock = api(), doc = new EventTarget(), page = new EventTarget();
  let time = 0;
  doc.visibilityState = "visible";
  const state = { loaded: true, paused: false, buffering: false, ended: false, error: null, duration: 1000, position: 12, speed: 1 };
  const player = new WebPlayer({ wakeLock, document: doc, page,
    request: async () => ({}), now: () => time, clearInterval() {},
  });
  player.engine = { snapshot: () => ({ ...state }), pause() { state.paused = true; }, close() {} };
  player.lease = { session_id: "s", generation: 1, media_id: "m", media_revision: 1 };
  player.ctx = { profile: "p" }; player.info = { media: { title: "Film" } };
  player.context = { kind: "standalone" };
  return { player, state, wakeLock, doc, page, advance(ms) { time += ms; } };
}
test("opening the application creates no wake lock; effective playback acquires one", async () => {
  const h = harness(); await flush(); assert.equal(h.wakeLock.grants.length, 0);
  await h.player.status(); await flush(); assert.equal(h.wakeLock.grants.length, 1);
  await h.player.status(); await flush(); assert.equal(h.wakeLock.grants.length, 1);
});
for (const [key, value] of [["loaded", false], ["paused", true], ["buffering", true], ["seeking", true], ["ended", true], ["error", "decode failed"], ["play_blocked", true]]) {
  test(`${key} releases the wake lock`, async () => {
    const h = harness(); h.player.syncWakeLock(); await flush();
    h.state[key] = value; await h.player.status(); await flush();
    assert.equal(h.wakeLock.grants[0].releases, 1);
    assert.equal(h.player.wake.lock, null);
  });
}
test("pause and stop release before a slow progress/release request completes", async () => {
  for (const action of ["pause", "stop"]) {
    const h = harness(); h.player.syncWakeLock(); await flush();
    let finish; h.player.d.request = () => new Promise(resolve => { finish = resolve; });
    const operation = h.player.command({ action }, h.player.ctx);
    assert.equal(h.wakeLock.grants[0].releases, 1);
    finish({}); await flush(); if (action === "stop") finish({});
    await operation;
  }
});
test("visibility and page lifecycle release and reacquire only while still playing", async () => {
  const h = harness(); h.player.syncWakeLock(); await flush();
  h.doc.visibilityState = "hidden"; h.doc.dispatchEvent(new Event("visibilitychange")); await flush();
  assert.equal(h.wakeLock.grants[0].releases, 1);
  h.doc.visibilityState = "visible"; h.doc.dispatchEvent(new Event("visibilitychange")); await flush();
  assert.equal(h.wakeLock.grants.length, 2);
  h.page.dispatchEvent(new Event("pagehide")); await flush();
  assert.equal(h.wakeLock.grants[1].releases, 1);
  h.state.paused = true; h.page.dispatchEvent(new Event("pageshow")); await flush();
  assert.equal(h.wakeLock.grants.length, 2);
});
test("late permission grants are released after pause, including rapid pause/resume", async () => {
  const source = api(); let grant;
  const wake = new ScreenWakeLock({ request: () => new Promise(resolve => { grant = resolve; }) });
  wake.setActive(true); await flush(); wake.setActive(false);
  const first = await source.request("screen"); grant(first); await flush();
  assert.equal(first.releases, 1); assert.equal(wake.lock, null);
  wake.setActive(true); await flush(); wake.setActive(false); wake.setActive(true);
  const old = await source.request("screen"); grant(old); await flush();
  assert.equal(old.releases, 1);
  const current = await source.request("screen"); grant(current); await flush();
  assert.equal(wake.lock, current); wake.setActive(false); await flush();
  assert.equal(current.releases, 1);
});
test("unsupported, denied and system-released locks never retry immediately", async () => {
  for (const wakeLock of [undefined, { request: async () => { throw new Error("NotAllowedError"); } }]) {
    const wake = new ScreenWakeLock(wakeLock); wake.setActive(true); await flush();
    assert.equal(wake.lock, null); wake.setActive(false);
  }
  const source = api(), wake = new ScreenWakeLock(source);
  wake.setActive(true); await flush(); await source.grants[0].release(); await flush();
  assert.equal(wake.lock, null); wake.setActive(true); await flush();
  assert.equal(source.grants.length, 1);
});
test("native media pause events release immediately without waiting for a UI poll", async () => {
  const h = harness(); h.player.engine.video = h.doc;
  h.player.syncWakeLock(); await flush(); h.state.paused = true;
  h.doc.dispatchEvent(new Event("pause"));
  assert.equal(h.wakeLock.grants[0].releases, 1);
});

// Regressions reported after the original playback-only implementation.
test("system revocation during continuous playback is reacquired with backoff", async () => {
  const h = harness(); await h.player.status(); await flush();
  await h.wakeLock.grants[0].release(); await flush();
  await h.player.status(); await flush();
  assert.equal(h.wakeLock.grants.length, 1, "no immediate request loop");
  h.advance(1000); await h.player.status(); await flush();
  assert.equal(h.wakeLock.grants.length, 2, "continuous playback recovers its revoked lock");
  h.state.paused = true; await h.player.status(); await flush();
  assert.equal(h.wakeLock.grants[1].releases, 1);
});
test("denials are retried a bounded number of times and reported only while playing", async () => {
  const h = harness(); let requests = 0;
  h.player.wake.api = { async request() { requests++; throw Error("NotAllowedError"); } };
  await h.player.status(); await flush();
  h.advance(1000); await h.player.status(); await flush();
  h.advance(5000); await h.player.status(); await flush();
  assert.equal(requests, 3);
  for (let i=0;i<20;i++) { h.advance(30000); await h.player.status(); await flush(); }
  assert.equal(requests, 3, "a persistent denial does not cause an endless retry loop");
  assert.match(h.player.view().note, /éveillé/);
  h.state.paused=true; await h.player.status();
  assert.doesNotMatch(h.player.view().note || "", /éveillé/);
});
test("a released sentinel returned by the browser is not mistaken for protection", async () => {
  const h=harness();
  h.player.wake.api={async request(){const sentinel=await h.wakeLock.request("screen"); sentinel.released=true; return sentinel;}};
  await h.player.status(); await flush();
  assert.equal(h.player.wake.lock,null);
});

test("immediate browser revocations share the bounded retry budget", async () => {
  const h=harness();
  for (const delay of [0,1000,5000]) {
    h.advance(delay); await h.player.status(); await flush();
    await h.wakeLock.grants.at(-1).release(); await flush();
  }
  h.advance(60000); await h.player.status(); await flush();
  assert.equal(h.wakeLock.grants.length,3);
  assert.match(h.player.view().note,/éveillé/);
});
test("a stable grant permits recovery again later in a long video", async () => {
  const h=harness();
  for(let i=0;i<5;i++) {
    await h.player.status(); await flush();
    h.advance(30000); await h.wakeLock.grants.at(-1).release(); await flush();
    h.advance(1000);
  }
  await h.player.status(); await flush();
  assert.equal(h.wakeLock.grants.length,6);
});
test("pause cancels a scheduled recovery and resume starts a fresh budget", async () => {
  const h=harness();await h.player.status();await flush();
  await h.wakeLock.grants[0].release(); await flush();
  h.state.paused=true;await h.player.status();h.advance(10000);await h.player.status();await flush();
  assert.equal(h.wakeLock.grants.length,1);
  h.state.paused=false;await h.player.status();await flush();assert.equal(h.wakeLock.grants.length,2);
});
test("a page without the API reports the limitation only during playback", async () => {
  const h=harness();h.player.wake.api=undefined;
  assert.equal(h.player.wake.notice(),null);
  await h.player.status();assert.match(h.player.view().note,/HTTPS/);
  h.state.ended=true;await h.player.status();assert.equal(h.player.wake.notice(),null);
});
