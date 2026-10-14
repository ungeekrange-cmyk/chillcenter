"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { SoftVideo, i420ToRgba, decoders } = require("../src/soft-video.js");
const { AviSource } = require("../src/avi.js");

const have = spawnSync("ffmpeg", ["-version"]).status === 0;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "chill-soft-"));
function avi() {
  const file = path.join(dir, "s.avi");
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=64x48:rate=25:duration=8", "-c:v", "mpeg4", "-vtag", "XVID", "-bf", "0", "-g", "25", "-an", file]);
  assert.equal(r.status, 0, String(r.stderr));
  const buf = fs.readFileSync(file);
  return { size: buf.length, read: async (a, b) => new Uint8Array(buf.subarray(a, b + 1)) };
}

/** Stand-in decoder: one 2x2 frame per sample whose luma encodes the sample's pts index; honours flush/reset. */
class FakeDecoder {
  constructor(delay = 0) { this.delay = delay; this.calls = 0; this.resets = 0; }
  async decode(data, key, pts) {
    this.calls++;
    if (this.delay) await new Promise(r => setTimeout(r, this.delay));
    if (data === null) return [];
    return [{ pts, width: 2, height: 2, y: new Uint8Array(4).fill(16 + (Math.round(pts / 3600) % 70) * 3), u: new Uint8Array(1).fill(128), v: new Uint8Array(1).fill(128) }];
  }
  async reset() { this.resets++; }
  close() { this.closed = true; }
}

function harness({ delay = 0, queueMax = 10 } = {}) {
  return (async () => {
    const src = new AviSource(avi());
    const movie = await src.open();
    const track = movie.tracks[0];
    const drawn = []; let time = 0, resized = null;
    const decoder = new FakeDecoder(delay);
    const sv = new SoftVideo({
      src, track, decoder, clock: { time: () => time },
      ctx: { putImageData: img => drawn.push(img.data[0]) }, resize: (w, h) => { resized = [w, h]; }, makeImage: (w, h) => ({ data: new Uint8ClampedArray(w * h * 4) }),
      raf: () => 1, caf: () => {},
    }, { queueMax, batch: 4 });
    return { sv, drawn, decoder, track, set: t => { time = t; }, resized: () => resized };
  })();
}

test("frames are decoded ahead (bounded), shown in presentation order and the late ones dropped", { skip: !have }, async () => {
  const h = await harness({ queueMax: 6 });
  await h.sv.reset(0);
  assert.ok(h.sv.queue.length >= 6 && h.sv.queue.length <= 6 + 4, `queue ${h.sv.queue.length} bounded by queueMax + one batch`);
  h.set(0.0); h.sv.tick(); h.set(0.2); h.sv.tick();
  assert.deepEqual(h.drawn.length, 2);
  assert.ok(h.drawn[1] > h.drawn[0], "later frame, later luma");
  h.set(1.0); h.sv.tick();               // jumped over several frames: only the newest due one is drawn
  assert.ok(h.sv.stats.dropped > 0);
  assert.deepEqual(h.resized(), [2, 2]);
});

test("seek restarts from the keyframe at or before the target and skips frames before it", { skip: !have }, async () => {
  const h = await harness();
  await h.sv.reset(0);
  const before = h.decoder.resets;
  await h.sv.reset(5.5);
  assert.equal(h.decoder.resets, before + 1);
  const first = h.sv.seconds(h.sv.queue[0].pts);
  assert.ok(first >= 5.5 - 0.25 - 0.05, `first queued frame ${first}`);
  h.set(5.52); h.sv.tick();
  assert.equal(h.drawn.length, 1);
});

test("a stale decode finishing after a newer seek is discarded", { skip: !have }, async () => {
  const h = await harness({ delay: 5 });
  const slow = h.sv.reset(0);
  await new Promise(r => setTimeout(r, 12));
  await h.sv.reset(6);
  await slow;
  assert.ok(h.sv.queue.every(f => h.sv.seconds(f.pts) >= 6 - 0.25), "no frame from the abandoned position");
});

test("end of stream flushes the decoder once and reports finished when drained", { skip: !have }, async () => {
  const h = await harness({ queueMax: 10000 });
  await h.sv.reset(0);
  assert.equal(h.sv.decodedAll, true);
  const flushes = h.decoder.calls;
  await h.sv.fill();
  assert.equal(h.decoder.calls, flushes, "no second flush");
  h.set(100); h.sv.tick();
  assert.equal(h.sv.finished, true);
});

test("a decoder failure surfaces as a message, not an exception", { skip: !have }, async () => {
  const h = await harness();
  h.sv.decoder.decode = async () => { throw new Error("flux corrompu"); };
  await h.sv.reset(0);
  assert.equal(h.sv.errorText, "flux corrompu");
});

test("I420 -> RGBA matches BT.601 limited range on reference colours", () => {
  const px = (Y, U, V) => Array.from(i420ToRgba({ width: 2, height: 2, y: new Uint8Array(4).fill(Y), u: new Uint8Array([U]), v: new Uint8Array([V]) }, new Uint8ClampedArray(16)).subarray(0, 3));
  assert.deepEqual(px(16, 128, 128), [0, 0, 0]);
  assert.deepEqual(px(235, 128, 128), [255, 255, 255]);
  const red = px(81, 90, 240);
  assert.ok(red[0] > 245 && red[1] < 10 && red[2] < 10, String(red));
});

test("decoder registry matches by track", () => {
  const entry = { match: t => t.type === "xvid", create: async () => new FakeDecoder() };
  decoders.register(entry);
  assert.equal(decoders.find({ type: "xvid" }), entry);
  assert.equal(decoders.find({ type: "h264" }), null);
  decoders.list.length = 0;
});
