"use strict";
// Zero-dependency MP4/MOV demuxer and fMP4 remuxer (ADR-0013). Reads sample tables from
// `moov`, then rebuilds per-track init segments and moof+mdat fragments for MediaSource.
// No decoding happens here: the browser decodes. Pure functions, no DOM.
(function (root) {
  const ascii = (b, o) => String.fromCharCode(b[o], b[o + 1], b[o + 2], b[o + 3]);
  const view = b => new DataView(b.buffer, b.byteOffset, b.byteLength);
  const fail = message => Object.assign(new Error(message), { code: "PLAYER_ERROR" });

  /** Child boxes of bytes[start,end): [{type, start, header, end}] (absolute offsets in bytes). */
  function boxes(bytes, start = 0, end = bytes.length) {
    const out = [], dv = view(bytes);
    let p = start;
    while (p + 8 <= end) {
      let size = dv.getUint32(p), header = 8;
      const type = ascii(bytes, p + 4);
      if (size === 1) { if (p + 16 > end) break; size = Number(dv.getBigUint64(p + 8)); header = 16; }
      else if (size === 0) size = end - p;
      if (size < header || p + size > end) break;
      out.push({ type, start: p, header, end: p + size });
      p += size;
    }
    return out;
  }
  const find = (bytes, parent, type) => boxes(bytes, parent.start + parent.header, parent.end).find(b => b.type === type);
  const findAll = (bytes, parent, type) => boxes(bytes, parent.start + parent.header, parent.end).filter(b => b.type === type);
  const payload = (bytes, box) => bytes.subarray(box.start + box.header, box.end);
  const raw = (bytes, box) => bytes.subarray(box.start, box.end);

  function concat(parts) {
    const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
    let o = 0; for (const p of parts) { out.set(p, o); o += p.length; }
    return out;
  }
  const u32b = n => Uint8Array.of(n >>> 24 & 255, n >>> 16 & 255, n >>> 8 & 255, n & 255);
  function box(type, ...parts) {
    const body = concat(parts);
    return concat([u32b(body.length + 8), new TextEncoder().encode(type), body]);
  }
  const full = (version, flags) => u32b(version << 24 | flags);

  // ---- codec strings -------------------------------------------------------------------
  const hex = n => n.toString(16);
  const pad2 = n => String(n).padStart(2, "0");
  function reverseBits32(n) { let r = 0; for (let i = 0; i < 32; i++) { r = (r << 1) | (n & 1); n >>>= 1; } return r >>> 0; }

  function readDescriptor(b, p) {
    const tag = b[p]; let len = 0, i = p + 1;
    for (let k = 0; k < 4; k++) { const c = b[i++]; len = (len << 7) | (c & 127); if (!(c & 128)) break; }
    return { tag, len, start: i };
  }
  function esdsCodec(b) {
    // ver/flags(4) | ES_Descr(0x03) { ES_ID(2) flags(1) ... DecoderConfig(0x04) { oti ... DSI(0x05) } }
    let d = readDescriptor(b, 4);
    if (d.tag !== 3) return null;
    const flags = b[d.start + 2]; let p = d.start + 3;
    if (flags & 0x80) p += 2;
    if (flags & 0x40) p += 1 + b[p];
    if (flags & 0x20) p += 2;
    d = readDescriptor(b, p);
    if (d.tag !== 4) return null;
    const oti = b[d.start];
    if (oti !== 0x40) return "mp4a." + hex(oti).toUpperCase();
    const dsi = readDescriptor(b, d.start + 13);
    return "mp4a.40." + (dsi.tag === 5 ? b[dsi.start] >> 3 : 2);
  }

  function sampleEntry(bytes, entry, kind) {
    const type = ascii(bytes, entry.start + 4);
    const dv = view(bytes);
    const body = entry.start + 8;
    let childStart;
    let info = { type };
    if (kind === "video") {
      info.width = dv.getUint16(body + 24); info.height = dv.getUint16(body + 26);
      childStart = body + 78;
    } else if (kind === "audio") {
      const v = dv.getUint16(body + 8);
      info.channels = dv.getUint16(body + 16);
      info.sampleRate = dv.getUint16(body + 24);
      childStart = body + 28 + (v === 1 ? 16 : v === 2 ? 36 : 0);
    } else return { type, codec: null };
    const kids = boxes(bytes, childStart, entry.end);
    const get = t => { const k = kids.find(x => x.type === t); return k && payload(bytes, k); };
    let codec = null;
    if (type === "avc1" || type === "avc3") { const c = get("avcC"); if (c) codec = type + "." + [c[1], c[2], c[3]].map(x => x.toString(16).padStart(2, "0")).join(""); }
    else if (type === "hvc1" || type === "hev1") {
      const c = get("hvcC");
      if (c) {
        const space = ["", "A", "B", "C"][c[1] >> 6], tier = c[1] & 32 ? "H" : "L";
        const compat = reverseBits32(view(c).getUint32(2));
        const cons = Array.from(c.subarray(6, 12)); while (cons.length > 1 && cons[cons.length - 1] === 0) cons.pop();
        codec = [type, space + (c[1] & 31), hex(compat), tier + c[12], ...cons.map(hex)].join(".").replace(/^(\w+)\./, "$1.");
      }
    } else if (type === "vp09") { const c = get("vpcC"); if (c) codec = `vp09.${pad2(c[4])}.${pad2(c[5])}.${pad2(c[6] >> 4)}`; }
    else if (type === "av01") { const c = get("av1C"); if (c) codec = `av01.${c[1] >> 5}.${pad2(c[1] & 31)}${c[2] & 128 ? "H" : "M"}.${pad2(c[2] & 64 ? (c[2] & 32 ? 12 : 10) : 8)}`; }
    else if (type === "mp4a") { const e = get("esds"); if (e) codec = esdsCodec(e); }
    else if (type === "Opus") codec = "opus";
    else if (type === "ac-3") codec = "ac-3";
    else if (type === "ec-3") codec = "ec-3";
    else if (type === "fLaC") codec = "flac";
    info.codec = codec;
    return info;
  }

  // ---- sample tables -------------------------------------------------------------------
  function parseTrack(bytes, trak, movieTimescale) {
    const dv = view(bytes);
    const tkhd = find(bytes, trak, "tkhd"), mdia = find(bytes, trak, "mdia");
    if (!tkhd || !mdia) return null;
    const mdhd = find(bytes, mdia, "mdhd"), hdlr = find(bytes, mdia, "hdlr"), minf = find(bytes, mdia, "minf");
    const stbl = minf && find(bytes, minf, "stbl");
    if (!mdhd || !hdlr || !stbl) return null;
    const tb = tkhd.start + tkhd.header, tv = bytes[tb];
    const id = dv.getUint32(tb + (tv === 1 ? 20 : 12));
    const mb = mdhd.start + mdhd.header, mv = bytes[mb];
    const timescale = dv.getUint32(mb + (mv === 1 ? 20 : 12));
    const duration = mv === 1 ? Number(dv.getBigUint64(mb + 24)) : dv.getUint32(mb + 16);
    const lang = dv.getUint16(mb + (mv === 1 ? 32 : 20));
    const language = lang ? String.fromCharCode(((lang >> 10) & 31) + 96, ((lang >> 5) & 31) + 96, (lang & 31) + 96) : null;
    const handler = ascii(bytes, hdlr.start + hdlr.header + 8);
    const kind = handler === "vide" ? "video" : handler === "soun" ? "audio" : handler === "subt" || handler === "text" ? "subtitle" : "other";

    const tbl = t => { const b = find(bytes, stbl, t); return b && { b, p: b.start + b.header }; };
    const stsd = tbl("stsd"), stts = tbl("stts"), stsc = tbl("stsc"), stsz = tbl("stsz"), stco = tbl("stco") || tbl("co64"), ctts = tbl("ctts"), stss = tbl("stss");
    if (!stsd || !stts || !stsc || !stsz || !stco) throw fail("Table d'échantillons MP4 incomplète.");
    const entry = boxes(bytes, stsd.p + 8, stsd.b.end)[0];
    const info = sampleEntry(bytes, entry, kind);

    const count = dv.getUint32(stsz.p + 8), fixed = dv.getUint32(stsz.p + 4);
    const sizes = new Uint32Array(count);
    for (let i = 0; i < count; i++) sizes[i] = fixed || dv.getUint32(stsz.p + 12 + 4 * i);

    const dts = new Float64Array(count + 1), dur = new Uint32Array(count);
    let n = 0, t = 0;
    for (let e = 0, entries = dv.getUint32(stts.p + 4); e < entries && n < count; e++) {
      const c = dv.getUint32(stts.p + 8 + 8 * e), d = dv.getUint32(stts.p + 12 + 8 * e);
      for (let k = 0; k < c && n < count; k++, n++) { dts[n] = t; dur[n] = d; t += d; }
    }
    dts[count] = t;

    const cts = new Int32Array(count);
    if (ctts) {
      let i = 0;
      for (let e = 0, entries = dv.getUint32(ctts.p + 4); e < entries && i < count; e++) {
        const c = dv.getUint32(ctts.p + 8 + 8 * e), o = dv.getInt32(ctts.p + 12 + 8 * e);
        for (let k = 0; k < c && i < count; k++) cts[i++] = o;
      }
    }
    const sync = new Uint8Array(count);
    if (stss) { for (let e = 0, entries = dv.getUint32(stss.p + 4); e < entries; e++) { const s = dv.getUint32(stss.p + 8 + 4 * e) - 1; if (s < count) sync[s] = 1; } }
    else sync.fill(1);

    const wide = stco.b.type === "co64", chunks = dv.getUint32(stco.p + 4);
    const chunkOffset = c => wide ? Number(dv.getBigUint64(stco.p + 8 + 8 * c)) : dv.getUint32(stco.p + 8 + 4 * c);
    const offsets = new Float64Array(count);
    const runs = dv.getUint32(stsc.p + 4);
    let s = 0;
    for (let r = 0; r < runs; r++) {
      const first = dv.getUint32(stsc.p + 8 + 12 * r) - 1, per = dv.getUint32(stsc.p + 12 + 12 * r);
      const last = r + 1 < runs ? dv.getUint32(stsc.p + 8 + 12 * (r + 1)) - 1 : chunks;
      for (let c = first; c < last; c++) { let o = chunkOffset(c); for (let k = 0; k < per && s < count; k++) { offsets[s] = o; o += sizes[s++]; } }
    }
    if (s !== count) throw fail("Table d'échantillons MP4 incohérente.");

    // Edit list: shift presentation so the first sample appears at media time 0 of the edit.
    let shift = 0;
    const edts = find(bytes, trak, "edts"), elst = edts && find(bytes, edts, "elst");
    if (elst) {
      const p = elst.start + elst.header, v = bytes[p], entries = dv.getUint32(p + 4);
      let delay = 0;
      for (let e = 0; e < entries; e++) {
        const q = p + 8 + e * (v === 1 ? 20 : 12);
        const segment = v === 1 ? Number(dv.getBigUint64(q)) : dv.getUint32(q);
        const mediaTime = v === 1 ? Number(dv.getBigInt64(q + 8)) : dv.getInt32(q + 4);
        if (mediaTime === -1) { delay += segment / movieTimescale; continue; }
        shift = delay - mediaTime / timescale; break;
      }
    }
    const minf2 = boxes(bytes, minf.start + minf.header, minf.end).filter(b => b.type !== "stbl");
    return {
      id, kind, timescale, duration, language, ...info, count, sizes, dts, dur, cts, sync, offsets, shift,
      supportedMime: null,
      mime: info.codec ? `${kind}/mp4; codecs="${info.codec}"` : null,
      raw: { tkhd: raw(bytes, tkhd), mdhd: raw(bytes, mdhd), hdlr: raw(bytes, hdlr), minf: minf2.map(b => raw(bytes, b)), stsd: raw(bytes, stsd.b) },
    };
  }

  /** @param moov Uint8Array containing the whole `moov` box (header included). */
  function parseMoov(moov) {
    const top = boxes(moov)[0];
    if (!top || top.type !== "moov") throw fail("Boîte moov absente.");
    const mvhd = find(moov, top, "mvhd");
    const dv = view(moov), mp = mvhd.start + mvhd.header, mv = moov[mp];
    const timescale = dv.getUint32(mp + (mv === 1 ? 20 : 12));
    const duration = mv === 1 ? Number(dv.getBigUint64(mp + 24)) : dv.getUint32(mp + 16);
    const tracks = findAll(moov, top, "trak").map(t => parseTrack(moov, t, timescale)).filter(Boolean);
    return { timescale, duration: duration / timescale, tracks };
  }

  // ---- fragments -----------------------------------------------------------------------
  function initSegment(track) {
    const empty = t => box(t, full(0, 0), u32b(0));
    const mvhd = box("mvhd", full(0, 0), u32b(0), u32b(0), u32b(1000), u32b(0), u32b(0x00010000), Uint8Array.of(1, 0, 0, 0), new Uint8Array(8),
      u32b(0x00010000), new Uint8Array(12), u32b(0), u32b(0x00010000), new Uint8Array(12), u32b(0), u32b(0x40000000), new Uint8Array(24), u32b(track.id + 1));
    const stbl = box("stbl", track.raw.stsd, empty("stts"), empty("stsc"), box("stsz", full(0, 0), u32b(0), u32b(0)), empty("stco"));
    const trak = box("trak", track.raw.tkhd, box("mdia", track.raw.mdhd, track.raw.hdlr, box("minf", ...track.raw.minf, stbl)));
    const mvex = box("mvex", box("trex", full(0, 0), u32b(track.id), u32b(1), u32b(0), u32b(0), u32b(0)));
    return concat([box("ftyp", new TextEncoder().encode("iso5"), u32b(512), new TextEncoder().encode("iso5iso6mp41")), box("moov", mvhd, trak, mvex)]);
  }

  /** Sample ranges for one fragment starting at `from`: ~`seconds` of media, at most `maxBytes`. */
  function planFragment(track, from, seconds = 2, maxBytes = 8 * 1024 * 1024) {
    let to = from, bytes = 0;
    const limit = track.dts[from] + seconds * track.timescale;
    while (to < track.count && (to === from || (track.dts[to] < limit && bytes + track.sizes[to] <= maxBytes))) bytes += track.sizes[to++];
    return { from, to, bytes };
  }

  /** Byte ranges [start,end] (inclusive) covering the samples, merging small gaps. */
  function sampleRanges(track, from, to, maxGap = 512 * 1024) {
    const ranges = [];
    for (let i = from; i < to; i++) {
      const start = track.offsets[i], end = start + track.sizes[i] - 1, last = ranges[ranges.length - 1];
      if (last && start >= last[0] && start - last[1] - 1 <= maxGap) last[1] = Math.max(last[1], end);
      else ranges.push([start, end]);
    }
    return ranges;
  }

  /** @param data function (absoluteOffset, size) -> Uint8Array of that sample's bytes. */
  function buildFragment(track, from, to, sequence, data) {
    // Fixed layout: moof{mfhd(16) traf{tfhd(16) tfdt(20) trun(20+16n)}} mdat
    const n = to - from, trunSize = 20 + 16 * n, moofSize = 8 + 16 + 8 + 16 + 20 + trunSize;
    let payloadBytes = 0; for (let i = from; i < to; i++) payloadBytes += track.sizes[i];
    const out = new Uint8Array(moofSize + 8 + payloadBytes), dv = view(out);
    const head = (p, size, type) => { dv.setUint32(p, size); out.set(new TextEncoder().encode(type), p + 4); };
    head(0, moofSize, "moof");
    head(8, 16, "mfhd"); dv.setUint32(20, sequence);
    head(24, 8 + 16 + 20 + trunSize, "traf");
    head(32, 16, "tfhd"); dv.setUint32(40, 0x020000); dv.setUint32(44, track.id); // default-base-is-moof
    head(48, 20, "tfdt"); dv.setUint32(56, 0x01000000);
    const base = Math.round(track.dts[from]);
    dv.setUint32(60, Math.floor(base / 4294967296)); dv.setUint32(64, base >>> 0);
    head(68, trunSize, "trun"); dv.setUint32(76, 0x01000F01); dv.setUint32(80, n); dv.setUint32(84, moofSize + 8);
    let p = 88, d = moofSize + 8;
    for (let i = from; i < to; i++) {
      dv.setUint32(p, track.dur[i]); dv.setUint32(p + 4, track.sizes[i]);
      dv.setUint32(p + 8, track.sync[i] ? 0x02000000 : 0x01010000); dv.setInt32(p + 12, track.cts[i]);
      p += 16;
      out.set(data(track.offsets[i], track.sizes[i]), d); d += track.sizes[i];
    }
    head(moofSize, 8 + payloadBytes, "mdat");
    return out;
  }

  /** Index of the sample to start from for `seconds`: video snaps back to a sync sample. */
  function sampleAtTime(track, seconds) {
    const target = (seconds - track.shift) * track.timescale;
    let lo = 0, hi = track.count - 1;
    while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (track.dts[mid] + track.cts[mid] <= target) lo = mid; else hi = mid - 1; }
    if (track.kind === "video") while (lo > 0 && !track.sync[lo]) lo--;
    return lo;
  }

  /** Top-level box layout from a reader: finds moov without loading mdat. */
  async function locateMoov(read, size, maxMoov = 64 * 1024 * 1024) {
    let p = 0;
    while (p + 8 <= size) {
      const head = await read(p, Math.min(p + 15, size - 1));
      const dv = view(head);
      let len = dv.getUint32(0), header = 8;
      const type = ascii(head, 4);
      if (len === 1) { len = Number(dv.getBigUint64(8)); header = 16; } else if (len === 0) len = size - p;
      if (len < header) throw fail("Fichier MP4 illisible.");
      if (type === "moov") { if (len > maxMoov) throw fail("Index MP4 trop volumineux."); return read(p, p + len - 1); }
      p += len;
    }
    throw fail("Index MP4 (moov) introuvable.");
  }

  /**
   * Common source interface used by the engine (also implemented by mkv.js):
   *   open() -> {duration, tracks}   seek(seconds)   next(trackId) -> {data, end} | null
   * `tracks[i]` carries kind/id/mime/language/codec/shift and what initSegment() needs.
   */
  class Mp4Source {
    constructor(reader) { this.reader = reader; this.cursor = new Map(); }
    async open() {
      const moov = await locateMoov((a, b) => this.reader.read(a, b), this.reader.size);
      this.movie = parseMoov(moov);
      for (const t of this.movie.tracks) this.cursor.set(t.id, 0);
      return this.movie;
    }
    seek(seconds) { for (const t of this.movie.tracks) this.cursor.set(t.id, sampleAtTime(t, seconds)); }
    /** Raw samples [{data, key, dts, cts, dur}] from index `from` (for software decoders). */
    async rawRead(track, from, count) {
      const to = Math.min(track.count, from + count), chunks = [];
      for (const [a, b] of sampleRanges(track, from, to)) chunks.push({ a, bytes: await this.reader.read(a, b) });
      const out = [];
      for (let i = from; i < to; i++) {
        const o = track.offsets[i], n = track.sizes[i], c = chunks.find(x => o >= x.a && o + n <= x.a + x.bytes.length);
        if (!c) throw fail("Échantillon hors plage lue.");
        out.push({ data: c.bytes.subarray(o - c.a, o - c.a + n), key: Boolean(track.sync[i]), dts: track.dts[i], cts: track.cts[i], dur: track.dur[i] });
      }
      return out;
    }
    async next(trackId) {
      const track = this.movie.tracks.find(t => t.id === trackId);
      const from = this.cursor.get(trackId);
      if (from >= track.count) return null;
      const plan = planFragment(track, from, 2);
      const chunks = [];
      for (const [a, b] of sampleRanges(track, plan.from, plan.to)) chunks.push({ a, bytes: await this.reader.read(a, b) });
      const data = (off, size) => { for (const c of chunks) if (off >= c.a && off + size <= c.a + c.bytes.length) return c.bytes.subarray(off - c.a, off - c.a + size); throw fail("Échantillon vidéo hors plage lue."); };
      this.cursor.set(trackId, plan.to);
      if (track.rawStream) { // e.g. MP3: appended as a bare byte stream, timed by the engine
        const parts = []; for (let i = plan.from; i < plan.to; i++) parts.push(data(track.offsets[i], track.sizes[i]));
        return { data: concat(parts), startTime: track.dts[plan.from] / track.timescale + (track.shift || 0) };
      }
      return { data: buildFragment(track, plan.from, plan.to, (this.seq = (this.seq || 0) + 1), data) };
    }
  }

  // ---- MPEG audio layer III -----------------------------------------------------------------
  const MP3_BR1 = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320], MP3_BR2 = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
  const MP3_SR = { 3: [44100, 48000, 32000], 2: [22050, 24000, 16000], 0: [11025, 12000, 8000] };
  /** Length in bytes of the layer III frame whose header starts at i, or 0 if there is no valid header. */
  function mp3FrameLength(b, i) {
    if (i + 4 > b.length || b[i] !== 0xff || (b[i + 1] & 0xe0) !== 0xe0) return 0;
    const version = (b[i + 1] >> 3) & 3, layer = (b[i + 1] >> 1) & 3, bi = b[i + 2] >> 4, si = (b[i + 2] >> 2) & 3;
    if (version === 1 || layer !== 1 || bi === 0 || bi === 15 || si === 3) return 0;
    const br = (version === 3 ? MP3_BR1 : MP3_BR2)[bi] * 1000, sr = MP3_SR[version][si], pad = (b[i + 2] >> 1) & 1;
    return Math.floor((version === 3 ? 144 : 72) * br / sr) + pad;
  }
  /** Drops leading bytes up to the first frame whose successor is also valid (AVI chunks are not frame aligned). */
  function mp3Resync(data, scan = 8192) {
    for (let i = 0; i < Math.min(data.length - 4, scan); i++) {
      const len = mp3FrameLength(data, i);
      if (len && (i + len + 4 > data.length || mp3FrameLength(data, i + len))) return i === 0 ? data : data.subarray(i);
    }
    return data;
  }

  const api = { mp3Resync, mp3FrameLength, boxes, parseMoov, initSegment, planFragment, sampleRanges, buildFragment, sampleAtTime, locateMoov, concat, box, full, u32b, sampleEntry, Mp4Source };
  if (typeof module !== "undefined") module.exports = api; else root.ChillMp4 = api;
})(typeof window !== "undefined" ? window : globalThis);
