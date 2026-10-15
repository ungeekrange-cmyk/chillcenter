"use strict";
// Needs ffmpeg/ffprobe (fixtures generated at test time, never committed).
const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const Mp4 = require("../src/mp4.js");
const { AviSource } = require("../src/avi.js");

const have = spawnSync("ffmpeg", ["-version"]).status === 0 && /libmp3lame/.test(String(spawnSync("ffmpeg", ["-encoders"]).stdout));
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "chill-avi-"));
function make(name, args) {
  const file = path.join(dir, name);
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=320x176:rate=25:duration=10", "-f", "lavfi", "-i", "sine=frequency=440:duration=10", ...args, file]);
  assert.equal(r.status, 0, String(r.stderr));
  return file;
}
const reader = file => { const buf = fs.readFileSync(file); return { size: buf.length, read: async (a, b) => new Uint8Array(buf.subarray(a, b + 1)) }; };
const packets = (file, sel) => String(spawnSync("ffprobe", ["-v", "error", "-select_streams", sel, "-show_entries", "packet=pts", "-of", "csv=p=0", file]).stdout).trim().split("\n").filter(Boolean).length;

test("AVI Xvid-style video + CBR MP3: tracks, counts, timing and a decodable audio rebuild", { skip: !have }, async () => {
  const file = make("a.avi", ["-c:v", "mpeg4", "-vtag", "XVID", "-bf", "0", "-g", "25", "-c:a", "libmp3lame", "-b:a", "128k", "-shortest"]);
  const src = new AviSource(reader(file));
  const movie = await src.open();
  const v = movie.tracks.find(t => t.kind === "video"), a = movie.tracks.find(t => t.kind === "audio");
  assert.equal(v.type, "xvid"); assert.equal(v.width, 320); assert.equal(v.height, 176); assert.equal(v.mime, null);
  assert.equal(v.decoder.fourcc, "xvid");
  assert.equal(v.count, packets(file, "v:0"));
  assert.ok(v.sync[0] === 1 && v.sync.reduce((x, y) => x + y) >= 10, "keyframes every 25 frames");
  assert.ok(Math.abs(movie.duration - 10) < 0.5, String(movie.duration));
  assert.equal(a.codec, "mp4a.6B"); assert.equal(a.timescale, 44100);
  assert.equal(a.count, packets(file, "a:0"));
  // Rebuild the audio as fMP4 and let ffmpeg decode it.
  await src.seek(0);
  const parts = [Mp4.initSegment(a)];
  for (let f; (f = await src.next(a.id));) parts.push(f.data);
  const out = path.join(dir, "audio.mp4"); fs.writeFileSync(out, Mp4.concat(parts));
  const r = spawnSync("ffmpeg", ["-v", "error", "-i", out, "-f", "null", "-"]);
  assert.equal(r.status, 0, String(r.stderr));
});

test("video sample tables follow frame position and seeking snaps to a keyframe", { skip: !have }, async () => {
  const file = make("b.avi", ["-c:v", "mpeg4", "-vtag", "XVID", "-bf", "0", "-g", "25", "-an"]);
  const src = new AviSource(reader(file));
  const movie = await src.open();
  const v = movie.tracks[0];
  assert.equal(v.kind, "video");
  const i = Mp4.sampleAtTime(v, 6.3);
  assert.equal(v.sync[i], 1);
  assert.ok(v.dts[i] / v.timescale <= 6.3 && v.dts[i] / v.timescale >= 5.0);
  for (let k = 1; k < v.count; k++) assert.ok(v.dts[k] > v.dts[k - 1], "monotonic dts");
});

test("AVI without an index is refused plainly; garbage too", async () => {
  await assert.rejects(new AviSource({ size: 64, read: async (a, b) => new Uint8Array(b - a + 1) }).open(), /AVI/);
});

test("engine: an AVI whose video needs a software decoder says so plainly when none is installed", { skip: !have }, async () => {
  const { MseEngine } = require("../src/mse-engine.js");
  const file = make("c.avi", ["-c:v", "mpeg4", "-vtag", "XVID", "-bf", "0", "-g", "25", "-c:a", "libmp3lame", "-shortest"]);
  const buf = fs.readFileSync(file);
  const fetch = async (url, init) => {
    const [, a, b] = /bytes=(\d+)-(\d+)/.exec(init.headers.Range).map(Number), end = Math.min(b, buf.length - 1);
    const h = new Map([["content-range", `bytes ${a}-${end}/${buf.length}`], ["etag", '"e"']]);
    return { status: 206, headers: { get: k => h.get(k) ?? null }, arrayBuffer: async () => buf.subarray(a, end + 1).slice().buffer };
  };
  const MS = function () {}; MS.isTypeSupported = () => true;
  const engine = new MseEngine({ replaceChildren() {} }, { fetch, MediaSource: MS, storage: { getItem: () => null }, navigator: {} });
  await assert.rejects(engine.open({ mediaId: "m", lease: { session_id: "s", generation: 1 }, startAt: 0, speed: 1 }),
    e => e.code === "PLAYER_ERROR" && /décodeur logiciel Xvid/.test(e.message) && !/Mac/.test(e.message));
});
