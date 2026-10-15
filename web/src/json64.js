"use strict";
// Lossless JSON for uint64 fields such as media_revision (API guide, "Entiers 64 bits").
// Integers beyond 2^53-1 become BigInt on parse and are re-emitted as exact JSON numbers.
(function (root) {
  const SAFE = BigInt(Number.MAX_SAFE_INTEGER);
  const MARK = "\u0000big:";

  function parse(text) {
    let out = "", i = 0;
    const n = text.length;
    while (i < n) {
      const c = text[i];
      if (c === '"') {
        let j = i + 1;
        while (j < n && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
        out += text.slice(i, j + 1);
        i = j + 1;
      } else if (c === "-" || (c >= "0" && c <= "9")) {
        let j = i + 1;
        while (j < n && /[0-9.eE+-]/.test(text[j])) j++;
        const token = text.slice(i, j);
        out += /^-?\d{16,}$/.test(token) && (BigInt(token) > SAFE || BigInt(token) < -SAFE)
          ? JSON.stringify(MARK + token) : token;
        i = j;
      } else {
        out += c;
        i++;
      }
    }
    return JSON.parse(out, (_key, value) =>
      typeof value === "string" && value.startsWith(MARK) ? BigInt(value.slice(MARK.length)) : value);
  }

  function stringify(value) {
    if (typeof value === "bigint") return value.toString();
    if (value === null || typeof value !== "object") return JSON.stringify(value);
    if (Array.isArray(value)) return "[" + value.map(v => (v === undefined ? "null" : stringify(v))).join(",") + "]";
    const parts = [];
    for (const [k, v] of Object.entries(value)) {
      if (v === undefined || typeof v === "function") continue;
      parts.push(JSON.stringify(k) + ":" + stringify(v));
    }
    return "{" + parts.join(",") + "}";
  }

  const api = { parse, stringify };
  if (typeof module !== "undefined") module.exports = api;
  else root.ChillJson64 = api;
})(typeof window !== "undefined" ? window : globalThis);
