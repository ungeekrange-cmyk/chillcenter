"use strict";
// Runs the WebAssembly MPEG-4 decoder off the main thread. Served from vendor/mpeg4/ next to the .wasm.
importScripts("mpeg4-decoder.js", "mpeg4-core.js");
let core = null;
self.onmessage = async ev => {
  const { id, op, data, key, pts, fifo } = ev.data;
  try {
    if (op === "open") {
      const M = await createMpeg4Decoder({ locateFile: f => new URL(f, self.location.href).href });
      core = ChillMpeg4Core.wrap(M, { fifo: Boolean(fifo) });
      core.open();
      self.postMessage({ id, ok: true });
    } else if (op === "decode" || op === "flush") {
      const frames = core.decode(op === "flush" ? null : data, key, pts);
      self.postMessage({ id, ok: true, frames }, frames.flatMap(f => [f.y.buffer, f.u.buffer, f.v.buffer]));
    } else if (op === "reset") { core.reset(); self.postMessage({ id, ok: true }); }
    else if (op === "close") { core.close(); self.postMessage({ id, ok: true }); }
  } catch (e) { self.postMessage({ id, ok: false, error: e.message || String(e) }); }
};
