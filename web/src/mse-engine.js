"use strict";
// MP4 playback through MediaSource (ADR-0015): bytes come from the leased Range reader, mp4.js
// remuxes them to fMP4 in the browser, the browser decodes. No dependency, no transcoding.
(function (root) {
  const Mp4 = typeof module !== "undefined" ? require("./mp4.js") : root.ChillMp4;
  const Mkv = typeof module !== "undefined" ? require("./mkv.js") : root.ChillMkv;
  const Avi = typeof module !== "undefined" ? require("./avi.js") : root.ChillAvi;
  const Soft = typeof module !== "undefined" ? require("./soft-video.js") : root.ChillSoft;
  const Text = typeof module !== "undefined" ? require("./subtitles.js") : root.ChillSubtitles;
  const { RangeReader } = typeof module !== "undefined" ? require("./range-reader.js") : root.ChillRangeReader;
  const fail = message => Object.assign(new Error(message), { code: "PLAYER_ERROR" });
  const LABELS = { "ac-3": "AC-3", "ec-3": "E-AC-3", flac: "FLAC", opus: "Opus", xvid: "Xvid (MPEG-4)", dx50: "DivX (MPEG-4)", divx: "DivX (MPEG-4)", mp4v: "MPEG-4" };
  const codecLabel = c => LABELS[c] || (/^mp4a\.6B/i.test(c || "") ? "MP3" : /^mp4a/.test(c || "") ? "AAC" : (c || "inconnu").split(".")[0]);
  const language = doc => (doc?.documentElement?.lang || "en").toLowerCase().startsWith("fr") ? "fr" : "en";
  const playbackNotice = (doc, reason) => {
    const fr = language(doc) === "fr";
    if (reason === "interrupted") return fr
      ? "La lecture a été interrompue. Touchez Lecture pour reprendre."
      : "Playback was interrupted. Tap Play to resume.";
    return fr
      ? "La lecture a été bloquée par le navigateur. Touchez Lecture pour reprendre."
      : "Playback was blocked by the browser. Tap Play to resume.";
  };
  const playbackFailure = doc => language(doc) === "fr"
    ? "Le navigateur n'a pas pu démarrer la lecture."
    : "The browser could not start playback.";

  function sniffContainer(head) {
    const tag = String.fromCharCode(...head.subarray(4, 8));
    if (tag === "ftyp" || tag === "moov" || tag === "mdat" || tag === "free" || tag === "wide") return "mp4";
    if (head[0] === 0x1a && head[1] === 0x45 && head[2] === 0xdf && head[3] === 0xa3) return "mkv";
    if (String.fromCharCode(...head.subarray(0, 4)) === "RIFF" && String.fromCharCode(...head.subarray(8, 12)) === "AVI ") return "avi";
    return "unknown";
  }

  // Respect Off and known languages. A lone text track without a language
  // is still useful (common in subtitled releases); do not invent its language.
  function initialSubtitle(tracks, preferred) {
    const wanted = String(preferred || "").toLowerCase();
    if (!wanted || wanted === "off") return null;
    const match = tracks.find(t => (t.language || "").toLowerCase().startsWith(wanted.slice(0, 2)));
    if (match) return match;
    const only = tracks.length === 1 ? tracks[0] : null;
    return only && (!only.language || only.language.toLowerCase() === "und") ? only : null;
  }

  class MseEngine {
    constructor(host, deps) { this.host = host; this.d = deps; this.streams = []; this.epoch = 0; this.closed = false; this.errorText = null; this.notice = null; this.rate = 1; this.hardware = "no"; this.captionEpoch = 0; this.external = []; this.handlesSubtitlePreference = true; }

    cacheSeconds() {
      let forward = 128, back = 64;
      try { const s = JSON.parse(this.d.storage.getItem("chill.settings") || "{}").playback_cache; if (s) { forward = s.forward_mib; back = s.back_mib; } } catch { /* defaults */ }
      const bps = Math.max(1, this.reader.size / Math.max(1, this.movie.duration));
      return { forward: Math.min(90, Math.max(8, forward * 1048576 / bps)), back: Math.min(60, back * 1048576 / bps) };
    }

    async open({ mediaId, lease, startAt, speed, subtitleLanguage, profileId }) {
      const MS = this.d.MediaSource;
      if (!MS) throw fail("Ce navigateur ne sait pas lire de vidéo par MediaSource.");
      // The player can leave (stop, another video) while this runs: after every await, check that this engine is still wanted, so that a
      // slow open never replaces the `<video>` of the engine that took over the shared host element (review 9, C-4).
      const alive = () => { if (this.closed) throw fail("Ouverture annulée."); };
      alive();
      this.reader = new RangeReader({ mediaId, lease, fetch: this.d.fetch });
      await this.reader.open(); alive();
      const kind = sniffContainer(this.reader.head);
      if (kind === "unknown") throw fail("Format de fichier non reconnu.");
      // AVI stores MP3 in an fMP4 stream for MSE.  Safari iPhone does not
      // reliably accept that sample entry (nor bare MPEG audio) in its
      // ManagedMediaSource, so give only that unsupported case to the local
      // Worker transcoder. Browsers that support either native form retain
      // their direct MP3 path.
      const nativeMp3 = MS.isTypeSupported('audio/mp4; codecs="mp4a.6B"') || MS.isTypeSupported("audio/mpeg");
      this.src = kind === "mp4" ? new Mp4.Mp4Source(this.reader)
        : kind === "mkv" ? new Mkv.MkvSource(this.reader, { audio: root.ChillAudio || null })
          : new Avi.AviSource(this.reader, { audio: root.ChillAudio || null, transcodeMp3: !nativeMp3 });
      this.movie = await this.src.open(); alive();
      // A legacy saved position at EOF cannot prime a playable buffer.
      // Match the native player: only the last 250 ms count as the end.
      if (this.movie.duration > 0 && startAt >= this.movie.duration - 0.25) startAt = 0;
      this.subtitleTracks = this.movie.tracks.filter(t => t.kind === "subtitle" && t.textSupported);
      this.captionLanguage = String(subtitleLanguage || "").toLowerCase();
      this.activeSubtitle = initialSubtitle(this.subtitleTracks, this.captionLanguage);
      this.src.setSubtitle?.(this.activeSubtitle?.id ?? null);
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
      alive();
      this.video = this.d.document.createElement(softTrack ? "audio" : "video");
      this.video.playsInline = true; this.video.preload = "auto";
      if (softTrack) {
        this.canvas = this.d.document.createElement("canvas");
        this.host.replaceChildren(this.canvas, this.video);
      } else this.host.replaceChildren(this.video);
      if (Text) { this.captions = new Text.Captions({ video: this.video, document: this.d.document, VTTCue: this.d.VTTCue }); this.captions.select(this.activeSubtitle); }
      this.bind();
      this.attachMediaSource(MS);
      await new Promise((res, rej) => { this.ms.addEventListener("sourceopen", res, { once: true }); setTimeout(() => rej(fail("MediaSource indisponible.")), 5000); }); alive();
      if (this.movie.duration) this.ms.duration = this.movie.duration;
      for (const t of [video, this.activeAudio].filter(Boolean)) {
        const sb = this.ms.addSourceBuffer(t.mime);
        if (this.d.managedMediaSource) sb.addEventListener("bufferedchange", () => {
          if (this.started && !this.video.paused) this.pump();
        });
        if (!t.rawStream) { sb.mode = "segments"; sb.timestampOffset = t.shift; }
        this.streams.push({ track: t, sb, done: false, needOffset: true });
      }
      this.src.setActive?.(this.streams.map(x => x.track.id));
      this.cache = this.cacheSeconds();
      for (const s of this.streams) if (!s.track.rawStream) { await this.append(s.sb, Mp4.initSegment(s.track)); alive(); }
      this.video.playbackRate = speed || 1; this.rate = speed || 1;
      await this.rewind(startAt || 0); alive();
      this.video.currentTime = startAt || 0;
      if (softTrack) { await this.initSoft(softTrack, softEntry, startAt || 0); alive(); }
      else this.probeHardware(video);
      // Show the first picture after a few seconds of media. Filling the whole
      // user cache here delayed Play by up to 90 seconds of downloads.
      await this.pump(3);
      this.started = true;
      // Sidecar discovery is lease-scoped and must not delay first playback.
      if (profileId) {
        this.subtitleBase = `/api/2026-10-14/profiles/${encodeURIComponent(profileId)}/media/${encodeURIComponent(mediaId)}/subtitles`;
        this.subtitleQuery = new URLSearchParams({ session: lease.session_id, generation: String(lease.generation) }).toString();
        this.loadExternalSubtitles();
      }
    }

    // Safari on iPhone exposes ManagedMediaSource rather than the older
    // MediaSource API. WebKit requires remote playback to be disabled (unless
    // an AirPlay alternative is provided) before that source is attached.
    // Keep this narrow: buffer ownership and range fetching remain identical.
    attachMediaSource(MS) {
      if (this.d.managedMediaSource) this.video.disableRemotePlayback = true;
      this.ms = new MS();
      if (this.d.managedMediaSource) this.ms.addEventListener("startstreaming", () => {
        if (this.started) this.pump();
      });
      this.url = this.d.URL.createObjectURL(this.ms);
      this.video.src = this.url;
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
      v.addEventListener("timeupdate", () => { this.pump(); if (this.activeSubtitle) this.captions?.prune(Math.max(0, v.currentTime - Math.max(30, this.cache?.back || 0) - 30)); });
      v.addEventListener("seeking", () => {
        if (!this.started) return;
        this.captions?.suspend(true);
        // Removing one old buffer can make the browser correct its time to
        // another old range. That is not a new user request. Explicit seek()
        // commands below still replace the target while reset is in flight.
        if (!this.repositioning && !this.isBuffered(v.currentTime)) this.reposition(v.currentTime).catch(e => { if (!this.closed) this.errorText = e.message; });
        this.soft?.reset(v.currentTime); // the decoder always restarts from the keyframe before the target
      });
      v.addEventListener("seeked", () => this.captions?.suspend(false));
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
      let limit = this.video.currentTime - keepBack;
      const stream = this.streams.find(s => s.sb === sb);
      if (stream?.track.kind === "video") {
        // MSE also removes dependent pictures up to the NEXT random access
        // point. Cutting inside the current GOP therefore deletes the very
        // picture we are trying to show (especially with a zero back cache).
        const safe = (stream.keyframes || []).filter(t => t <= limit);
        if (!safe.length) return;
        limit = Math.max(...safe);
      }
      if (b.start(0) < limit) {
        await this.removeRange(sb, 0, limit);
        if (stream?.keyframes) stream.keyframes = stream.keyframes.filter(t => t >= limit);
      }
    }
    bufferedEnd(sb, t) {
      const b = sb.buffered;
      for (let i = 0; i < b.length; i++) if (b.start(i) <= t + 0.25 && b.end(i) >= t) return b.end(i);
      return t;
    }
    isBuffered(t) { return this.streams.every(s => this.bufferedEnd(s.sb, t) > t + 0.05); }
    async rewind(t) { await this.src.seek(t); for (const s of this.streams) { s.done = false; s.needOffset = true; s.keyframes = []; } }
    async reposition(t) {
      const epoch = ++this.epoch;
      this.repositionTarget = t; this.repositioning = true;
      if (this.activeSubtitle) this.captions?.clear();
      const run = async () => {
        await this.waitIdle();
        if (this.closed || epoch !== this.epoch) return;
        for (const s of this.streams) {
          if (s.sb.updating) { try { s.sb.abort(); } catch { /* ignore */ } }
          await this.removeRange(s.sb, 0, Infinity);
          if (this.closed || epoch !== this.epoch) return;
        }
        await this.rewind(t);
        if (this.closed || epoch !== this.epoch) return;
        // Removing the old A/V ranges can resolve the browser's original seek
        // against one still-present old buffer. Anchor again AFTER the reset.
        this.video.currentTime = t;
        await this.pump(3);
      };
      // Rapid language changes/seeks share a single demux cursor. Old work
      // settles before the newest request rewinds it.
      this.repositionJob = (this.repositionJob || Promise.resolve()).then(run, run).finally(() => { if (epoch === this.epoch) this.repositioning = false; });
      return this.repositionJob;
    }
    waitIdle() { return this.pumping ? new Promise(r => setTimeout(r, 30)).then(() => this.waitIdle()) : Promise.resolve(); }

    async pump(targetSeconds = this.cache?.forward) {
      if (this.pumping || this.closed || !this.ms || this.ms.readyState === "closed") return;
      this.pumping = true;
      const epoch = this.epoch;
      try {
        for (let again = true; again && !this.closed && epoch === this.epoch;) {
          again = false;
          for (const s of this.streams) if (!s.done && await this.feed(s, epoch, targetSeconds)) again = true;
        }
        if (!this.closed && epoch === this.epoch && this.streams.every(s => s.done) && this.ms.readyState === "open" && !this.streams.some(s => s.sb.updating)) this.ms.endOfStream();
      } catch (e) { this.errorText = e.message || "Lecture impossible."; }
      finally { this.pumping = false; }
    }
    async feed(s, epoch, targetSeconds = this.cache.forward) {
      const t = this.video.currentTime;
      // Safari owns the managed buffer budget. Keep just a short safety prime
      // when it says it has enough, and restart on startstreaming/eviction.
      // The safety prime also covers WebKit versions that miss that event
      // after an out-of-buffer seek.
      if (this.d.managedMediaSource && this.ms.streaming === false) targetSeconds = Math.min(3, targetSeconds);
      if (this.bufferedEnd(s.sb, t) - t >= targetSeconds) return false;
      await this.evict(s.sb, this.cache.back);
      const fragment = await this.src.next(s.track.id);
      if (epoch !== this.epoch || this.closed) return false;
      this.receiveSubtitles();
      if (!fragment) { s.done = true; return false; }
      let bytes = fragment.data;
      if (s.track.rawStream && s.needOffset && fragment.startTime !== undefined) { s.sb.timestampOffset = fragment.startTime; s.needOffset = false; bytes = Mp4.mp3Resync(bytes); }
      await this.append(s.sb, bytes);
      if (fragment.keyframes?.length) s.keyframes = [...(s.keyframes || []), ...fragment.keyframes];
      return epoch === this.epoch;
    }

    // ---- Engine interface --------------------------------------------------------------
    async play() {
      if (!this.video) return false;
      // Continue filling in the background; this must never gate the user's
      // play gesture or the native media element's play() call.
      this.pump();
      const attempt = (this.playAttempt || 0) + 1;
      this.playAttempt = attempt;
      try {
        await this.video.play();
        if (this.closed || attempt !== this.playAttempt) return false;
        this.playBlocked = false;
        this.playBlockReason = null;
        return true;
      } catch (e) {
        // A newer pause/retry/close owns the media element. Its state must not
        // be overwritten by an old promise settling late.
        if (this.closed || attempt !== this.playAttempt) return false;
        if (e?.name === "NotAllowedError") {
          // Keep the loaded MediaSource buffers. A visible Play control is a
          // new gesture and can resume without downloading it again.
          this.playBlocked = true;
          this.playBlockReason = "blocked";
        } else if (e?.name === "AbortError") {
          // The current media element was interrupted without a newer local
          // command. Keep its buffers and offer a fresh, trusted retry.
          this.playBlocked = true;
          this.playBlockReason = "interrupted";
        } else if (e?.name !== "AbortError") {
          this.errorText = playbackFailure(this.d.document);
        }
        return false;
      }
    }
    pause() { this.playAttempt = (this.playAttempt || 0) + 1; this.video?.pause(); }
    seek(seconds) {
      if (!this.video) return;
      const target = Math.max(0, Math.min(seconds, this.movie.duration));
      if (this.started && (this.repositioning || !this.isBuffered(target))) this.reposition(target).catch(e => { if (!this.closed) this.errorText = e.message; });
      this.video.currentTime = target;
    }
    setSpeed(x) { this.rate = x; if (this.video) this.video.playbackRate = x; }
    setVolume(v) { if (this.video) this.video.volume = Math.min(1, Math.max(0, v / 100)); }
    receiveSubtitles() {
      const packets = this.src.drainSubtitles?.() || [];
      if (!this.activeSubtitle || !this.captions) return;
      try { for (const p of packets) this.captions.add(Text.packet(this.activeSubtitle, p)); }
      catch { this.captionNotice = language(this.d.document) === "fr" ? "Cette piste de sous-titres contient un texte invalide." : "This subtitle track contains invalid text."; }
    }
    async selectSubtitle(id) {
      this.subtitleManual = true; ++this.captionEpoch; this.subtitleFetch?.abort();
      const next = this.subtitleTracks?.find(t => t.id === id) || null;
      if (next === (this.activeSubtitle || null) && !this.activeExternal) return;
      this.activeSubtitle = next; this.activeExternal = null; this.captionNotice = null;
      this.captions?.select(next); this.src.setSubtitle?.(next?.id ?? null);
      if (next && this.started) await this.reposition(this.video.currentTime);
    }
    externalSubtitles() { return this.external; }
    async loadExternalSubtitles() {
      this.externalFetch = new AbortController();
      try {
        const response = await this.d.fetch(`${this.subtitleBase}?${this.subtitleQuery}`, { credentials: "same-origin", cache: "no-store", signal: this.externalFetch.signal });
        if (!response.ok) throw fail("Sous-titres externes indisponibles.");
        const data = await response.json();
        if (this.closed) return;
        this.external = (data.items || []).slice(0, 64).filter(t => /^sub-[a-f0-9]+$/.test(t.id) && /\.(srt|vtt|ass|ssa)$/i.test(t.title || ""));
        const preferred = this.external.filter(t => !t.unavailable && !t.ambiguous && this.captionLanguage && (t.language || "").toLowerCase().startsWith(this.captionLanguage.slice(0, 2)));
        if (!this.subtitleManual && !this.activeSubtitle && preferred.length === 1) await this.selectExternalSubtitle(preferred[0].id);
      } catch (e) {
        if (!this.closed && e.name !== "AbortError") this.captionNotice = language(this.d.document) === "fr" ? "La recherche de sous-titres externes a échoué." : "External subtitle discovery failed.";
      }
    }
    async selectExternalSubtitle(id) {
      const track = this.external.find(t => t.id === id && !t.unavailable);
      if (!track || !Text || !this.captions) return;
      const epoch = ++this.captionEpoch; this.subtitleManual = true;
      this.subtitleFetch?.abort(); this.subtitleFetch = new AbortController();
      this.activeSubtitle = null; this.activeExternal = track; this.captionNotice = null;
      this.src.setSubtitle?.(null); this.captions.select(null);
      try {
        const response = await this.d.fetch(`${this.subtitleBase}/${encodeURIComponent(id)}?${this.subtitleQuery}`, { credentials: "same-origin", cache: "no-store", signal: this.subtitleFetch.signal });
        if (!response.ok) throw fail("Sous-titres externes indisponibles.");
        if (Number(response.headers.get("content-length")) > 16 * 1024 * 1024) throw fail("Sous-titres trop volumineux.");
        const text = await response.text();
        if (this.closed || epoch !== this.captionEpoch) return;
        const records = Text.parseFile(text, track.title.split(".").at(-1));
        if (!records.length) throw fail("Ce fichier ne contient aucun sous-titre lisible.");
        this.captions.select(track); this.captions.add(records);
      } catch (e) {
        if (!this.closed && epoch === this.captionEpoch && e.name !== "AbortError") this.captionNotice = language(this.d.document) === "fr" ? "Impossible de charger ce fichier de sous-titres." : "This subtitle file could not be loaded.";
      }
    }
    async selectAudio(id) {
      const next = this.audio?.find(t => t.id === id), s = this.streams.find(x => x.track.kind === "audio");
      if (!next || !s || s.track === next) return;
      this.epoch++; await this.waitIdle();
      await this.repositionJob;
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
        loaded: v.readyState >= 1, paused: v.paused, buffering: !v.paused && !v.ended && v.readyState < 3, seeking: Boolean(v.seeking), ended: v.ended,
        error: this.errorText || this.soft?.errorText || null,
        notice: [this.notice, this.captionNotice, this.playBlocked ? playbackNotice(this.d.document, this.playBlockReason) : null].filter(Boolean).join(" ") || null,
        play_blocked: Boolean(this.playBlocked), position: v.currentTime || 0, duration: this.movie.duration, speed: v.playbackRate, volume: Math.round(v.volume * 100),
        tracks: [
          ...(this.audio || []).map((t, i) => ({ id: t.id, kind: "Audio", language: t.language, title: t.sourceLabel ? `${t.sourceLabel} (converti en AAC)` : codecLabel(t.codec), selected: t === this.activeAudio, original: i === 0 })),
          ...(this.subtitleTracks || []).map(t => ({ id: t.id, kind: "Subtitle", language: t.language, title: t.name || t.language || "Sous-titres", selected: t === this.activeSubtitle })),
          ...(this.activeExternal ? [{ ...this.activeExternal, kind: "Subtitle", selected: true }] : []),
        ],
        hardware_decoder: this.soft ? "no" : this.hardware, decoder: this.soft ? this.softLabel : "Décodeur du navigateur",
      };
    }
    close() {
      this.closed = true; this.epoch++; this.playAttempt = (this.playAttempt || 0) + 1;
      ++this.captionEpoch; this.externalFetch?.abort(); this.subtitleFetch?.abort(); this.captions?.close();
      try { this.src?.close?.(); } catch { /* ignore */ }
      try { this.soft?.close(); } catch { /* ignore */ }
      try { this.video?.pause(); this.video?.removeAttribute("src"); this.video?.load(); } catch { /* ignore */ }
      if (this.url) this.d.URL.revokeObjectURL(this.url);
      this.host?.replaceChildren();
    }
  }

  const api = { MseEngine, sniffContainer, initialSubtitle };
  if (typeof module !== "undefined") module.exports = api;
  else {
    // Prefer the established MSE implementation where both exist. iPhone
    // Safari may expose only ManagedMediaSource, which has the same engine
    // contract but needs the explicit remote-playback setting above.
    const MediaSourceCtor = root.MediaSource || root.ManagedMediaSource;
    if (MediaSourceCtor && root.ChillEngines) {
      root.ChillEngines.factories.push(host => new MseEngine(host, { fetch: (...a) => root.fetch(...a), MediaSource: MediaSourceCtor, managedMediaSource: !root.MediaSource && Boolean(root.ManagedMediaSource), document: root.document, URL: root.URL, storage: root.localStorage, navigator: root.navigator, requestAnimationFrame: f => root.requestAnimationFrame(f), cancelAnimationFrame: h => root.cancelAnimationFrame(h) }));
    }
  }
})(typeof window !== "undefined" ? window : globalThis);
