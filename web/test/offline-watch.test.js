"use strict";

const { test } = require("node:test");
const assert = require("node:assert/strict");
const { OfflineWatch } = require("../src/watch-clock.js");

const playing = { loaded: true, paused: false, buffering: false, speed: 1, duration: 20 };

test("local end requires actual uninterrupted playback of the tail", () => {
  const watch = new OfflineWatch();
  for (let position = 17; position < 20; position += 0.5) {
    assert.equal(watch.sample({ ...playing, position }, position * 1000), false);
  }
  assert.equal(watch.sample({ ...playing, position: 20, paused: true, ended: true }, 20_000), true);
});

test("offline auto-deletion gives no credit for a seek or an elapsed pause", () => {
  const watch = new OfflineWatch();
  watch.sample({ ...playing, position: 5 }, 5000);
  watch.sample({ ...playing, position: 19.5, seeking: true }, 5500);
  assert.equal(watch.sample({ ...playing, position: 20, ended: true }, 6000), false);
  watch.reset();
  watch.sample({ ...playing, position: 17.5 }, 0);
  watch.sample({ ...playing, position: 17.5, paused: true }, 500);
  assert.equal(watch.sample({ ...playing, position: 20, paused: true, ended: true }, 15_000), false);
});
