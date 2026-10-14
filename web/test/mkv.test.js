"use strict";
// Needs ffmpeg/ffprobe (fixtures generated at test time, never committed).
const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const Mp4 = require("../src/mp4.js");
const { MkvSource, blockFrames } = require("../src/mkv.js");

const have = spawnSync("ffmpeg", ["-version"]).status === 0;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "chill-mkv-"));
const lavfi = ["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=24:duration=8", "-f", "lavfi", "-i", "sine=frequency=440:duration=8"];
function make(name, args) {
  const file = path.join(dir, name);
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", ...lavfi, ...args, file]);
  assert.equal(r.status, 0, String(r.stderr));
  return file;
}
function reader(file) { const buf = fs.readFileSync(file); return { size: buf.length, read: async (a, b) => new Uint8Array(buf.subarray(a, b + 1)) }; }
function packets(file, kind) {
  const r = spawnSync("ffprobe", ["-v", "error", "-select_streams", kind === "video" ? "v:0" : "a:0", "-show_entries", "packet=pts", "-of", "csv=p=0", file]);
  return String(r.stdout).trim().split("\n").filter(Boolean).length;
}
async function remux(file, seekTo) {
  const src = new MkvSource(reader(file));
  const movie = await src.open();
  if (seekTo !== undefined) await src.seek(seekTo);
  const outputs = [];
  for (const track of movie.tracks.filter(t => t.mime)) {
    const parts = [Mp4.initSegment(track)];
    for (let f; (f = await src.next(track.id));) parts.push(f.data);
    const out = path.join(dir, `${path.basename(file)}.${seekTo ?? 0}.${track.kind}.mp4`);
    fs.writeFileSync(out, Mp4.concat(parts));
    outputs.push({ track, out });
  }
  return { src, movie, outputs };
}
const decodes = f => spawnSync("ffmpeg", ["-v", "error", "-i", f, "-f", "null", "-"]);

test("MKV H.264 + AAC with B-frames: tracks, codecs, every packet rebuilt and decodable", { skip: !have }, async () => {
  const file = make("h264.mkv", ["-c:v", "libx264", "-bf", "2", "-g", "24", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"]);
  const { movie, outputs } = await remux(file);
  assert.ok(movie.duration > 7 && movie.duration < 9, String(movie.duration));
  const v = movie.tracks.find(t => t.kind === "video"), a = movie.tracks.find(t => t.kind === "audio");
  assert.match(v.codec, /^avc1\.[0-9a-f]{6}$/); assert.equal(a.codec, "mp4a.40.2"); assert.equal(v.width, 320);
  for (const { track, out } of outputs) {
    const r = decodes(out); assert.equal(r.status, 0, String(r.stderr)); assert.equal(String(r.stderr).trim(), "");
    assert.equal(packets(out, track.kind), packets(file, track.kind), track.kind);
  }
});

test("MKV VP9 + Opus rebuilds and decodes", { skip: !have }, async () => {
  const file = make("vp9.mkv", ["-c:v", "libvpx-vp9", "-b:v", "200k", "-g", "24", "-c:a", "libopus", "-shortest"]);
  const { movie, outputs } = await remux(file);
  assert.match(movie.tracks.find(t => t.kind === "video").codec, /^vp09\.00\.10\.08$/);
  assert.equal(movie.tracks.find(t => t.kind === "audio").codec, "opus");
  for (const { track, out } of outputs) { assert.equal(decodes(out).status, 0); assert.equal(packets(out, track.kind), packets(file, track.kind)); }
});

test("seeking uses Cues, starts on a keyframe, and yields fewer packets", { skip: !have }, async () => {
  const file = make("seek.mkv", ["-c:v", "libx264", "-g", "24", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", "-cluster_time_limit", "1000"]);
  const full = await remux(file);
  const part = await remux(file, 4.5);
  assert.ok(part.src.cues.length > 1, "cues expected");
  const v = part.outputs.find(o => o.track.kind === "video");
  assert.ok(packets(v.out, "video") < packets(file, "video"));
  assert.equal(decodes(v.out).status, 0);
  const first = spawnSync("ffprobe", ["-v", "error", "-select_streams", "v:0", "-show_entries", "packet=pts_time,flags", "-of", "csv=p=0", "-read_intervals", "%+#1", v.out]);
  const [time, flags] = String(first.stdout).trim().split(",");
  assert.ok(Number(time) <= 4.5 && Number(time) >= 2.5, `starts at ${time}`);
  assert.match(flags, /K/);
  assert.ok(full.outputs.length === 2);
});

test("file without Cues still seeks by hopping over cluster headers", { skip: !have }, async () => {
  const file = make("nocues.mkv", ["-c:v", "libx264", "-g", "24", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", "-cluster_time_limit", "1000", "-write_crc32", "0"]);
  const src = new MkvSource(reader(file));
  await src.open(); src.cues = [];
  const early = await src.clusterFor(0.1), late = await src.clusterFor(6.2);
  assert.ok(late > early);
});

test("lacing: fixed, Xiph and EBML frames are split exactly", () => {
  const block = (flags, ...rest) => Uint8Array.from([0x81, 0, 0, flags, ...rest]);
  const fixed = block(0x04, 2, 1, 1, 2, 2, 3, 3);
  assert.deepEqual(blockFrames(fixed, 0, fixed.length).frames.map(f => [...f]), [[1, 1], [2, 2], [3, 3]]);
  const xiph = block(0x02, 2, 2, 3, 9, 9, 8, 8, 8, 7, 7, 7, 7);
  assert.deepEqual(blockFrames(xiph, 0, xiph.length).frames.map(f => f.length), [2, 3, 4]);
  const ebml = block(0x06, 2, 0x82, 0xc0, 9, 9, 8, 8, 8, 7, 7, 7, 7);
  assert.deepEqual(blockFrames(ebml, 0, ebml.length).frames.map(f => f.length), [2, 3, 4]);
  const plain = block(0x80, 5, 6);
  const one = blockFrames(plain, 0, plain.length);
  assert.equal(one.key, true); assert.deepEqual([...one.frames[0]], [5, 6]);
});

test("truncated MKV is a plain error", async () => {
  await assert.rejects(new MkvSource({ size: 10, read: async (a, b) => new Uint8Array(b - a + 1) }).open(), /MKV/);
});
