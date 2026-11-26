"use strict";
// Text-only subtitle parsing and display.  The caller owns file/range reads,
// selection epochs and media seeking; this module never turns subtitle bytes
// into HTML.
(function (root) {
  const MAX_CUES = 20000, MAX_TEXT = 64 * 1024, MAX_FILE = 16 * 1024 * 1024, FALLBACK_SECONDS = 5;
  const decoder = new TextDecoder("utf-8", { fatal: false });
  const fail = message => Object.assign(new Error(message), { code: "SUBTITLE_ERROR" });
  const plain = value => String(value || "")
    .replace(/&#(x[0-9a-f]+|\d+);/gi, (whole, n) => {
      const code = /^x/i.test(n) ? parseInt(n.slice(1), 16) : Number(n);
      return Number.isInteger(code) && code >= 0 && code <= 0x10ffff ? String.fromCodePoint(code) : whole;
    })
    .replace(/&(amp|lt|gt|quot|apos);/gi, (_, n) => ({ amp: "&", lt: "<", gt: ">", quot: "\"", apos: "'" })[n.toLowerCase()]);
  // These are text formatting tags defined by SRT/WebVTT.  They are removed,
  // never parsed as HTML. Unknown tags stay literal and are escaped below.
  const formatting = /<\/?(?:b|i|u|ruby|rt|c(?:\.[\w-]+)*|v(?:\s+[^>]*)?|lang(?:\s+[^>]*)?|font(?:\s+[^>]*)?)\s*>/gi;
  const textOnly = value => plain(value).replace(formatting, "");
  const safeVtt = value => textOnly(value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const assText = value => plain(value).replace(/\{[^}]*\}/g, "").replace(/\\[Nn]/g, "\n").replace(/\\h/g, " ");
  const boundedText = value => {
    const text = textOnly(value).replace(/\r\n?/g, "\n");
    if (text.length > MAX_TEXT) throw fail("Sous-titre trop long.");
    return text;
  };
  const seconds = value => {
    const m = /^\s*(?:(\d+):)?(\d{1,2}):(\d{2})[,.](\d{1,3})\s*$/.exec(value || "");
    if (!m) throw fail("Horodatage de sous-titre invalide.");
    const hours = Number(m[1] || 0), minutes = Number(m[2]), secs = Number(m[3]);
    if (minutes >= 60 || secs >= 60) throw fail("Horodatage de sous-titre invalide.");
    return hours * 3600 + minutes * 60 + secs + Number(m[4].padEnd(3, "0")) / 1000;
  };
  const record = (start, end, text) => {
    start = Number(start); end = Number(end);
    if (!Number.isFinite(start) || !Number.isFinite(end) || start < 0 || end <= start) throw fail("Intervalle de sous-titre invalide.");
    return { start, end, text: boundedText(text) };
  };
  const dedupe = rows => {
    const seen = new Set(), out = [];
    for (const row of rows) {
      const key = `${row.start}\u0000${row.end}\u0000${row.text}`;
      if (!seen.has(key)) { seen.add(key); out.push(row); }
    }
    if (out.length > MAX_CUES) throw fail("Trop de sous-titres.");
    return out.sort((a, b) => a.start - b.start || a.end - b.end);
  };
  const kind = value => String(value || "").toLowerCase().replace(/^s_text\//, "");

  function packet(track, sample) {
    const codec = kind(track?.codecId), start = Number(sample?.ms) / 1000;
    const duration = Number(sample?.durationMs);
    const end = start + (Number.isFinite(duration) && duration > 0 ? duration / 1000 : FALLBACK_SECONDS);
    if (!Number.isFinite(start) || start < 0) throw fail("Horodatage de sous-titre invalide.");
    if (!["utf8", "ass", "ssa", "webvtt"].includes(codec)) return [];
    let text = decoder.decode(sample?.data || new Uint8Array());
    if (codec === "ass" || codec === "ssa") {
      const body = text.replace(/^Dialogue\s*:\s*/i, "");
      let at = -1, from = 0;
      for (let i = 0; i < 8; i++) { at = body.indexOf(",", from); if (at < 0) return []; from = at + 1; }
      text = assText(body.slice(from));
    } else if (codec === "webvtt") {
      // Matroska's Block contains the complete cue payload. Cue identifiers
      // and positioning/settings live separately in BlockAdditional.
      text = plain(text.replace(/\r\n?/g, "\n"));
    } else text = plain(text);
    text = boundedText(text);
    return text ? [record(start, end, text)] : [];
  }

  function parseFile(source, format) {
    const text = String(source || "").replace(/^\uFEFF/, "").replace(/\r\n?/g, "\n");
    if (text.length > MAX_FILE || new TextEncoder().encode(text).byteLength > MAX_FILE) throw fail("Fichier de sous-titres trop volumineux.");
    const f = kind(format || "srt");
    if (f === "ass" || f === "ssa") {
      const rows = [];
      for (const line of text.split("\n")) if (/^Dialogue\s*:/i.test(line)) {
        const body = line.replace(/^Dialogue\s*:\s*/i, ""), parts = body.split(",");
        if (parts.length < 10) throw fail("Ligne ASS invalide.");
        rows.push(record(seconds(parts[1]), seconds(parts[2]), assText(parts.slice(9).join(","))));
      }
      return dedupe(rows);
    }
    const rows = [];
    for (const raw of text.split(/\n{2,}/)) {
      const lines = raw.split("\n").filter(Boolean);
      if (!lines.length || (f === "vtt" && (/^WEBVTT/i.test(lines[0]) || /^NOTE(?:\s|$)/i.test(lines[0])))) continue;
      const timing = lines.findIndex(line => line.includes("-->"));
      if (timing < 0) { if (f === "vtt" && /^STYLE|^REGION/.test(lines[0])) continue; throw fail("Cue de sous-titre invalide."); }
      const [a, b] = lines[timing].split("-->").map(v => v.trim().split(/\s+/, 1)[0]);
      rows.push(record(seconds(a), seconds(b), plain(lines.slice(timing + 1).join("\n"))));
    }
    return dedupe(rows);
  }

  class Captions {
    constructor({ video, document, VTTCue }) {
      this.video = video; this.document = document || root.document; this.VTTCue = VTTCue || root.VTTCue;
      this.records = []; this.keys = new Set(); this.cues = new Map(); this.suspended = false;
      this.native = String(video?.tagName || video?.localName || "").toLowerCase() === "video" && Boolean(this.VTTCue) && typeof video?.addTextTrack === "function";
      this.render = this.render.bind(this);
      video?.addEventListener?.("timeupdate", this.render); video?.addEventListener?.("seeking", this.render); video?.addEventListener?.("seeked", this.render);
    }
    select(track) {
      this.clear(); if (this.nativeTrack) this.nativeTrack.mode = "disabled";
      this.overlay?.remove?.(); this.overlay = null; this.textTrack = null; this.track = track || null;
      if (!track) return;
      if (this.native) {
        // A <track> without src stays unloaded on some native paths. The
        // programmatic API creates a ready TextTrack for our in-memory cues.
        // Reuse one owned track per video, including across language changes.
        this.nativeTrack ||= this.video.addTextTrack("subtitles", "Chill Center", "");
        this.textTrack = this.nativeTrack; this.textTrack.mode = this.suspended ? "hidden" : "showing";
      } else {
        const overlay = this.document.createElement("div");
        overlay.className = "chill-caption-overlay"; overlay.setAttribute?.("aria-live", "off"); overlay.hidden = true;
        (this.video?.parentElement || this.document.body).append(overlay); this.overlay = overlay;
      }
    }
    add(rows) {
      for (const input of rows || []) {
        const row = record(input.start, input.end, input.text), key = `${row.start}\u0000${row.end}\u0000${row.text}`;
        if (this.keys.has(key)) continue;
        if (this.records.length >= MAX_CUES) throw fail("Trop de sous-titres.");
        this.keys.add(key); this.records.push(row);
        if (this.textTrack) { const cue = new this.VTTCue(row.start, row.end, safeVtt(row.text)); this.textTrack.addCue(cue); this.cues.set(key, cue); }
      }
      this.records.sort((a, b) => a.start - b.start || a.end - b.end); this.render();
    }
    clear() {
      if (this.textTrack) for (const cue of this.cues.values()) { try { this.textTrack.removeCue(cue); } catch { /* already detached */ } }
      this.records = []; this.keys.clear(); this.cues.clear(); if (this.overlay) { this.overlay.textContent = ""; this.overlay.hidden = true; }
    }
    prune(beforeSeconds) {
      const keep = [];
      for (const row of this.records) {
        if (row.end >= beforeSeconds) { keep.push(row); continue; }
        const key = `${row.start}\u0000${row.end}\u0000${row.text}`;
        const cue = this.cues.get(key);
        if (cue && this.textTrack) this.textTrack.removeCue(cue);
        this.cues.delete(key); this.keys.delete(key);
      }
      this.records = keep; this.render();
    }
    suspend(value) { this.suspended = Boolean(value); if (this.textTrack) this.textTrack.mode = this.suspended ? "hidden" : "showing"; this.render(); }
    render() {
      if (!this.overlay) return;
      const now = Number(this.video?.currentTime || 0), active = this.suspended ? [] : this.records.filter(row => row.start <= now && now < row.end).map(row => row.text);
      this.overlay.textContent = active.join("\n"); this.overlay.hidden = !active.length;
    }
    close() {
      this.clear(); if (this.nativeTrack) this.nativeTrack.mode = "disabled";
      this.overlay?.remove?.();
      this.video?.removeEventListener?.("timeupdate", this.render); this.video?.removeEventListener?.("seeking", this.render); this.video?.removeEventListener?.("seeked", this.render);
      this.overlay = this.textTrack = this.nativeTrack = null;
    }
  }

  function matchTrack(tracks, choice) {
    if (!choice || choice.off) return null;
    const aliases = { eng:"en", fra:"fr", fre:"fr", deu:"de", ger:"de", spa:"es", ita:"it", jpn:"ja", por:"pt", zho:"zh", chi:"zh", rus:"ru", nld:"nl", dut:"nl", kor:"ko" };
    const language = value => {
      const key = (value || "").trim().toLowerCase().split(/[-_]/)[0];
      return key === "und" || !/^[a-z]{2,3}$/.test(key) ? "" : aliases[key] || key;
    };
    const title = value => (value || "").trim().toLowerCase();
    const wanted = language(choice.language);
    const available = tracks.filter(track => !track.unavailable);
    const named = available.filter(track => {
      const key = language(track.language);
      return (!wanted || !key || key === wanted) && choice.title && title(track.name || track.title) === title(choice.title);
    });
    if (named.length === 1) return named[0];
    // Missing and undefined language tags can match when only one fits.
    const matches = available.filter(track => language(track.language) === wanted);
    return matches.length === 1 ? matches[0] : null;
  }

  const api = { packet, parseFile, Captions, matchTrack };
  if (typeof module !== "undefined") module.exports = api; else root.ChillSubtitles = api;
})(typeof window !== "undefined" ? window : globalThis);
