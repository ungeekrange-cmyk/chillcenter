"use strict";
// Registers the WebAssembly MPEG-4 decoder (Xvid/DivX) with the software video path (ADR-0014).
// The decoder lives in a Worker; the .wasm is only fetched when such a video is played.
(function (root) {
  if (!root.ChillSoft || typeof Worker === "undefined") return;
  // MPEG-4 Part 2 fourccs found in AVI. MS-MPEG4 (div2/div3/mp43) is a different codec: not handled here.
  const FOURCC = new Set(["xvid", "divx", "dx50", "div4", "div5", "div6", "mp4v", "fmp4", "3iv2", "3ivx", "dvx1", "mp4s", "m4s2", "rmp4", "dm4v", "blz0", "hdx4", "geox", "fvfw", "ndig", "sedg", "smp4", "pm4v", "wv1f", "xvix", "siph"]);

  class WorkerDecoder {
    constructor(url) {
      this.worker = new Worker(url); this.pending = new Map(); this.n = 0;
      this.worker.onmessage = ev => {
        const { id, ok, error, frames } = ev.data, p = this.pending.get(id);
        if (!p) return;
        this.pending.delete(id);
        if (ok) p.resolve(frames); else p.reject(Object.assign(new Error(error), { code: "PLAYER_ERROR" }));
      };
      this.worker.onerror = () => { for (const p of this.pending.values()) p.reject(Object.assign(new Error("Le décodeur MPEG-4 n'a pas pu démarrer."), { code: "PLAYER_ERROR" })); this.pending.clear(); };
    }
    call(message, transfer) { return new Promise((resolve, reject) => { const id = ++this.n; this.pending.set(id, { resolve, reject }); this.worker.postMessage({ ...message, id }, transfer || []); }); }
    decode(data, key, pts) {
      if (data === null) return this.call({ op: "flush" });
      const copy = data.slice(); // a view would drag its whole underlying buffer through structured clone
      return this.call({ op: "decode", data: copy, key, pts }, [copy.buffer]);
    }
    reset() { return this.call({ op: "reset" }); }
    close() { this.worker.terminate(); }
  }

  root.ChillSoft.decoders.register({
    match: track => FOURCC.has(String(track.type).toLowerCase()),
    label: "Décodeur MPEG-4 (FFmpeg, WebAssembly)",
    async create(track) { const d = new WorkerDecoder("vendor/mpeg4/mpeg4-worker.js"); await d.call({ op: "open", fifo: track.ptsMode === "decode" }); return d; },
  });
})(typeof window !== "undefined" ? window : globalThis);
