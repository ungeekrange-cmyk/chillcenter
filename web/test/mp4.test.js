"use strict";
// Needs ffmpeg/ffprobe on the machine running the tests (fixtures are generated, never committed).
const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const Mp4 = require("../src/mp4.js");

const have = spawnSync("ffmpeg", ["-version"]).status === 0;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "chill-mp4-"));
const lavfi = ["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=24:duration=6", "-f", "lavfi", "-i", "sine=frequency=440:duration=6"];
function make(name, args) {
  const file = path.join(dir, name);
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", ...lavfi, ...args, file]);
  assert.equal(r.status, 0, String(r.stderr));
  return file;
}
const reader = file => { const buf = fs.readFileSync(file); return { size: buf.length, read: async (a, b) => new Uint8Array(buf.subarray(a, b + 1)) }; };

async function remux(file) {
  const { size, read } = reader(file);
  const movie = Mp4.parseMoov(await Mp4.locateMoov(read, size));
  const all = fs.readFileSync(file);
  const outputs = [];
  for (const track of movie.tracks.filter(t => t.kind === "video" || t.kind === "audio")) {
    const parts = [Mp4.initSegment(track)];
    for (let from = 0, seq = 1; from < track.count; seq++) {
      const plan = Mp4.planFragment(track, from, 2);
      parts.push(Mp4.buildFragment(track, plan.from, plan.to, seq, (o, s) => new Uint8Array(all.subarray(o, o + s))));
      from = plan.to;
    }
    const out = path.join(dir, `${path.basename(file)}.${track.kind}.mp4`);
    fs.writeFileSync(out, Mp4.concat(parts));
    outputs.push({ track, out });
  }
  return { movie, outputs };
}
const frames = (file, kind) => {
  const r = spawnSync("ffprobe", ["-v", "error", "-select_streams", kind === "video" ? "v:0" : "a:0", "-show_entries", "packet=pts", "-of", "csv=p=0", file]);
  return String(r.stdout).trim().split("\n").filter(Boolean).length;
};
const decodes = file => spawnSync("ffmpeg", ["-v", "error", "-i", file, "-f", "null", "-"]);

test("H.264 + AAC with B-frames: tables, codec strings and a decodable fMP4 rebuild", { skip: !have }, async () => {
  const file = make("h264.mp4", ["-c:v", "libx264", "-bf", "2", "-g", "24", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"]);
  const { movie, outputs } = await remux(file);
  const v = movie.tracks.find(t => t.kind === "video"), a = movie.tracks.find(t => t.kind === "audio");
  assert.match(v.codec, /^avc1\.[0-9a-f]{6}$/); assert.equal(a.codec, "mp4a.40.2");
  assert.equal(v.width, 320); assert.equal(v.height, 180);
  assert.ok(v.cts.some(x => x !== 0), "B-frames expected");
  assert.ok(v.sync[0] === 1 && v.sync.reduce((x, y) => x + y) > 1);
  for (const { track, out } of outputs) {
    const r = decodes(out); assert.equal(r.status, 0, String(r.stderr)); assert.equal(String(r.stderr).trim(), "");
    assert.equal(frames(out, track.kind), track.count);
  }
});

test("VP9 + Opus: codec strings and rebuild", { skip: !have }, async () => {
  const file = make("vp9.mp4", ["-c:v", "libvpx-vp9", "-b:v", "200k", "-c:a", "libopus", "-shortest"]);
  const { movie, outputs } = await remux(file);
  assert.match(movie.tracks.find(t => t.kind === "video").codec, /^vp09\.\d\d\.\d\d\.08$/);
  assert.equal(movie.tracks.find(t => t.kind === "audio").codec, "opus");
  for (const { out } of outputs) assert.equal(decodes(out).status, 0);
});

test("seek lands on a sync sample at or before the target; ranges merge small gaps", { skip: !have }, async () => {
  const file = make("seek.mp4", ["-c:v", "libx264", "-g", "24", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"]);
  const { movie } = await remux(file);
  const v = movie.tracks.find(t => t.kind === "video");
  const i = Mp4.sampleAtTime(v, 3.2);
  assert.equal(v.sync[i], 1);
  assert.ok(v.dts[i] / v.timescale <= 3.2 && v.dts[i] / v.timescale > 2.0);
  const ranges = Mp4.sampleRanges(v, 0, 10, 0);
  assert.ok(ranges.length >= 1 && ranges.every(r => r[1] >= r[0]));
  const merged = Mp4.sampleRanges(v, 0, 10, 10 * 1024 * 1024);
  assert.equal(merged.length, 1);
});

test("moov at the end of the file is found with small header reads only", { skip: !have }, async () => {
  const file = make("tail.mp4", ["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"]); // no faststart: moov last
  const { size, read } = reader(file);
  const reads = [];
  const moov = await Mp4.locateMoov(async (a, b) => { reads.push(b - a + 1); return read(a, b); }, size);
  assert.equal(String.fromCharCode(...moov.subarray(4, 8)), "moov");
  assert.ok(reads.slice(0, -1).every(n => n <= 16));
});

test("garbage is rejected with a plain error", async () => {
  await assert.rejects(Mp4.locateMoov(async (a, b) => new Uint8Array(b - a + 1), 64), /MP4/);
  assert.throws(() => Mp4.parseMoov(new Uint8Array(32)), /moov/);
});
