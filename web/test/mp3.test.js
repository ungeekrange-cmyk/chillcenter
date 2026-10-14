"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { mp3Resync, mp3FrameLength } = require("../src/mp4.js");

const have = spawnSync("ffmpeg", ["-version"]).status === 0 && /libmp3lame/.test(String(spawnSync("ffmpeg", ["-encoders"]).stdout));

function mp3(args) {
  const file = path.join(fs.mkdtempSync(path.join(os.tmpdir(), "chill-mp3-")), "a.mp3");
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=3", "-c:a", "libmp3lame", ...args, "-id3v2_version", "0", "-write_xing", "0", file]);
  assert.equal(r.status, 0, String(r.stderr));
  return new Uint8Array(fs.readFileSync(file));
}
function boundaries(b) { const out = []; for (let p = 0, n; p < b.length && (n = mp3FrameLength(b, p)); p += n) out.push(p); return out; }

test("resync lands on a real frame boundary after a cut in the middle of a frame (CBR and VBR)", { skip: !have }, () => {
  for (const args of [["-b:a", "128k"], ["-q:a", "4"], ["-b:a", "64k", "-ar", "22050"]]) {
    const full = mp3(args), frames = boundaries(full);
    assert.ok(frames.length > 50, `${args} frames ${frames.length}`);
    for (const cut of [frames[10] + 1, frames[10] + 37, frames[20] + 100]) {
      const out = mp3Resync(full.subarray(cut));
      const offset = full.length - out.length;
      assert.ok(frames.includes(offset) && offset > cut, `${args} cut ${cut} -> ${offset}`);
    }
    assert.equal(mp3Resync(full.subarray(frames[5])).length, full.length - frames[5], "already aligned: untouched");
  }
});

test("data without any valid frame is returned unchanged; headers are validated", () => {
  const junk = new Uint8Array(500).fill(7);
  assert.equal(mp3Resync(junk), junk);
  assert.equal(mp3FrameLength(Uint8Array.from([0xff, 0xfb, 0x90, 0x00]), 0), 417, "MPEG1 L3 128 kb/s 44.1 kHz");
  assert.equal(mp3FrameLength(Uint8Array.from([0xff, 0xfb, 0xf0, 0x00]), 0), 0, "bitrate index 15 is invalid");
  assert.equal(mp3FrameLength(Uint8Array.from([0xff, 0xfd, 0x90, 0x00]), 0), 0, "layer II is not layer III");
});
