"use strict";
// Thin JS layer over the WebAssembly MPEG-4 decoder (web/decoder/mpeg4). Pure logic, shared by the
// Worker and by the Node tests: packets in, I420 frames out, errors as plain messages.
(function (root) {
  const fail = (op, code) => Object.assign(new Error(`Décodeur MPEG-4 : ${op} a échoué (code ${code}).`), { code: "PLAYER_ERROR" });
  const YUV420P = 0, YUVJ420P = 12;

  // opts.fifo: the container only carries decode-order timestamps (AVI). With B-frames the frame that
  // comes out is not the one that went in, so output frames take the input timestamps in order
  // (presentation order of constant-rate video = order of the timestamps).
  function wrap(M, opts = {}) {
    let corrupt = 0;
    const times = [];
    function drain(out) {
      for (;;) {
        const r = M._cc_receive();
        if (r === 0) return;
        if (r < 0) throw fail("lecture d'image", r);
        const fmt = M._cc_format();
        if (fmt !== YUV420P && fmt !== YUVJ420P) throw Object.assign(new Error(`Format de pixels MPEG-4 non pris en charge (${fmt}).`), { code: "PLAYER_ERROR" });
        const w = M._cc_width(), h = M._cc_height(), ch = (h + 1) >> 1;
        const sy = M._cc_stride(0), su = M._cc_stride(1), sv = M._cc_stride(2);
        const heap = M.HEAPU8; // fetched after each call: the heap may have grown
        const p0 = M._cc_plane(0), p1 = M._cc_plane(1), p2 = M._cc_plane(2);
        out.push({ pts: opts.fifo && times.length ? times.shift() : M._cc_pts(), width: w, height: h, strideY: sy, strideUV: su, y: heap.slice(p0, p0 + sy * h), u: heap.slice(p1, p1 + su * ch), v: heap.slice(p2, p2 + sv * ch) });
      }
    }
    return {
      open() { const r = M._cc_open(); if (r < 0) throw fail("ouverture", r); },
      /** data === null flushes the decoder (end of stream) and leaves it ready for reuse. */
      decode(data, key, pts) {
        const out = [];
        if (data === null) { M._cc_flush_send(); drain(out); M._cc_reset(); times.length = 0; return out; }
        const p = M._cc_alloc(data.length);
        M.HEAPU8.set(data, p);
        let r;
        try { r = M._cc_send(p, data.length, pts); } finally { M._cc_free(p); }
        if (r < 0) { corrupt++; return out; } // a damaged frame is skipped, playback continues
        times.push(pts);
        drain(out);
        return out;
      },
      reset() { M._cc_reset(); times.length = 0; },
      close() { M._cc_close(); },
      get corruptPackets() { return corrupt; },
    };
  }

  const api = { wrap };
  if (typeof module !== "undefined") module.exports = api; else root.ChillMpeg4Core = api;
})(typeof self !== "undefined" ? self : globalThis);
