"use strict";
// Software video path (ADR-0016): raw samples -> pluggable decoder -> canvas, paced by an external
// clock (the audio element). The decoder is any object registered in ChillDecoders; none is bundled yet.
//
// Decoder contract (async, may live in a Worker):
//   create(track) -> {decode(data:Uint8Array, key:boolean, pts:number) -> Promise<Frame[]>, reset() -> Promise, close()}
//   Frame = {pts:number (same ticks as the input), width, height, y,u,v,strideY,strideUV}  (I420)
//        or {pts, width, height, rgba:Uint8ClampedArray}
(function (root) {
  const Mp4 = typeof module !== "undefined" ? require("./mp4.js") : root.ChillMp4;

  const decoders = {
    list: [],
    register(entry) { decoders.list.push(entry); },
    find(track) { return decoders.list.find(d => d.match(track)) || null; },
  };

  /** BT.601 limited-range I420 -> RGBA (SD content: a CPU conversion is enough). */
  function i420ToRgba(f, out) {
    const { width: w, height: h, y, u, v, strideY = w, strideUV = w >> 1 } = f;
    for (let row = 0; row < h; row++) {
      for (let col = 0; col < w; col++) {
        const Y = 1.164 * (y[row * strideY + col] - 16), ui = (row >> 1) * strideUV + (col >> 1);
        const U = u[ui] - 128, V = v[ui] - 128, o = (row * w + col) * 4;
        const r = Y + 1.596 * V, g = Y - 0.392 * U - 0.813 * V, b = Y + 2.017 * U;
        out[o] = r < 0 ? 0 : r > 255 ? 255 : r; out[o + 1] = g < 0 ? 0 : g > 255 ? 255 : g; out[o + 2] = b < 0 ? 0 : b > 255 ? 255 : b; out[o + 3] = 255;
      }
    }
    return out;
  }

  class SoftVideo {
    // io: {src (needs rawRead), track, decoder, clock:{time()->seconds}, ctx, resize(w,h), makeImage(w,h)->{data}, raf, caf}
    constructor(io, { queueMax = 10, batch = 6, lateSeconds = 0.25 } = {}) {
      Object.assign(this, io);
      this.queueMax = queueMax; this.batch = batch; this.late = lateSeconds;
      this.queue = []; this.gen = 0; this.cursor = 0; this.running = false; this.drawn = null; this.errorText = null; this.decodedAll = false;
      this.stats = { decoded: 0, drawn: 0, dropped: 0 };
    }
    seconds(pts) { return (pts + this.track.shift) / this.track.timescale; }

    async reset(t) {
      const gen = ++this.gen;
      this.queue = []; this.drawn = null; this.decodedAll = false;
      await this.decoder.reset();
      if (gen !== this.gen) return;
      this.cursor = Mp4.sampleAtTime(this.track, t);
      this.target = t;
      await this.fill();
    }

    async fill() {
      if (this.filling) return;
      this.filling = true;
      const gen = this.gen;
      try {
        while (gen === this.gen && this.queue.length < this.queueMax && this.cursor < this.track.count) {
          const from = this.cursor, samples = await this.src.rawRead(this.track, from, this.batch);
          if (gen !== this.gen) return;
          this.cursor = from + samples.length;
          for (const s of samples) {
            const frames = await this.decoder.decode(s.data, s.key, s.dts + s.cts);
            if (gen !== this.gen) return;
            for (const f of frames) {
              this.stats.decoded++;
              if (this.seconds(f.pts) < (this.target ?? 0) - this.late) { this.keepNearest(f); continue; }
              this.insert(f);
            }
          }
        }
        if (gen === this.gen && this.cursor >= this.track.count && !this.decodedAll) {
          const tail = await this.decoder.decode(null, false, 0); // flush
          if (gen === this.gen) { for (const f of tail || []) this.insert(f); this.decodedAll = true; }
        }
      } catch (e) { this.errorText = e.message || "Décodage vidéo impossible."; }
      finally { this.filling = false; }
    }
    // After a seek the frame just before the target is the picture to show while the next one decodes.
    keepNearest(f) { if (!this.before || f.pts > this.before.pts) this.before = f; }
    insert(f) { let i = this.queue.length; while (i > 0 && this.queue[i - 1].pts > f.pts) i--; this.queue.splice(i, 0, f); }

    start() {
      if (this.running) return;
      this.running = true;
      const loop = () => { if (!this.running) return; this.tick(); this.handle = this.raf(loop); };
      this.handle = this.raf(loop);
    }
    stop() { this.running = false; if (this.handle) this.caf(this.handle); }

    tick() {
      const t = this.clock.time();
      let best = null;
      while (this.queue.length && this.seconds(this.queue[0].pts) <= t + 0.005) { if (best) this.stats.dropped++; best = this.queue.shift(); }
      if (!best && !this.drawn && this.before && this.seconds(this.before.pts) <= t + 0.005) { best = this.before; this.before = null; }
      if (best) this.draw(best);
      if (this.queue.length < this.queueMax / 2 && !this.filling && !this.decodedAll) this.fill();
    }
    draw(f) {
      if (f.width !== this.w || f.height !== this.h) { this.w = f.width; this.h = f.height; this.resize(f.width, f.height); this.image = this.makeImage(f.width, f.height); }
      if (f.rgba) this.image.data.set(f.rgba); else i420ToRgba(f, this.image.data);
      this.ctx.putImageData(this.image, 0, 0);
      this.drawn = f; this.stats.drawn++;
    }
    get finished() { return this.decodedAll && !this.queue.length; }
    close() { this.stop(); this.gen++; try { this.decoder.close(); } catch { /* ignore */ } }
  }

  const api = { SoftVideo, i420ToRgba, decoders };
  if (typeof module !== "undefined") module.exports = api; else root.ChillSoft = api;
})(typeof window !== "undefined" ? window : globalThis);
