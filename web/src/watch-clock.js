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

  const api = { WatchClock, validSpeed, finalTenthStart, SPEEDS };
  if (typeof module !== "undefined") module.exports = api; else root.ChillWatchClock = api;
})(typeof window !== "undefined" ? window : globalThis);
