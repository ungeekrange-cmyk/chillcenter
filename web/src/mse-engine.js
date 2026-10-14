"use strict";
// MP4 playback through MediaSource (ADR-0013): bytes come from the leased Range reader, mp4.js
// remuxes them to fMP4 in the browser, the browser decodes. No dependency, no transcoding.
(function (root) {
  const Mp4 = typeof module !== "undefined" ? require("./mp4.js") : root.ChillMp4;
  const { RangeReader } = typeof module !== "undefined" ? require("./range-reader.js") : root.ChillRangeReader;
  const fail = message => Object.assign(new Error(message), { code: "PLAYER_ERROR" });
  const LABELS = { "ac-3": "AC-3", "ec-3": "E-AC-3", flac: "FLAC", opus: "Opus" };
  const codecLabel = c => LABELS[c] || (c || "inconnu").split(".")[0];

  function sniffContainer(head) {
    const tag = String.fromCharCode(...head.subarray(4, 8));
    if (tag === "ftyp" || tag === "moov" || tag === "mdat" || tag === "free" || tag === "wide") return "mp4";
    if (head[0] === 0x1a && head[1] === 0x45 && head[2] === 0xdf && head[3] === 0xa3) return "mkv";
    if (String.fromCharCode(...head.subarray(0, 4)) === "RIFF") return "avi";
    return "unknown";
  }

  class MseEngine {
    constructor(host, deps) { this.host = host; this.d = deps; this.streams = []; this.epoch = 0; this.closed = false; this.errorText = null; this.notice = null; this.rate = 1; this.hardware = "no"; }

    cacheSeconds() {
      let forward = 128, back = 64;
      try { const s = JSON.parse(this.d.storage.getItem("chill.settings") || "{}").playback_cache; if (s) { forward = s.forward_mib; back = s.back_mib; } } catch { /* defaults */ }
      const bps = Math.max(1, this.reader.size / Math.max(1, this.movie.duration));
      return { forward: Math.min(90, Math.max(8, forward * 1048576 / bps)), back: Math.min(60, back * 1048576 / bps) };
    }

    async open({ mediaId, lease, startAt, speed }) {
      const MS = this.d.MediaSource;
      if (!MS) throw fail("Ce navigateur ne sait pas lire de vidéo par MediaSource.");
      this.reader = new RangeReader({ mediaId, lease, fetch: this.d.fetch });
      await this.reader.open();
      const kind = sniffContainer(this.reader.head);
      if (kind === "mkv") throw fail("Format MKV : lecture dans le navigateur pas encore disponible dans cette version.");
      if (kind === "avi") throw fail("Format AVI : lecture dans le navigateur pas encore disponible dans cette version.");
      if (kind !== "mp4") throw fail("Format de fichier non reconnu.");
      const moov = await Mp4.locateMoov((a, b) => this.reader.read(a, b), this.reader.size);
      this.movie = Mp4.parseMoov(moov);
      const supported = t => t.mime && MS.isTypeSupported(t.mime);
      const video = this.movie.tracks.find(t => t.kind === "video" && supported(t));
      if (!video) {
        const v = this.movie.tracks.find(t => t.kind === "video");
        throw fail(v ? `Codec vidéo ${codecLabel(v.codec || v.type)} non pris en charge par ce navigateur.` : "Aucune piste vidéo.");
      }
      this.audio = this.movie.tracks.filter(t => t.kind === "audio" && supported(t));
      const missing = this.movie.tracks.filter(t => t.kind === "audio" && !supported(t));
      if (missing.length) this.notice = `Audio non pris en charge par ce navigateur : ${[...new Set(missing.map(t => codecLabel(t.codec || t.type)))].join(", ")}.`;
      if (!this.audio.length && missing.length) this.notice += " Lecture sans son.";
      this.activeAudio = this.audio[0] || null;

      this.video = this.d.document.createElement("video");
      this.video.playsInline = true; this.video.preload = "auto";
      this.host.replaceChildren(this.video);
      this.bind();
      this.ms = new MS();
      this.url = this.d.URL.createObjectURL(this.ms);
      this.video.src = this.url;
      await new Promise((res, rej) => { this.ms.addEventListener("sourceopen", res, { once: true }); setTimeout(() => rej(fail("MediaSource indisponible.")), 5000); });
      this.ms.duration = this.movie.duration;
      for (const t of [video, this.activeAudio].filter(Boolean)) {
        const sb = this.ms.addSourceBuffer(t.mime);
        sb.mode = "segments"; sb.timestampOffset = t.shift;
        this.streams.push({ track: t, sb, next: 0, seq: 1, done: false });
      }
      this.cache = this.cacheSeconds();
      for (const s of this.streams) await this.append(s.sb, Mp4.initSegment(s.track));
      this.video.playbackRate = speed || 1; this.rate = speed || 1;
      this.rewind(startAt || 0);
      this.video.currentTime = startAt || 0;
      this.probeHardware(video);
      await this.pump();
      this.started = true;
    }

    bind() {
      const v = this.video;
      v.addEventListener("error", () => { this.errorText = "Le navigateur n'a pas pu décoder cette vidéo."; });
      v.addEventListener("timeupdate", () => this.pump());
      v.addEventListener("seeking", () => { if (this.started && !this.isBuffered(v.currentTime)) this.reposition(v.currentTime); });
      v.addEventListener("waiting", () => this.pump());
    }
    async probeHardware(t) {
      try {
        const info = await this.d.navigator.mediaCapabilities.decodingInfo({ type: "media-source", video: { contentType: t.mime.replace("video/mp4; codecs=", "video/mp4;codecs="), width: t.width, height: t.height, bitrate: 4e6, framerate: 24 } });
        this.hardware = info.powerEfficient ? "navigateur" : "no";
      } catch { /* unknown stays software */ }
    }

    // ---- buffering ---------------------------------------------------------------------
    append(sb, data, retried) {
      return new Promise((resolve, reject) => {
        const off = () => { sb.removeEventListener("updateend", ok); sb.removeEventListener("error", bad); };
        const ok = () => { off(); resolve(); };
        const bad = () => { off(); reject(fail("Le navigateur a refusé un segment vidéo.")); };
        sb.addEventListener("updateend", ok); sb.addEventListener("error", bad);
        try { sb.appendBuffer(data); }
        catch (e) {
          off();
          if (e.name === "QuotaExceededError" && !retried) { this.evict(sb, 0).then(() => this.append(sb, data, true)).then(resolve, reject); }
          else reject(fail("Mémoire vidéo du navigateur insuffisante."));
        }
      });
    }
    removeRange(sb, a, b) {
      return new Promise(resolve => {
        if (b <= a || sb.updating) return resolve();
        const done = () => { sb.removeEventListener("updateend", done); resolve(); };
        sb.addEventListener("updateend", done);
        try { sb.remove(a, b); } catch { sb.removeEventListener("updateend", done); resolve(); }
      });
    }
    async evict(sb, keepBack) {
      const b = sb.buffered; if (!b.length) return;
      const limit = this.video.currentTime - keepBack;
      if (b.start(0) < limit) await this.removeRange(sb, 0, limit);
    }
    bufferedEnd(sb, t) {
      const b = sb.buffered;
      for (let i = 0; i < b.length; i++) if (b.start(i) <= t + 0.25 && b.end(i) >= t) return b.end(i);
      return t;
    }
    isBuffered(t) { return this.streams.every(s => this.bufferedEnd(s.sb, t) > t + 0.05); }
    rewind(t) { for (const s of this.streams) { s.next = Mp4.sampleAtTime(s.track, t); s.done = false; } }
    async reposition(t) {
      this.epoch++;
      await this.waitIdle();
      for (const s of this.streams) { if (s.sb.updating) { try { s.sb.abort(); } catch { /* ignore */ } } await this.removeRange(s.sb, 0, Infinity); }
      this.rewind(t);
      this.pumping = false;
      await this.pump();
    }
    waitIdle() { return this.pumping ? new Promise(r => setTimeout(r, 30)).then(() => this.waitIdle()) : Promise.resolve(); }

    async pump() {
      if (this.pumping || this.closed || !this.ms || this.ms.readyState === "closed") return;
      this.pumping = true;
      const epoch = this.epoch;
      try {
        for (let again = true; again && !this.closed && epoch === this.epoch;) {
          again = false;
          for (const s of this.streams) if (await this.feed(s, epoch)) again = true;
        }
        if (!this.closed && epoch === this.epoch && this.streams.every(s => s.done) && this.ms.readyState === "open" && !this.streams.some(s => s.sb.updating)) this.ms.endOfStream();
      } catch (e) { this.errorText = e.message || "Lecture impossible."; }
      finally { this.pumping = false; }
    }
    async feed(s, epoch) {
      const t = this.video.currentTime;
      if (this.bufferedEnd(s.sb, t) - t >= this.cache.forward) return false;
      if (s.next >= s.track.count) { s.done = true; return false; }
      await this.evict(s.sb, this.cache.back);
      const plan = Mp4.planFragment(s.track, s.next, 2);
      const chunks = [];
      for (const [a, b] of Mp4.sampleRanges(s.track, plan.from, plan.to)) chunks.push({ a, bytes: await this.reader.read(a, b) });
      if (epoch !== this.epoch || this.closed) return false;
      const data = (off, size) => { for (const c of chunks) if (off >= c.a && off + size <= c.a + c.bytes.length) return c.bytes.subarray(off - c.a, off - c.a + size); throw fail("Échantillon vidéo hors plage lue."); };
      await this.append(s.sb, Mp4.buildFragment(s.track, plan.from, plan.to, s.seq++, data));
      if (epoch !== this.epoch) return false;
      s.next = plan.to;
      return true;
    }

    // ---- Engine interface --------------------------------------------------------------
    play() { this.video?.play().catch(() => {}); }
    pause() { this.video?.pause(); }
    seek(seconds) { if (this.video) this.video.currentTime = Math.min(seconds, this.movie.duration); }
    setSpeed(x) { this.rate = x; if (this.video) this.video.playbackRate = x; }
    setVolume(v) { if (this.video) this.video.volume = Math.min(1, Math.max(0, v / 100)); }
    selectSubtitle() {}
    externalSubtitles() { return []; }
    async selectAudio(id) {
      const next = this.audio?.find(t => t.id === id), s = this.streams.find(x => x.track.kind === "audio");
      if (!next || !s || s.track === next) return;
      this.epoch++; await this.waitIdle();
      await this.removeRange(s.sb, 0, Infinity);
      if (s.track.mime !== next.mime) s.sb.changeType(next.mime);
      s.track = next; s.sb.timestampOffset = next.shift; this.activeAudio = next;
      await this.append(s.sb, Mp4.initSegment(next));
      s.next = Mp4.sampleAtTime(next, this.video.currentTime); s.done = false;
      await this.pump();
    }
    snapshot() {
      const v = this.video;
      if (!v) return { loaded: false, paused: true, buffering: false, ended: false, error: this.errorText, position: 0, duration: 0, speed: this.rate, volume: 100, tracks: [] };
      return {
        loaded: v.readyState >= 1, paused: v.paused, buffering: !v.paused && !v.ended && v.readyState < 3, ended: v.ended,
        error: this.errorText, notice: this.notice, position: v.currentTime || 0, duration: this.movie.duration, speed: v.playbackRate, volume: Math.round(v.volume * 100),
        tracks: (this.audio || []).map((t, i) => ({ id: t.id, kind: "Audio", language: t.language, title: codecLabel(t.codec), selected: t === this.activeAudio, original: i === 0 })),
        hardware_decoder: this.hardware, decoder: "Décodeur du navigateur",
      };
    }
    close() {
      this.closed = true; this.epoch++;
      try { this.video?.pause(); this.video?.removeAttribute("src"); this.video?.load(); } catch { /* ignore */ }
      if (this.url) this.d.URL.revokeObjectURL(this.url);
      this.host?.replaceChildren();
    }
  }

  const api = { MseEngine, sniffContainer };
  if (typeof module !== "undefined") module.exports = api;
  else if (root.MediaSource && root.ChillEngines) {
    root.ChillEngines.factories.push(host => new MseEngine(host, { fetch: (...a) => root.fetch(...a), MediaSource: root.MediaSource, document: root.document, URL: root.URL, storage: root.localStorage, navigator: root.navigator }));
  }
})(typeof window !== "undefined" ? window : globalThis);
