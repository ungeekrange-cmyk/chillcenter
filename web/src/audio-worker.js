"use strict";
// Runs the WebAssembly audio transcoder off the main thread. It also handles MP3 only when native MSE lacks support (ADR-0020).
// The page starts this worker as audio-worker.js?v=<build>: every file it loads carries the same version (no stale mix after an update).
const V = self.location.search;
importScripts("audio-transcoder.js" + V, "audio-core.js" + V);
let core = null;
self.onmessage = async ev => {
  const { id, op, frames, codec, rate, channels, kbps } = ev.data;
  try {
    if (op === "open") {
      const M = await createAudioTranscoder({ locateFile: f => new URL(f, self.location.href).href.split("?")[0] + V });
      core = ChillAudioCore.wrap(M, { codec, rate, channels, kbps });
      core.open();
      self.postMessage({ id, ok: true, asc: core.asc, frameSize: core.frameSize, priming: core.priming });
    } else if (op === "convert" || op === "flush") {
      const packets = op === "flush" ? core.flush() : core.convert(frames);
      self.postMessage({ id, ok: true, packets, errors: core.errors }, packets.map(p => p.buffer));
    } else if (op === "reset") { core.reset(); self.postMessage({ id, ok: true }); }
    else if (op === "close") { core.close(); self.postMessage({ id, ok: true }); }
  } catch (e) { self.postMessage({ id, ok: false, error: e.message || String(e) }); }
};
