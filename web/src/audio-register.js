"use strict";
// Factory used by the MKV demuxer to convert audio the browser cannot decode (AC-3, E-AC-3, DTS, TrueHD) into AAC (ADR-0020).
// The transcoder lives in a Worker; the .wasm (~1 MB) is only fetched when such a track is played.
(function (root) {
  if (typeof Worker === "undefined") return;
  const V = (root.document && root.document.currentScript && new URL(root.document.currentScript.src).search) || "";   // ?v=<build> of this page
  class WorkerTranscoder {
    constructor(url, params) {
      this.params = params; this.worker = new Worker(url); this.pending = new Map(); this.n = 0;
      this.worker.onmessage = ev => {
        const { id, ok, error } = ev.data, p = this.pending.get(id);
        if (!p) return;
        this.pending.delete(id);
        if (ok) p.resolve(ev.data); else p.reject(Object.assign(new Error(error), { code: "PLAYER_ERROR" }));
      };
      this.worker.onerror = () => { for (const p of this.pending.values()) p.reject(Object.assign(new Error("Le transcodeur audio n'a pas pu démarrer."), { code: "PLAYER_ERROR" })); this.pending.clear(); };
    }
    call(message, transfer) { return new Promise((resolve, reject) => { const id = ++this.n; this.pending.set(id, { resolve, reject }); this.worker.postMessage({ ...message, id }, transfer || []); }); }
    async open() { const r = await this.call({ op: "open", ...this.params }); this.asc = r.asc; this.frameSize = r.frameSize; this.priming = r.priming; }
    async convert(frames) {
      const copies = frames.map(f => f.slice());   // a view would drag its whole underlying buffer through structured clone
      return (await this.call({ op: "convert", frames: copies }, copies.map(c => c.buffer))).packets;
    }
    async flush() { return (await this.call({ op: "flush" })).packets; }
    reset() { return this.call({ op: "reset" }); }
    close() { this.worker.terminate(); }
  }
  root.ChillAudio = { create: params => new WorkerTranscoder("vendor/audio/audio-worker.js" + V, params) };
})(typeof window !== "undefined" ? window : globalThis);
