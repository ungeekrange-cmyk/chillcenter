"use strict";
// Zero-dependency Matroska/WebM demuxer + fMP4 remuxer (ADR-0013). Unlike MP4 there is no
// global sample table: clusters are parsed lazily while playing, and the Cues index (or a
// header-only cluster hop) drives seeking. No decoding: the browser decodes.
(function (root) {
  const Mp4 = typeof module !== "undefined" ? require("./mp4.js") : root.ChillMp4;
  const { box, full, u32b, concat } = Mp4;
  const fail = message => Object.assign(new Error(message), { code: "PLAYER_ERROR" });
  const enc = s => new TextEncoder().encode(s);
  const u16b = n => Uint8Array.of(n >> 8 & 255, n & 255);
  const zeros = n => new Uint8Array(n);

  const ID = {
    EBML: 0x1a45dfa3, Segment: 0x18538067, SeekHead: 0x114d9b74, Seek: 0x4dbb, SeekID: 0x53ab, SeekPosition: 0x53ac,
    Info: 0x1549a966, TimecodeScale: 0x2ad7b1, Duration: 0x4489, Tracks: 0x1654ae6b, TrackEntry: 0xae,
    TrackNumber: 0xd7, TrackType: 0x83, CodecID: 0x86, CodecPrivate: 0x63a2, Language: 0x22b59c, LanguageBCP47: 0x22b59d,
    DefaultDuration: 0x23e383, Name: 0x536e, FlagDefault: 0x88, FlagEnabled: 0xb9, Video: 0xe0, PixelWidth: 0xb0, PixelHeight: 0xba,
    Audio: 0xe1, SamplingFrequency: 0xb5, Channels: 0x9f, ContentEncodings: 0x6d80, CodecDelay: 0x56aa,
    Cues: 0x1c53bb6b, CuePoint: 0xbb, CueTime: 0xb3, CueTrackPositions: 0xb7, CueTrack: 0xf7, CueClusterPosition: 0xf1,
    Cluster: 0x1f43b675, Timecode: 0xe7, SimpleBlock: 0xa3, BlockGroup: 0xa0, Block: 0xa1, BlockDuration: 0x9b, ReferenceBlock: 0xfb,
  };
  const LANG2 = { fre: "fr", fra: "fr", ger: "de", deu: "de", spa: "es", ita: "it", jpn: "ja", kor: "ko", chi: "zh", zho: "zh", dut: "nl", nld: "nl", por: "pt", rus: "ru", ara: "ar", pol: "pl", tur: "tr", swe: "sv", dan: "da", nor: "no", fin: "fi", hin: "hi", cze: "cs", ces: "cs", gre: "el", ell: "el", heb: "he", hun: "hu", eng: "en" };

  // ---- EBML primitives -----------------------------------------------------------------
  function readId(b, p) {
    const first = b[p]; let len = 1;
    while (len <= 4 && !(first & (0x80 >> (len - 1)))) len++;
    if (len > 4) throw fail("Fichier MKV illisible (identifiant).");
    let id = 0; for (let i = 0; i < len; i++) id = id * 256 + b[p + i];
    return { id, len };
  }
  function readVint(b, p, keepMarker) {
    const first = b[p]; let len = 1;
    while (len <= 8 && !(first & (0x80 >> (len - 1)))) len++;
    if (len > 8) throw fail("Fichier MKV illisible (taille).");
    let value = keepMarker ? first : first & (0xff >> len), allOnes = (first & (0xff >> len)) === (0xff >> len);
    for (let i = 1; i < len; i++) { value = value * 256 + b[p + i]; if (b[p + i] !== 0xff) allOnes = false; }
    return { value, len, unknown: allOnes };
  }
  /** Child elements of b[start,end): [{id, start (data), end}]; stops at a truncated element. */
  function children(b, start = 0, end = b.length) {
    const out = [];
    for (let p = start; p < end;) {
      let id, size;
      try { id = readId(b, p); size = readVint(b, p + id.len); } catch { break; }
      const s = p + id.len + size.len, e = size.unknown ? end : s + size.value;
      if (e > end) break;
      out.push({ id: id.id, start: s, end: e });
      p = e;
    }
    return out;
  }
  const uint = (b, el) => { let v = 0; for (let i = el.start; i < el.end; i++) v = v * 256 + b[i]; return v; };
  const float = (b, el) => { const dv = new DataView(b.buffer, b.byteOffset + el.start, el.end - el.start); return el.end - el.start === 4 ? dv.getFloat32(0) : dv.getFloat64(0); };
  const str = (b, el) => String.fromCharCode(...b.subarray(el.start, el.end)).replace(/\0+$/, "");

  // ---- sample entries --------------------------------------------------------------------
  function desc(tag, ...parts) {
    const body = concat(parts); const len = [];
    let n = body.length; len.unshift(n & 127); while ((n >>= 7)) len.unshift((n & 127) | 128);
    return concat([Uint8Array.of(tag), Uint8Array.from(len), body]);
  }
  function esds(oti, streamType, dsi) {
    const dec = desc(0x04, Uint8Array.of(oti, streamType << 2 | 1, 0, 0, 0), zeros(8), dsi ? desc(0x05, dsi) : zeros(0));
    return box("esds", full(0, 0), desc(0x03, u16b(0), Uint8Array.of(0), dec, desc(0x06, Uint8Array.of(2))));
  }
  const visual = (type, w, h, cfg) => box(type, zeros(6), u16b(1), zeros(16), u16b(w), u16b(h), u32b(0x00480000), u32b(0x00480000), u32b(0), u16b(1), zeros(32), u16b(0x18), u16b(0xffff), cfg);
  const audio = (type, ch, rate, cfg) => box(type, zeros(6), u16b(1), zeros(8), u16b(ch), u16b(16), u16b(0), u16b(0), u32b(Math.min(rate, 65535) * 65536), cfg);

  /** Matroska track -> MP4 sample entry bytes, or null when this build cannot remux the codec. */
  function sampleEntryFor(t) {
    const cp = t.codecPrivate;
    switch (t.codecId) {
      case "V_MPEG4/ISO/AVC": return cp && visual("avc1", t.width, t.height, box("avcC", cp));
      case "V_MPEGH/ISO/HEVC": return cp && visual("hvc1", t.width, t.height, box("hvcC", cp));
      case "V_AV1": return cp && visual("av01", t.width, t.height, box("av1C", cp));
      case "V_VP9": return visual("vp09", t.width, t.height, box("vpcC", full(1, 0), Uint8Array.of(0, 10, 0x82, 1, 2, 2), zeros(0), u16b(0)));
      case "A_AAC": return cp && audio("mp4a", t.channels, t.sampleRate, esds(0x40, 5, cp));
      case "A_MPEG/L3": return audio("mp4a", t.channels, t.sampleRate, esds(0x6b, 5, null));
      case "A_OPUS": {
        if (!cp || cp.length < 19) return null;
        const dv = new DataView(cp.buffer, cp.byteOffset, cp.byteLength);
        const dops = concat([Uint8Array.of(0, cp[9]), u16b(dv.getUint16(10, true)), u32b(dv.getUint32(12, true)), u16b(dv.getUint16(16, true)), Uint8Array.of(cp[18]), cp.subarray(19)]);
        return audio("Opus", t.channels, 48000, box("dOps", dops));
      }
      default: return null;
    }
  }

  function buildTrack(t) {
    const kind = t.type === 1 ? "video" : t.type === 2 ? "audio" : "other";
    if (kind === "other") return null;
    const timescale = kind === "video" ? 90000 : Math.round(t.sampleRate) || 48000;
    const entry = sampleEntryFor(t);
    const lang = (t.language || "und").padEnd(3, "d").slice(0, 3);
    const packed = ((lang.charCodeAt(0) - 96) << 10) | ((lang.charCodeAt(1) - 96) << 5) | (lang.charCodeAt(2) - 96);
    const base = { id: t.number, kind, timescale, shift: 0, language: LANG2[t.language] || (t.bcp47 ? t.bcp47.slice(0, 2) : t.language) || null, name: t.name, width: t.width, height: t.height, codecId: t.codecId, defaultDuration: t.defaultDuration, flagDefault: t.flagDefault };
    if (!entry) return { ...base, codec: null, mime: null, type: t.codecId };
    const stsdBox = box("stsd", full(0, 0), u32b(1), entry);
    const parsed = Mp4.sampleEntry(stsdBox, Mp4.boxes(stsdBox, 16)[0], kind);
    return {
      ...base, codec: parsed.codec, type: parsed.type,
      mime: parsed.codec ? `${kind}/mp4; codecs="${parsed.codec}"` : null,
      raw: {
        tkhd: box("tkhd", full(0, 3), u32b(0), u32b(0), u32b(t.number), u32b(0), u32b(0), zeros(8), u16b(0), u16b(0), u16b(kind === "audio" ? 0x100 : 0), u16b(0),
          u32b(0x10000), zeros(12), u32b(0x10000), zeros(12), u32b(0x40000000), u32b((t.width || 0) * 65536), u32b((t.height || 0) * 65536)),
        mdhd: box("mdhd", full(0, 0), u32b(0), u32b(0), u32b(timescale), u32b(0), u16b(packed), u16b(0)),
        hdlr: box("hdlr", full(0, 0), u32b(0), enc(kind === "video" ? "vide" : "soun"), zeros(12), zeros(1)),
        minf: [kind === "video" ? box("vmhd", full(0, 1), zeros(8)) : box("smhd", full(0, 0), zeros(4)), box("dinf", box("dref", full(0, 0), u32b(1), box("url ", full(0, 1))))],
        stsd: stsdBox,
      },
    };
  }

  function parseTracks(b, tracksEl) {
    const out = [];
    for (const te of children(b, tracksEl.start, tracksEl.end).filter(e => e.id === ID.TrackEntry)) {
      const t = { sampleRate: 8000, channels: 1, flagDefault: 1 };
      for (const c of children(b, te.start, te.end)) {
        switch (c.id) {
          case ID.TrackNumber: t.number = uint(b, c); break;
          case ID.TrackType: t.type = uint(b, c); break;
          case ID.CodecID: t.codecId = str(b, c); break;
          case ID.CodecPrivate: t.codecPrivate = b.subarray(c.start, c.end); break;
          case ID.Language: t.language = str(b, c); break;
          case ID.LanguageBCP47: t.bcp47 = str(b, c); break;
          case ID.Name: t.name = str(b, c); break;
          case ID.DefaultDuration: t.defaultDuration = uint(b, c); break;
          case ID.FlagDefault: t.flagDefault = uint(b, c); break;
          case ID.ContentEncodings: t.encoded = true; break;
          case ID.Video: for (const v of children(b, c.start, c.end)) { if (v.id === ID.PixelWidth) t.width = uint(b, v); if (v.id === ID.PixelHeight) t.height = uint(b, v); } break;
          case ID.Audio: for (const a of children(b, c.start, c.end)) { if (a.id === ID.SamplingFrequency) t.sampleRate = float(b, a); if (a.id === ID.Channels) t.channels = uint(b, a); } break;
          default: break;
        }
      }
      if (t.number === undefined) continue;
      if (t.encoded && (t.type === 1 || t.type === 2)) { out.push({ id: t.number, kind: t.type === 1 ? "video" : "audio", codec: null, mime: null, type: t.codecId, unsupportedReason: "compression de pistes MKV" }); continue; }
      const track = buildTrack(t);
      if (track) out.push(track);
    }
    return out;
  }

  // ---- blocks and clusters ---------------------------------------------------------------
  /** Splits a (Simple)Block body into frames; handles Xiph, fixed-size and EBML lacing. */
  function blockFrames(b, start, end) {
    const tn = readVint(b, start);
    let p = start + tn.len;
    const rel = (b[p] << 8 | b[p + 1]) << 16 >> 16, flags = b[p + 2]; p += 3;
    const lacing = (flags >> 1) & 3;
    const frames = [];
    if (lacing === 0) frames.push(b.subarray(p, end));
    else {
      const n = b[p++] + 1, sizes = [];
      if (lacing === 1) { for (let i = 0; i < n - 1; i++) { let s = 0, c; do { c = b[p++]; s += c; } while (c === 255); sizes.push(s); } }
      else if (lacing === 3) {
        let v = readVint(b, p); p += v.len; let prev = v.value; sizes.push(prev);
        for (let i = 1; i < n - 1; i++) { v = readVint(b, p); p += v.len; prev += v.value - (Math.pow(2, 7 * v.len - 1) - 1); sizes.push(prev); }
      } else { const each = Math.floor((end - p) / n); for (let i = 0; i < n - 1; i++) sizes.push(each); }
      sizes.push(end - p - sizes.reduce((a, c) => a + c, 0));
      for (const s of sizes) { if (s < 0 || p + s > end) throw fail("Bloc MKV avec laçage invalide."); frames.push(b.subarray(p, p + s)); p += s; }
    }
    return { track: tn.value, rel, key: Boolean(flags & 0x80), frames };
  }

  /** Samples of one cluster: [{track, ms, key, data, durationMs}] in file order. */
  function parseCluster(b, start, end, scaleMs, defaults) {
    let base = 0; const out = [];
    for (const c of children(b, start, end)) {
      if (c.id === ID.Timecode) base = uint(b, c);
      else if (c.id === ID.SimpleBlock || c.id === ID.BlockGroup) {
        let body = c, key, durationMs = 0, hasRef = false;
        if (c.id === ID.BlockGroup) {
          for (const g of children(b, c.start, c.end)) {
            if (g.id === ID.Block) body = g; else if (g.id === ID.BlockDuration) durationMs = uint(b, g) * scaleMs; else if (g.id === ID.ReferenceBlock) hasRef = true;
          }
          if (body === c) continue;
        }
        const blk = blockFrames(b, body.start, body.end);
        key = c.id === ID.SimpleBlock ? blk.key : !hasRef;
        const step = defaults.get(blk.track) || (durationMs && durationMs / blk.frames.length) || 0;
        blk.frames.forEach((data, i) => out.push({ track: blk.track, ms: (base + blk.rel) * scaleMs + i * step, key: key && i === 0, data, durationMs: durationMs ? durationMs / blk.frames.length : step }));
      }
    }
    return out;
  }

  /**
   * Streaming timing for one track. MKV stores only presentation times, so for video the decode
   * time of the j-th sample is the j-th smallest PTS seen (reorder window of REORDER samples), and
   * every sample's duration is the distance to the next decode time. Independent of cluster size.
   */
  const REORDER = 16;
  class Pipeline {
    constructor(track) { this.track = track; this.ts = track.timescale; this.video = track.kind === "video"; this.pool = []; this.fifo = []; this.held = null; this.out = []; }
    ticks(ms) { return Math.round(ms * this.ts / 1000); }
    settle(entry) {
      if (this.held) { this.held.dur = Math.max(1, entry.dts - this.held.dts); this.out.push(this.held); }
      this.held = entry;
    }
    feed(samples) {
      for (const s of samples) {
        const entry = { s, pts: this.ticks(s.ms), dts: 0, dur: 0 };
        if (!this.video) { entry.dts = entry.pts; this.settle(entry); continue; }
        let i = this.pool.length; while (i > 0 && this.pool[i - 1] > entry.pts) i--;
        this.pool.splice(i, 0, entry.pts); this.fifo.push(entry);
        if (this.pool.length >= REORDER) { const e = this.fifo.shift(); e.dts = this.pool.shift(); this.settle(e); }
      }
      const ready = this.out; this.out = []; return ready;
    }
    flush() {
      while (this.fifo.length) { const e = this.fifo.shift(); e.dts = this.pool.shift(); this.settle(e); }
      if (this.held) {
        const frameMs = (this.track.defaultDuration ? this.track.defaultDuration / 1e6 : 0) || this.held.s.durationMs || 1000 / 30;
        this.held.dur = Math.max(1, Math.round(frameMs * this.ts / 1000)); this.out.push(this.held); this.held = null;
      }
      const ready = this.out; this.out = []; return ready;
    }
  }
  function buildBatch(track, entries) {
    const n = entries.length;
    const sizes = new Uint32Array(n), dur = new Uint32Array(n), cts = new Int32Array(n), sync = new Uint8Array(n), offsets = new Float64Array(n), dts = new Float64Array(n + 1);
    entries.forEach((e, i) => { sizes[i] = e.s.data.length; dur[i] = e.dur; cts[i] = e.pts - e.dts; sync[i] = e.s.key || track.kind === "audio" ? 1 : 0; offsets[i] = i; dts[i] = e.dts; });
    return { id: track.id, count: n, sizes, dur, dts, cts, sync, offsets, samples: entries.map(e => e.s) };
  }

  class MkvSource {
    constructor(reader) { this.reader = reader; this.queues = new Map(); this.cursor = 0; this.gen = 0; this.pipes = new Map(); this.seq = 0; this.active = null; }
    setActive(ids) { this.active = new Set(ids); for (const id of this.queues.keys()) if (!this.active.has(id)) this.queues.set(id, []); }

    async read(a, b) { return this.reader.read(a, Math.min(b, this.reader.size - 1)); }
    async header(p) {
      const h = await this.read(p, p + 15), id = readId(h, 0), size = readVint(h, id.len);
      return { id: id.id, dataStart: p + id.len + size.len, size: size.unknown ? -1 : size.value };
    }

    async open() {
      const ebml = await this.header(0);
      if (ebml.id !== ID.EBML) throw fail("Fichier MKV illisible (en-tête).");
      const seg = await this.header(ebml.dataStart + ebml.size);
      if (seg.id !== ID.Segment) throw fail("Fichier MKV illisible (segment).");
      this.segStart = seg.dataStart;
      this.scaleMs = 1; let duration = 0, tracksEl = null, cuesPos = null, firstCluster = null;
      const wanted = new Map();
      for (let p = this.segStart; p < this.reader.size;) {
        const h = await this.header(p);
        const end = h.size < 0 ? this.reader.size : h.dataStart + h.size;
        if (h.id === ID.Cluster) { firstCluster = p; break; }
        if ([ID.Info, ID.Tracks, ID.SeekHead, ID.Cues].includes(h.id)) {
          if (end - h.dataStart > 32 * 1024 * 1024) throw fail("Index MKV trop volumineux.");
          wanted.set(h.id, { bytes: await this.read(h.dataStart, end - 1), pos: p });
        }
        p = end;
      }
      this.firstCluster = firstCluster;
      const info = wanted.get(ID.Info);
      if (info) {
        let scale = 1000000, dur = 0;
        for (const c of children(info.bytes)) { if (c.id === ID.TimecodeScale) scale = uint(info.bytes, c); if (c.id === ID.Duration) dur = float(info.bytes, c); }
        this.scaleMs = scale / 1e6; duration = dur * this.scaleMs / 1000;
      }
      const trk = wanted.get(ID.Tracks);
      if (!trk) throw fail("Pistes MKV introuvables.");
      this.tracks = parseTracks(trk.bytes, { start: 0, end: trk.bytes.length });
      this.defaults = new Map(this.tracks.filter(t => t.defaultDuration).map(t => [t.id, t.defaultDuration / 1e6]));
      let cues = wanted.get(ID.Cues);
      if (!cues) {
        const sh = wanted.get(ID.SeekHead);
        if (sh) for (const s of children(sh.bytes)) if (s.id === ID.Seek) {
          let sid = 0, pos = 0;
          for (const c of children(sh.bytes, s.start, s.end)) { if (c.id === ID.SeekID) sid = uint(sh.bytes, c); if (c.id === ID.SeekPosition) pos = uint(sh.bytes, c); }
          if (sid === ID.Cues) cuesPos = this.segStart + pos;
        }
        if (cuesPos !== null && cuesPos < this.reader.size) {
          const h = await this.header(cuesPos);
          if (h.id === ID.Cues && h.size > 0 && h.size < 32 * 1024 * 1024) cues = { bytes: await this.read(h.dataStart, h.dataStart + h.size - 1) };
        }
      }
      this.cues = cues ? this.parseCues(cues.bytes) : [];
      if (!duration && this.cues.length) duration = this.cues[this.cues.length - 1].ms / 1000 + 5;
      this.duration = duration || 0;
      this.cursor = firstCluster;
      for (const t of this.tracks) this.queues.set(t.id, []);
      return { duration: this.duration, tracks: this.tracks };
    }

    parseCues(b) {
      const points = [];
      for (const cp of children(b).filter(c => c.id === ID.CuePoint)) {
        let time = 0, pos = null;
        for (const c of children(b, cp.start, cp.end)) {
          if (c.id === ID.CueTime) time = uint(b, c);
          if (c.id === ID.CueTrackPositions) for (const q of children(b, c.start, c.end)) if (q.id === ID.CueClusterPosition && pos === null) pos = uint(b, q);
        }
        if (pos !== null) points.push({ ms: time * this.scaleMs, pos: this.segStart + pos });
      }
      return points.sort((a, c) => a.ms - c.ms);
    }

    /** Position of the cluster to start from for `seconds` (Cues, else a header-only hop). */
    async clusterFor(seconds) {
      const ms = seconds * 1000;
      if (this.cues.length) {
        let best = this.cues[0];
        for (const c of this.cues) { if (c.ms <= ms) best = c; else break; }
        return best.pos;
      }
      let p = this.firstCluster, last = p;
      while (p < this.reader.size) {
        const h = await this.header(p);
        if (h.id !== ID.Cluster) { p = h.dataStart + Math.max(0, h.size); continue; }
        const head = await this.read(h.dataStart, h.dataStart + 15);
        const t = children(head).find(c => c.id === ID.Timecode);
        if (t && uint(head, t) * this.scaleMs > ms) break;
        last = p;
        if (h.size < 0) break;
        p = h.dataStart + h.size;
      }
      return last;
    }

    seek(seconds) { this.pendingSeek = seconds; return this.doSeek(seconds); }
    async doSeek(seconds) {
      this.gen++;
      for (const id of this.queues.keys()) this.queues.set(id, []);
      this.pipes.clear();
      this.cursor = await this.clusterFor(seconds);
      this.needKey = new Set(this.tracks.filter(t => t.kind === "video").map(t => t.id));
      this.eof = false;
    }

    /** Parses the next cluster into per-track queues; false at end of file. */
    async ingest() {
      const gen = this.gen;
      for (; this.cursor < this.reader.size;) {
        const h = await this.header(this.cursor);
        const end = h.size < 0 ? this.reader.size : h.dataStart + h.size;
        if (h.id !== ID.Cluster) { this.cursor = end; continue; }
        if (end - h.dataStart > 128 * 1024 * 1024) throw fail("Segment MKV trop volumineux.");
        const bytes = await this.read(h.dataStart, end - 1);
        if (gen !== this.gen) return true; // a seek happened while reading: discard
        this.cursor = end;
        const samples = parseCluster(bytes, 0, bytes.length, this.scaleMs, this.defaults);
        const byTrack = new Map();
        for (const s of samples) { if (!this.queues.has(s.track) || (this.active && !this.active.has(s.track))) continue; (byTrack.get(s.track) || byTrack.set(s.track, []).get(s.track)).push(s); }
        for (const [id, list] of byTrack) {
          const track = this.tracks.find(t => t.id === id);
          if (!track.mime) continue;
          let usable = list;
          if (this.needKey?.has(id)) { const k = usable.findIndex(s => s.key); if (k < 0) continue; usable = usable.slice(k); this.needKey.delete(id); }
          if (!this.pipes.has(id)) this.pipes.set(id, new Pipeline(track));
          const ready = this.pipes.get(id).feed(usable);
          if (ready.length) this.queues.get(id).push(buildBatch(track, ready));
        }
        return true;
      }
      // EOF: flush what the reorder window and duration pairing still hold.
      let flushed = false;
      for (const [id, pipe] of this.pipes) {
        const ready = pipe.flush();
        if (ready.length) { this.queues.get(id).push(buildBatch(this.tracks.find(t => t.id === id), ready)); flushed = true; }
      }
      this.pipes.clear();
      this.eof = true;
      return flushed;
    }

    async next(trackId) {
      const track = this.tracks.find(t => t.id === trackId);
      const q = this.queues.get(trackId);
      while (!q.length) { if (this.eof) return null; const more = await this.ingest(); if (!more && !q.length) return null; }
      const batch = q.shift();
      const data = (off, size) => batch.samples[off].data;
      return { data: Mp4.buildFragment({ ...batch, id: track.id }, 0, batch.count, ++this.seq, data) };
    }
  }

  const api = { MkvSource, parseCluster, blockFrames, children, readVint, Pipeline, ID };
  if (typeof module !== "undefined") module.exports = api; else root.ChillMkv = api;
})(typeof window !== "undefined" ? window : globalThis);
