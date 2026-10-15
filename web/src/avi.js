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
      const base = s.format === 0x55 ? Mkv.buildTrack({ number: index + 1, type: 2, codecId: "A_MPEG/L3", channels: s.channels, sampleRate: s.sampleRate, language: "und" })
        : { id: index + 1, kind: "audio", timescale: s.sampleRate || 48000, codec: null, mime: null, type: "0x" + (s.format || 0).toString(16) };
      base.timescale = base.timescale || s.sampleRate || 48000;
      base.language = null;
      return this.finish(base, list, times);
    }
  }

  const api = { AviSource, parseHeader };
  if (typeof module !== "undefined") module.exports = api; else root.ChillAvi = api;
})(typeof window !== "undefined" ? window : globalThis);
