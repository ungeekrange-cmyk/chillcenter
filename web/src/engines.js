"use strict";
// Engine registry. An Engine decodes/plays bytes fetched with Range from the leased stream
// (client-side compute, ADR-0014). Interface:
//   open({mediaId, lease, startAt, speed}) -> Promise      play() pause() seek(s) setSpeed(x) setVolume(0..100)
//   selectAudio(id) selectSubtitle(id|null) externalSubtitles() snapshot() close()
//   snapshot() -> {loaded, paused, buffering, ended, error, notice, position, duration, speed, volume,
//                  tracks:[{id, kind:"Audio"|"Subtitle", language, title, selected, original}],
//                  hardware_decoder, decoder}
// No engine that can read real files is bundled yet: MP4/MKV/AVI demuxing and WASM decoding
// require a measured spike and their own ADR. Until then the player explains it plainly.
(function (root) {
  class UnsupportedEngine {
    constructor() { this.volume = 100; this.speed = 1; }
    async open() {
      throw Object.assign(new Error("Le moteur de lecture du client web n'est pas encore installé : la lecture vidéo est indisponible dans cette version."), { code: "PLAYER_ERROR" });
    }
    play() {} pause() {} seek() {} setSpeed(s) { this.speed = s; } setVolume(v) { this.volume = v; }
    selectAudio() {} selectSubtitle() {} externalSubtitles() { return []; } close() {}
    snapshot() { return { loaded: false, paused: true, buffering: false, ended: false, error: null, position: 0, duration: 0, speed: this.speed, volume: this.volume, tracks: [] }; }
  }
  const registry = { factories: [], create(host) { for (const f of registry.factories) { const e = f(host); if (e) return e; } return new UnsupportedEngine(); } };
  const api = { registry, UnsupportedEngine };
  if (typeof module !== "undefined") module.exports = api; else root.ChillEngines = registry;
})(typeof window !== "undefined" ? window : globalThis);
