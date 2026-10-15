"use strict";
// MP4 playback through MediaSource (ADR-0015): bytes come from the leased Range reader, mp4.js
// remuxes them to fMP4 in the browser, the browser decodes. No dependency, no transcoding.
(function (root) {
  const Mp4 = typeof module !== "undefined" ? require("./mp4.js") : root.ChillMp4;
  const Mkv = typeof module !== "undefined" ? require("./mkv.js") : root.ChillMkv;
  const Avi = typeof module !== "undefined" ? require("./avi.js") : root.ChillAvi;
  const Soft = typeof module !== "undefined" ? require("./soft-video.js") : root.ChillSoft;
  const { RangeReader } = typeof module !== "undefined" ? require("./range-reader.js") : root.ChillRangeReader;
  const fail = message => Object.assign(new Error(message), { code: "PLAYER_ERROR" });
  const LABELS = { "ac-3": "AC-3", "ec-3": "E-AC-3", flac: "FLAC", opus: "Opus", xvid: "Xvid (MPEG-4)", dx50: "DivX (MPEG-4)", divx: "DivX (MPEG-4)", mp4v: "MPEG-4" };
  const codecLabel = c => LABELS[c] || (/^mp4a\.6B/i.test(c || "") ? "MP3" : /^mp4a/.test(c || "") ? "AAC" : (c || "inconnu").split(".")[0]);

  function sniffContainer(head) {
    const tag = String.fromCharCode(...head.subarray(4, 8));
    if (tag === "ftyp" || tag === "moov" || tag === "mdat" || tag === "free" || tag === "wide") return "mp4";
    if (head[0] === 0x1a && head[1] === 0x45 && head[2] === 0xdf && head[3] === 0xa3) return "mkv";
    if (String.fromCharCode(...head.subarray(0, 4)) === "RIFF" && String.fromCharCode(...head.subarray(8, 12)) === "AVI ") return "avi";
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
      if (kind === "unknown") throw fail("Format de fichier non reconnu.");
      this.src = kind === "mp4" ? new Mp4.Mp4Source(this.reader) : kind === "mkv" ? new Mkv.MkvSource(this.reader, { audio: root.ChillAudio || null }) : new Avi.AviSource(this.reader);
      this.movie = await this.src.open();
      // MP3 is not accepted inside audio/mp4 everywhere (Chromium builds): fall back to a bare MPEG audio stream.
      for (const t of this.movie.tracks) {
        if (t.kind === "audio" && t.codec === "mp4a.6B" && !MS.isTypeSupported(t.mime) && MS.isTypeSupported("audio/mpeg")) { t.mime = "audio/mpeg"; t.rawStream = true; }
      }
      const supported = t => t.mime && MS.isTypeSupported(t.mime);
      let video = this.movie.tracks.find(t => t.kind === "video" && supported(t));
      let softTrack = null, softEntry = null;
      if (!video) {
        const v = this.movie.tracks.find(t => t.kind === "video");
        if (!v) throw fail("Aucune piste vidéo.");
        softEntry = Soft.decoders.find(v);
        if (!softEntry || !this.src.rawRead) {
          throw fail(v.decoder || !v.mime
            ? `Le décodeur logiciel ${codecLabel(v.codec || v.type)} n'est pas installé : cette vidéo n'est pas encore lisible dans le navigateur.`
            : `Codec vidéo ${codecLabel(v.codec || v.type)} non pris en charge par ce navigateur.`);
        }
        softTrack = v;
      }
      this.audio = this.movie.tracks.filter(t => t.kind === "audio" && supported(t));
      const missing = this.movie.tracks.filter(t => t.kind === "audio" && !supported(t));
      if (missing.length) this.notice = `Audio non pris en charge par ce navigateur : ${[...new Set(missing.map(t => codecLabel(t.codec || t.type)))].join(", ")}.`;
      if (!this.audio.length && missing.length) this.notice += " Lecture sans son.";
      this.activeAudio = this.audio[0] || null;
      if (softTrack && !this.activeAudio) throw fail("Cette vidéo nécessite le décodage logiciel, et son audio n'est pas pris en charge par ce navigateur : lecture impossible pour l'instant.");

      // Software video: the audio element is the master clock, frames are painted on a canvas.
      this.video = this.d.document.createElement(softTrack ? "audio" : "video");
      this.video.playsInline = true; this.video.preload = "auto";
      if (softTrack) {
        this.canvas = this.d.document.createElement("canvas");
        this.host.replaceChildren(this.canvas, this.video);
      } else this.host.replaceChildren(this.video);
      this.bind();
      this.ms = new MS();
      this.url = this.d.URL.createObjectURL(this.ms);
      this.video.src = this.url;
      await new Promise((res, rej) => { this.ms.addEventListener("sourceopen", res, { once: true }); setTimeout(() => rej(fail("MediaSource indisponible.")), 5000); });
      if (this.movie.duration) this.ms.duration = this.movie.duration;
      for (const t of [video, this.activeAudio].filter(Boolean)) {
        const sb = this.ms.addSourceBuffer(t.mime);
        if (!t.rawStream) { sb.mode = "segments"; sb.timestampOffset = t.shift; }
        this.streams.push({ track: t, sb, done: false, needOffset: true });
      }
      this.src.setActive?.(this.streams.map(x => x.track.id));
      this.cache = this.cacheSeconds();
      for (const s of this.streams) if (!s.track.rawStream) await this.append(s.sb, Mp4.initSegment(s.track));
      this.video.playbackRate = speed || 1; this.rate = speed || 1;
      await this.rewind(startAt || 0);
      this.video.currentTime = startAt || 0;
      if (softTrack) await this.initSoft(softTrack, softEntry, startAt || 0);
      else this.probeHardware(video);
      await this.pump();
      this.started = true;
    }

    async initSoft(track, entry, startAt) {
      const decoder = await entry.create(track);
      this.canvas.width = track.width || 2; this.canvas.height = track.height || 2;
      const ctx = this.canvas.getContext("2d");
      this.soft = new Soft.SoftVideo({
        src: this.src, track, decoder, clock: { time: () => this.video.currentTime }, ctx,
        resize: (w, h) => { this.canvas.width = w; this.canvas.height = h; },
        makeImage: (w, h) => ctx.createImageData(w, h),
        raf: f => this.d.requestAnimationFrame(f), caf: h => this.d.cancelAnimationFrame(h),
      });
      this.softLabel = entry.label || "Décodeur logiciel (WebAssembly)";
      await this.soft.reset(startAt);
      this.soft.start();
    }

    bind() {
      const v = this.video;
      v.addEventListener("error", () => { this.errorText = "Le navigateur n'a pas pu décoder cette vidéo."; });
      v.addEventListener("timeupdate", () => this.pump());
      v.addEventListener("seeking", () => {
        if (!this.started) return;
        if (!this.isBuffered(v.currentTime)) this.reposition(v.currentTime);
        this.soft?.reset(v.currentTime); // the decoder always restarts from the keyframe before the target
      });
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
    async rewind(t) { await this.src.seek(t); for (const s of this.streams) { s.done = false; s.needOffset = true; } }
    async reposition(t) {
      this.epoch++;
      await this.waitIdle();
      for (const s of this.streams) { if (s.sb.updating) { try { s.sb.abort(); } catch { /* ignore */ } } await this.removeRange(s.sb, 0, Infinity); }
      await this.rewind(t);
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
      await this.evict(s.sb, this.cache.back);
      const fragment = await this.src.next(s.track.id);
      if (epoch !== this.epoch || this.closed) return false;
      if (!fragment) { s.done = true; return false; }
      let bytes = fragment.data;
      if (s.track.rawStream && s.needOffset && fragment.startTime !== undefined) { s.sb.timestampOffset = fragment.startTime; s.needOffset = false; bytes = Mp4.mp3Resync(bytes); }
      await this.append(s.sb, bytes);
      return epoch === this.epoch;
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
      s.track = next; if (!next.rawStream) s.sb.timestampOffset = next.shift; this.activeAudio = next; s.needOffset = true;
      this.src.setActive?.(this.streams.map(x => x.track.id));
      if (!next.rawStream) await this.append(s.sb, Mp4.initSegment(next));
      await this.reposition(this.video.currentTime);
    }
    snapshot() {
      const v = this.video;
      if (!v) return { loaded: false, paused: true, buffering: false, ended: false, error: this.errorText, position: 0, duration: 0, speed: this.rate, volume: 100, tracks: [] };
      return {
        loaded: v.readyState >= 1, paused: v.paused, buffering: !v.paused && !v.ended && v.readyState < 3, ended: v.ended,
        error: this.errorText || this.soft?.errorText || null, notice: this.notice, position: v.currentTime || 0, duration: this.movie.duration, speed: v.playbackRate, volume: Math.round(v.volume * 100),
        tracks: (this.audio || []).map((t, i) => ({ id: t.id, kind: "Audio", language: t.language, title: t.sourceLabel ? `${t.sourceLabel} (converti en AAC)` : codecLabel(t.codec), selected: t === this.activeAudio, original: i === 0 })),
        hardware_decoder: this.soft ? "no" : this.hardware, decoder: this.soft ? this.softLabel : "Décodeur du navigateur",
      };
    }
    close() {
      this.closed = true; this.epoch++;
      try { this.src?.close?.(); } catch { /* ignore */ }
      try { this.soft?.close(); } catch { /* ignore */ }
      try { this.video?.pause(); this.video?.removeAttribute("src"); this.video?.load(); } catch { /* ignore */ }
      if (this.url) this.d.URL.revokeObjectURL(this.url);
      this.host?.replaceChildren();
    }
  }

  const api = { MseEngine, sniffContainer };
  if (typeof module !== "undefined") module.exports = api;
  else if (root.MediaSource && root.ChillEngines) {
    root.ChillEngines.factories.push(host => new MseEngine(host, { fetch: (...a) => root.fetch(...a), MediaSource: root.MediaSource, document: root.document, URL: root.URL, storage: root.localStorage, navigator: root.navigator, requestAnimationFrame: f => root.requestAnimationFrame(f), cancelAnimationFrame: h => root.cancelAnimationFrame(h) }));
  }
})(typeof window !== "undefined" ? window : globalThis);
