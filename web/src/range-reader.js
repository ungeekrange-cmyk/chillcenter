"use strict";
// Leased byte reader over the stream route: single Range per request, headers carry the lease,
// ETag pinned after the first answer (If-Match afterwards). Same-origin fetch only (ADR-0012/0013).
(function (root) {
  const MAX_READ = 8 * 1024 * 1024;
  const fail = (message, code, status) => Object.assign(new Error(message), { code: code || "PLAYER_ERROR", status: status ?? null });

  class RangeReader {
    constructor({ mediaId, lease, fetch }) {
      this.url = `/api/2026-10-14/media/${mediaId}/stream`;
      this.lease = lease; this.fetch = fetch; this.size = 0; this.etag = null; this.head = null;
    }
    async open() {
      const data = await this.range(0, 65535, true);
      this.head = data;
      return this.size;
    }
    async range(start, end, first) {
      const headers = { "X-Chill-Session": this.lease.session_id, "X-Chill-Generation": String(this.lease.generation), Range: `bytes=${start}-${end}` };
      if (this.etag) headers["If-Match"] = this.etag;
      let response;
      try { response = await this.fetch(this.url, { headers, credentials: "same-origin", cache: "no-store" }); }
      catch { throw fail("Connexion au serveur perdue pendant la lecture."); }
      if (response.status === 412) throw fail("Le fichier a changé sur le serveur : relancez la lecture.", "MEDIA_CHANGED", 412);
      if (response.status === 409) throw fail("Lecture interrompue : un autre appareil a repris ce profil.", "LEASE_INVALID", 409);
      if (response.status === 416) throw fail("Plage vidéo invalide.", "RANGE_INVALID", 416);
      if (response.status !== 206 && response.status !== 200) throw fail("Le serveur a refusé la lecture.", "STREAM_REFUSED", response.status);
      if (first) {
        const total = /\/(\d+)\s*$/.exec(response.headers.get("content-range") || "");
        this.size = total ? Number(total[1]) : Number(response.headers.get("content-length") || 0);
        this.etag = response.headers.get("etag");
        if (!this.size) throw fail("Taille de la vidéo inconnue.");
      }
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (response.status === 200 && bytes.length > end - start + 1) throw fail("Le serveur n'a pas respecté la plage demandée.");
      return bytes;
    }
    /** Inclusive byte range [start, end], clamped to the file. */
    async read(start, end) {
      end = Math.min(end, this.size - 1);
      if (start < 0 || start > end) throw fail("Plage vidéo invalide.", "RANGE_INVALID");
      if (this.head && end < this.head.length) return this.head.subarray(start, end + 1);
      const parts = [];
      for (let p = start; p <= end; p += MAX_READ) parts.push(await this.range(p, Math.min(p + MAX_READ - 1, end), false));
      if (parts.length === 1) return parts[0];
      const out = new Uint8Array(end - start + 1); let o = 0;
      for (const part of parts) { out.set(part, o); o += part.length; }
      return out;
    }
  }

  const api = { RangeReader };
  if (typeof module !== "undefined") module.exports = api; else root.ChillRangeReader = api;
})(typeof window !== "undefined" ? window : globalThis);
