"use strict";
// Needs the built decoder (web/vendor/mpeg4) and ffmpeg/ffprobe (reference decoder + fixtures).
const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { wrap } = require("../src/mpeg4-core.js");
const { AviSource } = require("../src/avi.js");

const wasm = path.join(__dirname, "../vendor/mpeg4/mpeg4-decoder.wasm");
const ready = fs.existsSync(wasm) && spawnSync("ffmpeg", ["-version"]).status === 0;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "chill-m4-"));
const W = 320, H = 176;

function makeAvi(name, extra) {
  const file = path.join(dir, name);
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", "-f", "lavfi", "-i", `testsrc2=size=${W}x${H}:rate=25:duration=6`, "-c:v", "mpeg4", "-vtag", "XVID", "-an", ...extra, file]);
  assert.equal(r.status, 0, String(r.stderr));
  return file;
}
function reference(file) { // frames decoded by the native ffmpeg, I420, tightly packed
  const out = path.join(dir, path.basename(file) + ".yuv");
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", "-i", file, "-f", "rawvideo", "-pix_fmt", "yuv420p", out]);
  assert.equal(r.status, 0, String(r.stderr));
  return fs.readFileSync(out);
}
const reader = file => { const buf = fs.readFileSync(file); return { size: buf.length, read: async (a, b) => new Uint8Array(buf.subarray(a, b + 1)) }; };
function lumaOf(frame) { const o = new Uint8Array(frame.width * frame.height); for (let r = 0; r < frame.height; r++) o.set(frame.y.subarray(r * frame.strideY, r * frame.strideY + frame.width), r * frame.width); return o; }
function psnr(a, b) { let se = 0; for (let i = 0; i < a.length; i++) { const d = a[i] - b[i]; se += d * d; } return se === 0 ? 99 : 10 * Math.log10(255 * 255 / (se / a.length)); }

async function setup(name, extra) {
  const file = makeAvi(name, extra);
  const src = new AviSource(reader(file));
  const movie = await src.open();
  const track = movie.tracks[0];
  const samples = await src.rawRead(track, 0, track.count);
  const M = await require("../vendor/mpeg4/mpeg4-decoder.js")();
  const dec = wrap(M, { fifo: true }); dec.open();
  return { file, track, samples, dec, ref: reference(file) };
}

test("every frame comes back, in order, matching ffmpeg's own decode (with B-frames)", { skip: !ready }, async () => {
  const { track, samples, dec, ref } = await setup("b.avi", ["-bf", "2", "-g", "25"]);
  const frames = [];
  for (const s of samples) frames.push(...dec.decode(s.data, s.key, s.dts + s.cts));
  frames.push(...dec.decode(null, false, 0));
  assert.equal(frames.length, track.count, "one picture per packet after flushing the reorder delay");
  for (let i = 1; i < frames.length; i++) assert.ok(frames[i].pts > frames[i - 1].pts, "timestamps follow presentation order");
  assert.deepEqual([frames[0].width, frames[0].height], [W, H]);
  const frameBytes = W * H * 3 / 2;
  let worst = 99;
  for (const i of [0, 1, 2, 3, 10, 37, 74, 100, 149]) {
    worst = Math.min(worst, psnr(lumaOf(frames[i]), ref.subarray(i * frameBytes, i * frameBytes + W * H)));
  }
  assert.ok(worst > 40, `luma PSNR vs native ffmpeg: ${worst.toFixed(1)} dB`);
});

test("reset restarts cleanly from a keyframe (what a seek does)", { skip: !ready }, async () => {
  const { track, samples, dec, ref } = await setup("s.avi", ["-bf", "0", "-g", "25"]);
  for (const s of samples.slice(0, 60)) dec.decode(s.data, s.key, s.dts + s.cts);
  dec.reset();
  const k = [...track.sync].findIndex((v, i) => v && i >= 75);
  const out = [];
  for (let i = k; i < k + 10; i++) out.push(...dec.decode(samples[i].data, samples[i].key, samples[i].dts));
  assert.equal(out.length, 10);
  const frameBytes = W * H * 3 / 2;
  assert.ok(psnr(lumaOf(out[0]), ref.subarray(k * frameBytes, k * frameBytes + W * H)) > 40);
  assert.equal(out[0].pts, samples[k].dts, "first output carries the first input timestamp");
});

test("a damaged packet is skipped without throwing and decoding continues", { skip: !ready }, async () => {
  const { samples, dec } = await setup("c.avi", ["-bf", "0", "-g", "25"]);
  let n = 0;
  for (let i = 0; i < 30; i++) n += dec.decode(samples[i].data, samples[i].key, samples[i].dts).length;
  const junk = Uint8Array.from({ length: 400 }, (_, i) => (i * 37) % 251);
  assert.doesNotThrow(() => dec.decode(junk, false, 99999));
  for (let i = 30; i < 60; i++) n += dec.decode(samples[i].data, samples[i].key, samples[i].dts).length;
  assert.ok(n >= 55, `frames after damage: ${n}`);
});

test("throughput at 320x176 is far above real time (measured, logged)", { skip: !ready }, async () => {
  const { samples, dec } = await setup("p.avi", ["-bf", "2", "-g", "25"]);
  const t0 = process.hrtime.bigint();
  let n = 0;
  for (const s of samples) n += dec.decode(s.data, s.key, s.dts).length;
  const ms = Number(process.hrtime.bigint() - t0) / 1e6;
  const fps = n / (ms / 1000);
  console.log(`# wasm decode ${W}x${H}: ${n} frames in ${ms.toFixed(0)} ms = ${fps.toFixed(0)} i/s (x${(fps / 25).toFixed(0)} temps réel)`);
  assert.ok(fps > 100);
});
