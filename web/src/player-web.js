"use strict";
// Web playback controller. Implements the `player_command` contract of the Mac client
// (apps/desktop/src/player_bridge.rs) on top of the lease API. Decoding is delegated to
// an Engine (see engines/); nothing here touches the NAS or transcodes (ADR-0014).
(function (root) {
  const Clock = typeof module !== "undefined" ? require("./watch-clock.js") : root.ChillWatchClock;
  const HEARTBEAT_MS = 5000;
  const STATUS_MS = 500;
  const FALLBACK_NOTE = "Piste anglaise absente : piste d'origine utilisée.";

  // F-10/F-11: preferred language, else original/first audio; subtitles off when absent.
  function pickTrack(tracks, language, kind) {
    const list = tracks.filter(t => t.kind === kind);
    if (!list.length) return { id: null, fallback: false };
    const wanted = (language || "").toLowerCase();
    const match = wanted && list.find(t => (t.language || "").toLowerCase().startsWith(wanted.slice(0, 2)));
    if (match) return { id: match.id, fallback: false };
    if (kind === "Subtitle") return { id: null, fallback: false };
    const known = list.some(t => t.language && t.language !== "und");
    return { id: (list.find(t => t.original) || list[0]).id, fallback: Boolean(wanted) && known };
  }

  class WebPlayer {
    // deps: {request, engineFactory, storage, now, setInterval, clearInterval, EventSource, uuid, setFullscreen}
    constructor(deps) {
      this.d = deps; this.state = null; this.timers = []; this.events = null;
      this.clock = new Clock.WatchClock(); this.busy = false;
    }
    get deviceId() {
      let id = null;
      try { id = this.d.storage.getItem("chill.device_id"); } catch { /* memory only */ }
      if (!id) { id = this.d.uuid(); try { this.d.storage.setItem("chill.device_id", id); } catch { /* ignore */ } }
      return id;
    }
    closed(error) { return { stage: "closed", error: error || null }; }

    async command(c, ctx) {
      this.ctx = ctx;
      switch (c.action) {
        case "start": return this.start(c.media_id, c.context || { kind: "standalone", id: null });
        case "status": return this.status();
        case "confirm_transfer": return this.confirmTransfer();
        case "refuse_transfer": await this.stop(false); return this.closed();
        case "next": return this.playNext(c.expected_id);
        case "pause": this.engine?.pause(); await this.report(false, true); return this.view();
        case "resume": this.engine?.play(); return this.view();
        case "stop": await this.stop(true); return this.closed();
        case "mount": return this.view();
        case "seek": return this.seek(c.position);
        case "jump": return this.seek((this.engine?.snapshot().position || 0) + c.seconds);
        case "speed": return this.setSpeed(c.speed);
        case "audio": this.engine?.selectAudio(c.track_id); return this.view();
        case "subtitle": this.engine?.selectSubtitle(c.track_id); return this.view();
        case "external_subtitle": await this.engine?.selectExternalSubtitle?.(c.id); return this.view();
        case "fullscreen": { const want = c.enabled ?? !this.fullscreen; const actual = await this.d.setFullscreen?.(want); this.fullscreen = typeof actual === "boolean" ? actual : want; return this.view(); }
        case "volume": this.engine?.setVolume(c.volume); return this.view();
        default: throw Object.assign(new Error("Commande de lecture inconnue."), { code: "PLAYER_ERROR" });
      }
    }

    api(path) { return `/profiles/${this.ctx.profile}${path}`; }
    control() { const l = this.lease; return { session_id: l.session_id, generation: l.generation, media_id: l.media_id, media_revision: l.media_revision }; }

    async start(mediaId, context) {
      await this.stop(true);
      this.context = context; this.note = ""; this.ended = false; this.nextResolved = null;
      this.info = await this.d.request("GET", this.api(`/media/${mediaId}/playback-info`));
      this.mediaId = mediaId;
      const target = { media_id: mediaId, device_id: this.deviceId, context };
      try {
        const grant = await this.d.request("POST", this.api("/playback/acquire"), { body: target });
        return this.begin(grant);
      } catch (e) {
        if (e.status === 409 && e.code === "PLAYBACK_ACTIVE") {
          const offer = await this.d.request("POST", this.api("/playback/transfer/propose"), { body: { target } });
          this.transfer = { token: offer.token, confirmed: false, remaining_ms: offer.expires_in_ms };
          return this.view();
        }
        throw e;
      }
    }
    async confirmTransfer() {
      if (!this.transfer) return this.closed();
      await this.d.request("POST", this.api("/playback/transfer/confirm"), { body: { token: this.transfer.token } });
      this.transfer.confirmed = true;
      const poll = async () => {
        const s = await this.d.request("GET", this.api("/playback/transfer/status"), { query: [["token", this.transfer.token]] });
        this.transfer.remaining_ms = s.remaining_ms;
        if (s.phase === "activated" && s.lease) { const t = this.transfer; this.transfer = null; return this.begin({ lease: s.lease, personal: s.personal || { position_ms: 0 } }, t); }
        if (s.phase === "expired") { this.transfer = null; return this.closed("Le transfert a expiré."); }
        return null;
      };
      this.transferTimer = this.d.setInterval(async () => { try { const r = await poll(); if (r) this.clearTransfer(); } catch { /* next tick */ } }, STATUS_MS);
      return this.view();
    }
    clearTransfer() { if (this.transferTimer) { this.d.clearInterval(this.transferTimer); this.transferTimer = null; } }

    // The lease is held and the title is known: answer at once with a "loading" view and open the file (container
    // index, first fragments) in the background. A slow open used to keep the PREVIOUS video's title on screen and
    // froze the player controls until it finished (BUG-0070). Failures reach the UI through the next `status` poll.
    async begin(grant) {
      this.clearTransfer(); this.transfer = null;
      this.lease = grant.lease; this.clock = new Clock.WatchClock(); this.lastSeek = false;
      const opening = { engine: this.d.engineFactory(), cancelled: false, promise: null };
      this.opening = opening; this.openError = null;
      opening.promise = this.lastOpen = this.finishOpening(opening, grant);   // lastOpen: settled-state hook for tests
      return this.view();
    }
    async finishOpening(opening, grant) {
      const engine = opening.engine;
      this.t0 = this.d.now();
      try {
        await engine.open({ mediaId: this.mediaId, lease: this.lease, startAt: (grant.personal?.position_ms || 0) / 1000, speed: this.info.speed || 1 });
      } catch (e) {
        if (opening.cancelled) return;
        this.openError = e.message || "Lecture impossible.";
        await this.stop(true);
        return;
      }
      if (opening.cancelled) { try { engine.close(); } catch { /* already closed */ } return; }
      this.engine = engine; this.opening = null;
      const prefs = this.info.preferences || {};
      const tracks = engine.snapshot().tracks || [];
      const audio = pickTrack(tracks, prefs.audio_language, "Audio"), sub = pickTrack(tracks, prefs.subtitle_language, "Subtitle");
      if (audio.id !== null) engine.selectAudio(audio.id);
      engine.selectSubtitle(sub.id);
      if (audio.fallback) this.note = FALLBACK_NOTE;
      engine.play();
      this.timers.push(this.d.setInterval(() => this.heartbeat(), HEARTBEAT_MS));
      this.timers.push(this.d.setInterval(() => this.report(false, false), HEARTBEAT_MS));
      this.openEvents();
    }

    openEvents() {
      if (!this.d.EventSource) return;
      const l = this.lease;
      const url = `/api/2026-10-14/profiles/${this.ctx.profile}/playback/events?session=${encodeURIComponent(l.session_id)}&generation=${l.generation}`;
      this.events = new this.d.EventSource(url);
      this.events.addEventListener("pause", async ev => {
        // Server-ordered pause (transfer): stop locally even with buffered data, then ACK.
        let token; try { token = JSON.parse(ev.data).token; } catch { return; }
        this.engine?.pause();
        const progress = this.observe(false, true);
        try { await this.d.request("POST", this.api("/playback/transfer/ack"), { body: { token, lease: this.control(), progress } }); } catch { /* deadline path */ }
        await this.stop(false);
        this.remoteStopped = true;
      });
    }

    async heartbeat() {
      if (!this.lease) return;
      try { this.lease = { ...this.lease, ...(await this.d.request("POST", this.api("/playback/heartbeat"), { body: this.control() })) }; }
      catch (e) { if (e.status === 409 || e.code === "LEASE_INVALID") { this.engine?.pause(); this.note = "Lecture interrompue : le bail n'est plus valide."; } }
    }
    observe(seek, forcePaused) {
      const s = this.engine.snapshot();
      return this.clock.event(this.lease, {
        positionMs: Math.round((s.position || 0) * 1000), durationMs: Math.round((s.duration || 0) * 1000),
        speed: s.speed || 1, playing: !forcePaused && !s.paused && !s.ended, buffering: Boolean(s.buffering), seek: Boolean(seek),
      }, this.d.now());
    }
    async report(seek, forcePaused) {
      if (!this.lease || !this.engine) return null;
      const event = this.observe(seek, forcePaused);
      if (!event) return null;
      try { await this.d.request("POST", this.api("/playback/progress"), { body: event }); this.clock.acknowledge(event, this.d.now()); }
      catch { this.clock.forgetCredit(); }
      return event;
    }
    async seek(position) {
      if (!this.engine) return this.opening ? this.view() : this.closed();
      this.engine.seek(Math.max(0, position));
      this.ended = false; this.nextResolved = null;
      this.clock.forgetCredit();
      await this.report(true, false);
      return this.view();
    }
    async setSpeed(speed) {
      if (!Clock.validSpeed(speed)) throw Object.assign(new Error("Vitesse invalide."), { code: "PLAYER_ERROR" });
      if (!this.engine) return this.view();
      this.engine.setSpeed(speed);
      this.clock.forgetCredit();
      try { await this.d.request("POST", this.api("/playback/speed"), { body: { lease: this.control(), speed } }); } catch { /* local speed still applies */ }
      return this.view();
    }

    async status() {
      if (!this.engine) {
        if (this.openError) { const error = this.openError; this.openError = null; return this.closed(error); }
        return this.transfer || this.opening ? this.view() : this.closed();
      }
      const s = this.engine.snapshot();
      if (s.ended && !this.ended) {
        this.ended = true;
        await this.report(false, true);
        await this.release();
        await this.resolveNext();
      }
      return this.view();
    }
    async resolveNext() {
      if (this.context.kind === "standalone") { this.nextResolved = { media: null, new_season: false, available: false }; return; }
      try { this.nextResolved = await this.d.request("POST", this.api("/playback/next"), { body: { media_id: this.mediaId, context: this.context } }); }
      catch { this.nextResolved = { media: null, new_season: false, available: false }; }
    }
    async playNext(expectedId) {
      const next = this.nextResolved;
      if (!next || !next.media || next.media.id !== expectedId || !next.available) return this.closed("Vidéo suivante indisponible.");
      return this.start(expectedId, this.context);
    }

    async release() {
      if (!this.lease) return;
      const lease = this.control();
      this.lease = null; this.timers.splice(0).forEach(t => this.d.clearInterval(t));
      this.events?.close(); this.events = null;
      try { await this.d.request("POST", this.api("/playback/release"), { body: { lease, progress: null } }); } catch { /* lease expires */ }
    }
    async stop(report) {
      this.clearTransfer(); this.transfer = null;
      if (this.opening) { this.opening.cancelled = true; try { this.opening.engine.close(); } catch { /* not opened yet */ } this.opening = null; }
      if (this.engine && this.lease && report) await this.report(false, true);
      await this.release();
      this.engine?.close(); this.engine = null;
    }

    view() {
      if (this.transfer) return { stage: "transfer", title: this.info?.media?.title || "", confirmed: this.transfer.confirmed, remaining_ms: this.transfer.remaining_ms, note: "Ce profil lit déjà une vidéo sur un autre appareil. Confirmez pour y interrompre la lecture et la reprendre ici." };
      if (!this.engine && this.opening) {
        return {
          stage: "player", media_id: this.mediaId, title: this.info.media.title, note: this.note,
          position: 0, duration: 0, active: Boolean(this.lease), context: this.context, next: null,
          autoplay_seconds: this.info.preferences?.autoplay_seconds ?? null, external_subtitles: [],
          native: { loaded: false, paused: true, buffering: false, ended: false, error: null, notice: null, speed: this.info.speed || 1, volume: 100, tracks: [], fullscreen: Boolean(this.fullscreen), hardware_decoder: "no", decoder: null },
        };
      }
      if (!this.engine) return this.closed();
      const s = this.engine.snapshot();
      return {
        stage: "player", media_id: this.mediaId, title: this.info.media.title, note: this.note,
        position: s.position || 0, duration: s.duration || 0, active: Boolean(this.lease), context: this.context,
        next: this.nextResolved, autoplay_seconds: this.info.preferences?.autoplay_seconds ?? null,
        external_subtitles: this.engine.externalSubtitles?.() || [],
        native: { loaded: s.loaded, paused: s.paused, buffering: s.buffering, ended: s.ended || this.ended, error: s.error || null, notice: s.notice || null, speed: s.speed || 1, volume: s.volume ?? 100, tracks: s.tracks || [], fullscreen: Boolean(this.fullscreen), hardware_decoder: s.hardware_decoder || "no", decoder: s.decoder || null },
      };
    }
  }

  // `crypto.randomUUID` only exists in secure contexts; the plain-HTTP listener (ADR-0018) is not one, but getRandomValues is.
  function uuidV4(c) {
    if (typeof c.randomUUID === "function") return c.randomUUID();
    const b = c.getRandomValues(new Uint8Array(16));
    b[6] = (b[6] & 0x0f) | 0x40; b[8] = (b[8] & 0x3f) | 0x80;
    const h = Array.from(b, x => x.toString(16).padStart(2, "0")).join("");
    return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
  }

  const api = { WebPlayer, pickTrack, uuidV4 };
  if (typeof module !== "undefined") module.exports = api;
  else {
    root.ChillPlayerCore = api;
    let player = null;
    root.ChillWebPlayer = {
      command(c, ctx) {
        player = player || new WebPlayer({
          request: ctx.request.bind ? (m, p, o) => ctx.request(m, p, o) : ctx.request,
          engineFactory: () => root.ChillEngines.create(document.getElementById("native-video")),
          storage: root.localStorage, now: () => performance.now(), setInterval: (f, t) => root.setInterval(f, t), clearInterval: t => root.clearInterval(t),
          EventSource: root.EventSource, uuid: () => uuidV4(root.crypto),
          setFullscreen: async on => { // browsers refuse this without a recent user gesture: report the real state instead of failing
          const el = document.getElementById("player-panel");
          try { if (on) await el.requestFullscreen?.(); else if (document.fullscreenElement) await document.exitFullscreen(); } catch { /* refused */ }
          return Boolean(document.fullscreenElement);
        },
        });
        return player.command(c, ctx);
      },
    };
  }
})(typeof window !== "undefined" ? window : globalThis);
