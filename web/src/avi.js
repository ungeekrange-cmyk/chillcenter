"use strict";
// Zero-dependency AVI (RIFF) demuxer (ADR-0015/0016). Builds the same track tables as the MP4
// source, so seeking and fragment building are shared. Video that browsers cannot decode (Xvid,
// DivX) is exposed with `decoder` info and `mime: null` for a pluggable software decoder.
(function (root) {
  const Mp4 = typeof module !== "undefined" ? require("./mp4.js") : root.ChillMp4;
  const Mkv = typeof module !== "undefined" ? require("./mkv.js") : root.ChillMkv;
  const fail = message => Object.assign(new Error(message), { code: "PLAYER_ERROR" });
  const four = (b, o) => String.fromCharCode(b[o], b[o + 1], b[o + 2], b[o + 3]);
  const clean = s => s.replace(/[^\x20-\x7e]/g, "").trim().toLowerCase();
  const MAX_HEADER = 4 * 1024 * 1024;

  function parseHeader(b) {
    const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
    const streams = [];
    let cur = null, avih = null;
    const walk = (start, end) => {
      for (let p = start; p + 8 <= end;) {
        const id = four(b, p), size = dv.getUint32(p + 4, true), body = p + 8;
        if (id === "LIST") { const kind = four(b, body); if (kind === "strl") { cur = { odml: null }; streams.push(cur); } if (kind !== "movi") walk(body + 4, Math.min(end, body + size)); }
        else if (id === "avih") avih = { usPerFrame: dv.getUint32(body, true), width: dv.getUint32(body + 32, true), height: dv.getUint32(body + 36, true) };
        else if (id === "strh" && cur) Object.assign(cur, { type: four(b, body), handler: clean(four(b, body + 4)), scale: dv.getUint32(body + 20, true), rate: dv.getUint32(body + 24, true), length: dv.getUint32(body + 32, true), sampleSize: dv.getUint32(body + 44, true) });
        else if (id === "strf" && cur) {
          if (cur.type === "vids") Object.assign(cur, { width: dv.getInt32(body + 4, true), height: Math.abs(dv.getInt32(body + 8, true)), fourcc: clean(four(b, body + 16)) || cur.handler });
          else if (cur.type === "auds") Object.assign(cur, { format: dv.getUint16(body, true), channels: dv.getUint16(body + 2, true), sampleRate: dv.getUint32(body + 4, true), blockAlign: dv.getUint16(body + 12, true) });
        } else if (id === "indx" && cur) {
          // OpenDML super index: wLongsPerEntry(2) bIndexSubType(1) bIndexType(1) nEntriesInUse(4) dwChunkId(4) reserved(12)
          const n = dv.getUint32(body + 4, true); cur.odml = [];
          for (let i = 0; i < n; i++) { const e = body + 24 + i * 16; cur.odml.push({ offset: Number(dv.getBigUint64(e, true)), size: dv.getUint32(e + 8, true) }); }
        }
        if (id === "movi") return;
        p = body + size + (size & 1);
      }
    };
    walk(12, b.length);
    return { streams, avih };
  }

  /** idx1 entries -> [{stream, kind, offset (data start), size, key}] after offset-base detection. */
  async function readIdx1(read, idxPos, idxSize, moviTag, size) {
    const raw = await read(idxPos + 8, idxPos + 7 + idxSize);
    const dv = new DataView(raw.buffer, raw.byteOffset, raw.byteLength), n = Math.floor(raw.length / 16);
    if (!n) return [];
    // Offsets are normally relative to the 'movi' tag; some muxers write absolute offsets.
    const first = dv.getUint32(8, true), id0 = four(raw, 0);
    let base = moviTag;
    const probe = async b => { try { return four(await read(b + first, b + first + 3), 0) === id0; } catch { return false; } };
    if (!(await probe(moviTag))) base = (await probe(0)) ? 0 : moviTag;
    const out = [];
    for (let i = 0; i < n; i++) {
      const o = i * 16, id = four(raw, o), flags = dv.getUint32(o + 4, true), off = dv.getUint32(o + 8, true), len = dv.getUint32(o + 12, true);
      const m = /^(\d\d)(dc|db|wb)$/.exec(id);
      if (m && base + off + 8 + len <= size) out.push({ stream: Number(m[1]), kind: m[2], offset: base + off + 8, size: len, key: Boolean(flags & 0x10) });
    }
    return out;
  }

  async function readOdml(read, streams, size) {
    const per = streams.map(() => []);
    for (let s = 0; s < streams.length; s++) {
      for (const sup of streams[s].odml || []) {
        if (sup.offset + sup.size > size) continue;
        if (sup.size < 32) continue;
        const b = await read(sup.offset + 8, sup.offset + sup.size - 1); // body of the 'ix##' chunk (size includes its 8-byte header)
        const dv = new DataView(b.buffer, b.byteOffset, b.byteLength), n = dv.getUint32(4, true), base = Number(dv.getBigUint64(12, true));
        for (let i = 0; i < n; i++) {
          const e = 24 + i * 8, off = dv.getUint32(e, true), raw = dv.getUint32(e + 4, true), len = raw & 0x7fffffff;
          per[s].push({ stream: s, kind: streams[s].type === "auds" ? "wb" : "dc", offset: base + off, size: len, key: !(raw & 0x80000000) });
        }
      }
    }
    return per.flat();
  }

  class AviSource extends Mp4.Mp4Source {
    // `transcodeMp3` is chosen by the MSE engine before opening the AVI.  It
    // is deliberately false by default: browsers which accept MPEG audio in
    // MediaSource keep the direct, no-CPU route.
    constructor(reader, options = {}) {
      super(reader);
      this.audio = options.audio || null;
      this.transcodeMp3 = Boolean(options.transcodeMp3);
      this.transcoders = new Map();
      this.seq = 0; this.gen = 0; this.closed = false;
    }

    close() {
      this.closed = true; this.gen++;
      for (const state of this.transcoders.values()) {
        try { state.tc.close(); } catch { /* Worker already stopped. */ }
      }
      this.transcoders.clear();
    }
    async open() {
      const r = this.reader, read = (a, b) => r.read(a, b);
      const head = await read(0, Math.min(r.size, MAX_HEADER) - 1);
      if (four(head, 0) !== "RIFF" || four(head, 8) !== "AVI ") throw fail("Fichier AVI illisible.");
      const { streams } = parseHeader(head);
      if (!streams.length) throw fail("En-tête AVI illisible.");
      // Find 'movi' and what follows it.
      let moviTag = -1, moviEnd = 0;
      for (let p = 12; p + 12 <= r.size;) {
        const h = await read(p, p + 11), id = four(h, 0), len = new DataView(h.buffer, h.byteOffset).getUint32(4, true);
        if (id === "LIST" && four(h, 8) === "movi") { moviTag = p + 8; moviEnd = p + 8 + len + (len & 1); break; }
        p += 8 + len + (len & 1);
      }
      if (moviTag < 0) throw fail("Données AVI introuvables.");
      let entries = [];
      if (streams.some(s => s.odml && s.odml.length)) entries = await readOdml(read, streams, r.size);
      if (!entries.length && moviEnd + 8 <= r.size) {
        const h = await read(moviEnd, moviEnd + 7);
        if (four(h, 0) === "idx1") entries = await readIdx1(read, moviEnd, new DataView(h.buffer, h.byteOffset).getUint32(4, true), moviTag, r.size);
      }
      if (!entries.length) throw fail("Index AVI absent : lecture non prise en charge pour ce fichier.");

      const tracks = [];
      streams.forEach((s, index) => {
        const mine = entries.filter(e => e.stream === index && e.size > 0);
        if (s.type === "vids") tracks.push(this.videoTrack(s, index, entries.filter(e => e.stream === index), mine));
        else if (s.type === "auds") tracks.push(this.audioTrack(s, index, mine));
      });
      this.movie = { duration: Math.max(0, ...tracks.map(t => t.count ? (t.dts[t.count] || 0) / t.timescale : 0)), tracks: tracks.filter(Boolean) };
      for (const t of this.movie.tracks) this.cursor.set(t.id, 0);
      return this.movie;
    }

    finish(track, list, times) {
      const n = list.length, timescale = track.timescale;
      const dts = new Float64Array(n + 1), dur = new Uint32Array(n);
      for (let i = 0; i < n; i++) dts[i] = Math.round(times[i] * timescale);
      for (let i = 0; i + 1 < n; i++) dur[i] = Math.max(1, dts[i + 1] - dts[i]);
      if (n) { dur[n - 1] = n > 1 ? dur[n - 2] : Math.round(timescale / 25); dts[n] = dts[n - 1] + dur[n - 1]; }
      return Object.assign(track, {
        count: n, shift: 0, dts, dur, cts: new Int32Array(n),
        sizes: Uint32Array.from(list, e => e.size), offsets: Float64Array.from(list, e => e.offset), sync: Uint8Array.from(list, e => (e.key ? 1 : 0)),
      });
    }

    videoTrack(s, index, all, mine) {
      const frame = s.scale && s.rate ? s.scale / s.rate : 1 / 25;
      // A zero-length chunk repeats the previous frame: keep the time base by frame position.
      const times = []; const list = [];
      all.forEach((e, i) => { if (e.size > 0) { list.push(e); times.push(i * frame); } });
      const track = { id: index + 1, kind: "video", timescale: 90000, language: null, codec: null, mime: null, type: s.fourcc, width: s.width, height: s.height, ptsMode: "decode", decoder: { fourcc: s.fourcc, width: s.width, height: s.height }, name: null };
      return this.finish(track, list, times);
    }

    audioTrack(s, index, list) {
      // VBR (sampleSize 0): one sample per chunk; CBR: position follows cumulative bytes.
      const unit = s.scale / (s.rate || 1), times = []; let cum = 0;
      list.forEach((e, i) => { times.push(s.sampleSize ? (cum / s.sampleSize) * unit : i * unit); cum += e.size; });
      // AVI MP3 chunks are converted locally only when `transcodeMp3` was
      // selected from MediaSource capability detection.  Reuse the audited
      // AAC sample-entry builder for the declared fMP4 track, then retain the
      // real source codec for the worker and the user-facing label.
      const mp3Fallback = s.format === 0x55 && this.transcodeMp3;
      const base = mp3Fallback ? Mkv.buildTrack({ number: index + 1, type: 2, codecId: "A_MPEG/L3", transcode: "mp3", sourceLabel: "MP3", channels: 2, sampleRate: s.sampleRate, language: "und" })
        : s.format === 0x55 ? Mkv.buildTrack({ number: index + 1, type: 2, codecId: "A_MPEG/L3", channels: s.channels, sampleRate: s.sampleRate, language: "und" })
        : { id: index + 1, kind: "audio", timescale: s.sampleRate || 48000, codec: null, mime: null, type: "0x" + (s.format || 0).toString(16) };
      base.timescale = base.timescale || s.sampleRate || 48000;
      base.language = null;
      return this.finish(base, list, times);
    }

    /** Convert AVI MP3 chunks to AAC fMP4 fragments while retaining AVI's sample timeline. */
    async transcodeEntries(track, samples, flush = false, gen = this.gen) {
      let state = this.transcoders.get(track.id);
      if (!state) {
        if (!this.audio) throw fail(`Cette piste audio (${track.sourceLabel || track.type}) doit être convertie, mais le transcodeur audio n'est pas disponible.`);
        const tc = this.audio.create({ codec: track.transcode, rate: track.timescale, channels: 2, kbps: 192 });
        await tc.open();
        if (this.closed || gen !== this.gen) { tc.close(); return []; }
        state = { tc, anchor: null, count: 0, priming: tc.priming || 1024, flushed: false, partial: new Uint8Array() };
        this.transcoders.set(track.id, state);
      }
      if (!flush && state.anchor === null && samples.length) {
        state.anchor = Math.round(samples[0].dts);
        state.count = 0;
      }
      const input = flush ? [] : this.mp3Frames(state, samples.map(s => s.data));
      const packets = flush ? await state.tc.flush() : input.length ? await state.tc.convert(input) : [];
      if (this.closed || gen !== this.gen) return [];
      const entries = [];
      for (const data of packets) {
        const dts = state.anchor + (state.count - 1) * 1024;
        state.count += 1;
        if (dts < 0) continue; // AAC encoder pre-roll before the first AVI sample.
        entries.push({ data, dts, dur: 1024 });
      }
      return entries;
    }

    // AVI audio chunks need not align to MPEG frames. Keep at most one
    // partial tail, resynchronise safely, and send complete frames only to
    // FFmpeg; this avoids a device-dependent decoder/parser assumption.
    mp3Frames(state, chunks) {
      const bytes = Mp4.concat([state.partial, ...chunks]);
      const frames = []; let p = 0;
      while (p + 4 <= bytes.length) {
        const n = Mp4.mp3FrameLength(bytes, p);
        if (!n) { p++; continue; }
        if (p + n > bytes.length) break;
        frames.push(bytes.slice(p, p + n)); p += n;
      }
      state.partial = bytes.slice(p);
      return frames;
    }

    buildTranscodedFragment(track, entries) {
      const n = entries.length;
      const batch = {
        ...track,
        count: n,
        sizes: Uint32Array.from(entries, e => e.data.length),
        dur: Uint32Array.from(entries, e => e.dur),
        dts: Float64Array.from([...entries.map(e => e.dts), entries[n - 1].dts + entries[n - 1].dur]),
        cts: new Int32Array(n), sync: new Uint8Array(n).fill(1), offsets: Uint32Array.from(entries, (_, i) => i),
      };
      return Mp4.buildFragment(batch, 0, n, ++this.seq, offset => entries[offset].data);
    }

    async next(trackId) {
      const track = this.movie.tracks.find(t => t.id === trackId);
      if (!track?.transcode) return super.next(trackId);
      const gen = this.gen;
      let entries = [];
      // AAC encoding holds back one frame. Feed a bounded chunk loop so MSE
      // never receives an empty fragment merely because the encoder is warm.
      while (!entries.length && this.cursor.get(trackId) < track.count) {
        const from = this.cursor.get(trackId);
        const to = Math.min(track.count, from + 32);
        const raw = await this.rawRead(track, from, to - from);
        if (this.closed || gen !== this.gen) return null;
        this.cursor.set(trackId, to);
        entries = await this.transcodeEntries(track, raw, false, gen);
      }
      const state = this.transcoders.get(trackId);
      if (!entries.length && this.cursor.get(trackId) >= track.count && state && !state.flushed) {
        state.flushed = true;
        entries = await this.transcodeEntries(track, [], true, gen);
      }
      if (this.closed || gen !== this.gen) return null;
      return entries.length ? { data: this.buildTranscodedFragment(track, entries) } : null;
    }

    async seek(seconds) {
      const gen = ++this.gen;
      super.seek(seconds);
      for (const state of this.transcoders.values()) {
        state.anchor = null; state.count = 0; state.flushed = false; state.partial = new Uint8Array();
        await state.tc.reset();
        if (this.closed || gen !== this.gen) return;
      }
    }
  }

  const api = { AviSource, parseHeader };
  if (typeof module !== "undefined") module.exports = api; else root.ChillAvi = api;
})(typeof window !== "undefined" ? window : globalThis);
