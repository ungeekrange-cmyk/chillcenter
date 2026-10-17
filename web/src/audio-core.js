"use strict";
// Thin JS layer over the WebAssembly audio transcoder (web/decoder/audio, ADR-0020). Pure logic shared by the Worker and the
// Node tests: compressed access units in (AC-3, E-AC-3, DTS, TrueHD, MP3), AAC-LC access units out (1024 samples each).
(function (root) {
  // MP3 is only selected when the current MediaSource implementation cannot
  // accept it natively. This keeps a capable browser on the zero-CPU path.
  const CODECS = { ac3: 0, eac3: 1, dts: 2, truehd: 3, mlp: 4, mp3: 5 };
  const RATES = [96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000];
  const fail = (op, code) => Object.assign(new Error(`Transcodeur audio : ${op} a échoué (code ${code}).`), { code: "PLAYER_ERROR" });

  /** AudioSpecificConfig of AAC-LC for a rate and channel count (what the init segment declares before any audio is encoded). */
  function audioSpecificConfig(rate, channels) {
    const f = RATES.indexOf(rate);
    if (f < 0 || channels < 1 || channels > 7) return null;
    return Uint8Array.of((2 << 3) | (f >> 1), ((f & 1) << 7) | (channels << 3));
  }

  function wrap(M, { codec, rate, channels = 2, kbps = 192 }) {
    if (!(codec in CODECS)) throw Object.assign(new Error(`Codec audio non pris en charge : ${codec}.`), { code: "PLAYER_ERROR" });
    function drain(out) {
      for (;;) {
        const n = M._cc_a_next();
        if (n === 0) return;
        if (n < 0) throw fail("paquet AAC trop grand", n);
        const p = M._cc_a_packet();
        out.push(M.HEAPU8.slice(p, p + n));   // heap fetched after each call: it may have grown
      }
    }
    return {
      open() {
        const r = M._cc_a_open(CODECS[codec], rate, channels, kbps);
        if (r < 0) throw fail("ouverture", r);
        const p = M._cc_a_asc(), size = M._cc_a_asc_size();
        this.asc = M.HEAPU8.slice(p, p + size);
        this.frameSize = M._cc_a_frame_size(); this.priming = M._cc_a_initial_padding();
      },
      /** frames: Uint8Array[] (one compressed access unit each) -> AAC access units ready so far (the encoder holds back ~1 frame). */
      convert(frames) {
        const out = [];
        for (const data of frames) {
          const p = M._cc_a_alloc(data.length);
          M.HEAPU8.set(data, p);
          try { M._cc_a_send(p, data.length); } finally { M._cc_a_free(p); }
          drain(out);
        }
        return out;
      },
      /** End of stream: everything still inside the decoder, resampler and encoder. */
      flush() { const out = []; M._cc_a_flush(); drain(out); return out; },
      /** Seek: forget the stream; codecs stay configured. */
      reset() { const r = M._cc_a_reset(); if (r < 0) throw fail("réinitialisation", r); },
      close() { M._cc_a_close(); },
      get errors() { return M._cc_a_errors(); },
      get sourceRate() { return M._cc_a_src_rate(); },
      get sourceChannels() { return M._cc_a_src_channels(); },
    };
  }

  const api = { wrap, audioSpecificConfig, CODECS, RATES };
  if (typeof module !== "undefined") module.exports = api; else root.ChillAudioCore = api;
})(typeof self !== "undefined" ? self : globalThis);
