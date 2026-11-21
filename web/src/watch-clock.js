"use strict";
// Port of crates/chill-client/src/watch_clock.rs: conservative credit for the final tenth,
// based only on acknowledged, uninterrupted observations (F-08).
(function (root) {
  const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];
  const validSpeed = s => SPEEDS.includes(s);
  const finalTenthStart = d => Math.floor(d / 10) * 9 + Math.floor(((d % 10) * 9) / 10);

  class WatchClock {
    constructor() { this.sequence = 0; this.acknowledged = null; }
    forgetCredit() { this.acknowledged = null; }
    // obs: {positionMs, durationMs, speed, playing, buffering, seek}; nowMs: monotonic clock.
    event(lease, obs, nowMs) {
      if (!obs.durationMs || obs.durationMs <= 0 || !validSpeed(obs.speed)) { this.forgetCredit(); return null; }
      this.sequence += 1;
      const position = Math.min(obs.positionMs, obs.durationMs);
      let credit = 0;
      if (this.acknowledged) {
        const { event: last, at } = this.acknowledged;
        const elapsed = Math.max(0, nowMs - at);
        if (obs.playing && !obs.buffering && !obs.seek && last.playing && !last.buffering && !last.seek
          && last.duration_ms === obs.durationMs && last.speed === obs.speed
          && position >= last.position_ms && position - last.position_ms <= elapsed * obs.speed + 1000) {
          const finalDistance = Math.max(0, position - Math.max(last.position_ms, finalTenthStart(obs.durationMs)));
          credit = Math.min(Math.floor(elapsed), Math.floor(finalDistance / obs.speed));
        }
      }
      return {
        session_id: lease.session_id, generation: lease.generation, media_id: lease.media_id,
        media_revision: lease.media_revision, sequence: this.sequence, position_ms: Math.round(position),
        duration_ms: Math.round(obs.durationMs), speed: obs.speed, final_elapsed_ms: credit,
        playing: obs.playing, buffering: obs.buffering, seek: obs.seek,
      };
    }
    acknowledge(event, receivedAtMs) { this.acknowledged = { event, at: receivedAtMs }; }
  }

  // Local evidence is used for deletion only within this playback. Samples
  // must advance with the media clock and span the tail of short videos.
  class OfflineWatch {
    constructor() { this.reset(); }
    reset() { this.previous = null; this.credit = 0; this.contiguous = 0; this.fromStart = false; }
    sample(snapshot, now, seek = false) {
      if (!snapshot.loaded || snapshot.error || snapshot.seeking || seek) { this.reset(); return false; }
      const { duration, position, speed, ended, paused, buffering } = snapshot;
      if (!(duration > 0) || !Number.isFinite(position) || !validSpeed(speed)) return false;
      const elapsed = this.previous ? now - this.previous.now : 0;
      const distance = this.previous ? position - this.previous.position : 0;
      const start = finalTenthStart(Math.round(duration * 1000)) / 1000;
      const active = !paused && !buffering;
      if (active && position <= start) this.fromStart = true;
      if (this.previous?.active && (active || ended) && this.previous.speed === speed &&
          elapsed > 0 && elapsed <= 2500 && distance >= 0 && distance <= elapsed * speed / 1000 + 0.3) {
        const tail = Math.max(0, position - Math.max(start, this.previous.position));
        this.credit += Math.min(elapsed / 1000, tail / speed);
        if (this.fromStart) this.contiguous += Math.min(tail, elapsed * speed / 1000);
      } else if (this.previous && (distance > 0.3 || distance < 0 || elapsed > 2500)) {
        this.reset();
        if (active && position <= start) this.fromStart = true;
      }
      this.previous = { now, position, speed, active };
      return Boolean(ended) && (duration < 100
        ? this.fromStart && this.contiguous >= Math.max(0.001, duration - start - 0.1)
        : this.credit >= 10);
    }
  }
  const api = { WatchClock, OfflineWatch, validSpeed, finalTenthStart, SPEEDS };
  if (typeof module !== "undefined") module.exports = api; else root.ChillWatchClock = api;
})(typeof window !== "undefined" ? window : globalThis);
