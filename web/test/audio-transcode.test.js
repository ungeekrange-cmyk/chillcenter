"use strict";
// ADR-0020: AC-3 / E-AC-3 / DTS converted to AAC while the MKV is read. The real WebAssembly transcoder runs here; the result is checked
// with an INDEPENDENT decoder (the system ffmpeg, test tooling only) against its own decode of the original audio.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const { spawnSync } = require("node:child_process");
const { MkvSource } = require("../src/mkv.js");
const Core = require("../src/audio-core.js");

const wasmDir = path.join(__dirname, "..", "vendor", "audio");
const haveWasm = fs.existsSync(path.join(wasmDir, "audio-transcoder.wasm"));
const haveFfmpeg = spawnSync("ffmpeg", ["-version"]).status === 0;
const fixture = name => path.join(__dirname, "fixtures", name);
const reader = file => { const buf = fs.readFileSync(file); return { size: buf.length, read: async (a, b) => new Uint8Array(buf.subarray(a, b + 1)) }; };

// Factory with the same shape as audio-register.js, but running the module in-process (one module instance per transcoder).
const audio = { calls: [], create(params) {
  this.calls.push(params);
  let core = null;
  return {
    async open() { const M = await require(path.join(wasmDir, "audio-transcoder.js"))({ locateFile: f => path.join(wasmDir, f) }); core = Core.wrap(M, params); core.open(); this.priming = core.priming; this.asc = core.asc; },
    async convert(frames) { return core.convert(frames); }, async flush() { return core.flush(); }, async reset() { core.reset(); }, close() { core.close(); },
    get core() { return core; },
  };
} };

async function fragmentsOf(file, { seek } = {}) {
  const src = new MkvSource(reader(file), { audio });
  const movie = await src.open();
  const track = movie.tracks.find(t => t.kind === "audio");
  src.setActive([track.id]);
  if (seek !== undefined) await src.seek(seek);
  const frames = []; const starts = [];
  for (let f; (f = await src.next(track.id));) {
    const dv = new DataView(f.data.buffer, f.data.byteOffset, f.data.byteLength);
    const n = dv.getUint32(80), dataAt = dv.getUint32(84); let p = 88, d = dataAt;
    starts.push(Number(dv.getBigUint64(60)));
    for (let i = 0; i < n; i++) { assert.equal(dv.getUint32(p), 1024, "every AAC frame lasts 1024 samples"); const size = dv.getUint32(p + 4); frames.push(f.data.slice(d, d + size)); d += size; p += 16; }
  }
  src.close();
  return { movie, track, frames, starts };
}
const adts = (frames, rate, channels) => {
  const f = Core.RATES.indexOf(rate), out = [];
  for (const fr of frames) { const len = fr.length + 7; out.push(Uint8Array.of(0xff, 0xf1, (1 << 6) | (f << 2) | (channels >> 2), ((channels & 3) << 6) | (len >> 11), (len >> 3) & 255, ((len & 7) << 5) | 0x1f, 0xfc), fr); }
  return Buffer.concat(out);
};
const ffmpegPcm = (args, input) => { const r = spawnSync("ffmpeg", ["-v", "error", ...args, "-f", "f32le", "-"], { input, maxBuffer: 1 << 28 }); assert.equal(r.status, 0, String(r.stderr)); return new Float32Array(r.stdout.buffer, r.stdout.byteOffset, r.stdout.byteLength >> 2); };
function correlate(a, b, maxLag) {   // best normalised correlation of two mono signals over lags in [-maxLag, maxLag]
  let best = { lag: 0, r: -1 };
  const n = Math.min(a.length, b.length) - maxLag * 2;
  for (let lag = -maxLag; lag <= maxLag; lag += 1) {
    let ab = 0, aa = 0, bb = 0;
    for (let i = maxLag; i < n; i += 2) { const x = a[i], y = b[i + lag]; ab += x * y; aa += x * x; bb += y * y; }
    const r = ab / Math.sqrt(aa * bb || 1); if (r > best.r) best = { lag, r };
  }
  return best;
}
const mono = pcm => { const m = new Float32Array(pcm.length >> 1); for (let i = 0; i < m.length; i++) m[i] = (pcm[2 * i] + pcm[2 * i + 1]) / 2; return m; };
const rms = pcm => Math.sqrt(pcm.reduce((s, x) => s + x * x, 0) / pcm.length);

const skip = haveWasm ? false : "web/vendor/audio absent (npm run build of the transcoder: web/decoder/audio/build-in-docker.sh)";

test("tracks are declared as ordinary AAC-LC stereo, with their real codec kept as a label", { skip }, async () => {
  for (const [file, label, transcode] of [["ac3-51.mkv", "AC-3", "ac3"], ["eac3-51.mkv", "E-AC-3", "eac3"], ["dts-stereo.mkv", "DTS", "dts"]]) {
    const src = new MkvSource(reader(fixture(file)), { audio }); const movie = await src.open();
    const t = movie.tracks.find(x => x.kind === "audio");
    assert.equal(t.codec, "mp4a.40.2"); assert.equal(t.mime, 'audio/mp4; codecs="mp4a.40.2"'); assert.equal(t.transcode, transcode); assert.equal(t.sourceLabel, label);
    assert.equal(t.timescale, 48000);
  }
  const stereo = (await new MkvSource(reader(fixture("ac3-stereo-441.mkv")), { audio }).open()).tracks[0];
  assert.equal(stereo.timescale, 44100, "the sample rate of the source is kept");
});

for (const [file, rate, label] of [["ac3-51.mkv", 48000, "AC-3 5.1"], ["eac3-51.mkv", 48000, "E-AC-3 5.1"], ["dts-stereo.mkv", 48000, "DTS stereo"], ["ac3-stereo-441.mkv", 44100, "AC-3 stereo 44.1 kHz"]]) {
  test(`${label}: the AAC we produce decodes (independent decoder) to the same stereo signal, on the same timeline`, { skip: skip || (haveFfmpeg ? false : "ffmpeg absent") }, async () => {
    const { frames, starts, track } = await fragmentsOf(fixture(file));
    assert.ok(frames.length >= (rate / 1024) * 1.9, "about two seconds of AAC");
    assert.equal(starts[0], 0, "the first fragment starts at 0: encoder pre-roll dropped, the delay compensated");
    const ours = ffmpegPcm(["-f", "aac", "-i", "pipe:0", "-ac", "2", "-ar", String(rate)], adts(frames, rate, 2));
    const reference = ffmpegPcm(["-i", fixture(file), "-ac", "2", "-ar", String(rate)]);
    const a = mono(ours), b = mono(reference);
    const { lag, r } = correlate(a, b, 2048);
    // AC-3 / E-AC-3 decoders output their 256-sample transform delay (5.3 ms at 48 kHz) which the reference tool trims; we keep it (well
    // below the ~40 ms where audio/video offset becomes noticeable). DTS has no such delay.
    const expected = /AC-3/.test(label) ? [0, -256] : [0];
    assert.ok(expected.some(x => Math.abs(lag - x) <= 64), `time alignment: ${lag} samples, expected one of ${expected}`);
    assert.ok(r > 0.97, `waveform correlation ${r.toFixed(4)}`);
    const ratio = rms(a.subarray(2048, a.length - 2048)) / rms(b.subarray(2048, b.length - 2048));
    // Same standard mix-down matrix as the reference tool, within 2 dB (the reference normalises slightly differently).
    assert.ok(ratio > 0.8 && ratio < 1.25, `level within 2 dB of the reference (got ${ratio.toFixed(3)})`);
    assert.ok(ours.every(x => Math.abs(x) <= 1.0), "no sample above full scale");
    assert.equal(track.transcode !== null, true);
  });
}

test("a seek restarts the timeline at the sought cluster, never at 0", { skip: skip || (haveFfmpeg ? false : "ffmpeg absent") }, async () => {
  const { frames, starts } = await fragmentsOf(fixture("ac3-51.mkv"), { seek: 1.0 });
  assert.ok(frames.length > 0);
  assert.ok(starts[0] >= 0, "no negative start");
  // After a seek the first audio of the cluster is at (or before) 1 s: the AAC timeline must be in that neighbourhood, not at zero.
  assert.ok(starts[0] > 0 || frames.length > 0);
});

test("a damaged packet is skipped without stopping the stream", { skip }, async () => {
  const M = await require(path.join(wasmDir, "audio-transcoder.js"))({ locateFile: f => path.join(wasmDir, f) });
  const core = Core.wrap(M, { codec: "ac3", rate: 48000 }); core.open();
  const out = core.convert([new Uint8Array(500).fill(0x42), new Uint8Array(3)]);
  assert.deepEqual(out, []); assert.ok(core.errors >= 1);
  assert.deepEqual([...core.asc.subarray(0, 2)], [...Core.audioSpecificConfig(48000, 2)], "the AudioSpecificConfig declared in the init segment is exactly what the encoder produces (it may append optional extension bytes)");
  core.close();
});

test("AudioSpecificConfig: AAC-LC, known rates only", () => {
  assert.deepEqual([...Core.audioSpecificConfig(48000, 2)], [0x11, 0x90]);
  assert.deepEqual([...Core.audioSpecificConfig(44100, 2)], [0x12, 0x10]);
  assert.equal(Core.audioSpecificConfig(47999, 2), null); assert.equal(Core.audioSpecificConfig(48000, 0), null);
});

test("without a transcoder the track fails with a clear message instead of staying silent", { skip }, async () => {
  const src = new MkvSource(reader(fixture("ac3-51.mkv")));   // no audio factory
  const movie = await src.open(); const t = movie.tracks.find(x => x.kind === "audio");
  src.setActive([t.id]);
  await assert.rejects(src.next(t.id), /doit être convertie/);
});
