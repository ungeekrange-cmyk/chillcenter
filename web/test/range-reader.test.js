"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { RangeReader } = require("../src/range-reader.js");
const { sniffContainer } = require("../src/mse-engine.js");

const lease = { session_id: "s", generation: 7 };
function fake(status = 206, extra = {}) {
  const calls = [];
  const data = Uint8Array.from({ length: 200000 }, (_, i) => i % 251);
  const fetch = async (url, init) => {
    calls.push({ url, headers: init.headers });
    const m = /bytes=(\d+)-(\d+)/.exec(init.headers.Range), a = +m[1], b = Math.min(+m[2], data.length - 1);
    const headers = new Map(Object.entries({ "content-range": `bytes ${a}-${b}/${data.length}`, etag: '"e1"', ...extra }));
    return { status, headers: { get: k => headers.get(k) ?? null }, arrayBuffer: async () => data.slice(a, b + 1).buffer };
  };
  return { fetch, calls, data };
}

test("lease headers on every read, ETag pinned after the first, head cached", async () => {
  const f = fake();
  const r = new RangeReader({ mediaId: "m", lease, fetch: f.fetch });
  assert.equal(await r.open(), 200000);
  assert.equal(f.calls[0].headers["X-Chill-Session"], "s");
  assert.equal(f.calls[0].headers["X-Chill-Generation"], "7");
  assert.equal(f.calls[0].headers["If-Match"], undefined);
  assert.deepEqual([...await r.read(10, 19)], [...f.data.subarray(10, 20)]);
  assert.equal(f.calls.length, 1, "served from the cached head");
  await r.read(100000, 100009);
  assert.equal(f.calls[1].headers["If-Match"], '"e1"');
  assert.equal(f.calls[1].headers.Range, "bytes=100000-100009");
  assert.equal(f.calls[1].url, "/api/2026-10-14/media/m/stream");
});

test("stream errors are explicit: changed file, lost lease, bad range", async () => {
  for (const [status, code] of [[412, "MEDIA_CHANGED"], [409, "LEASE_INVALID"], [416, "RANGE_INVALID"]]) {
    const r = new RangeReader({ mediaId: "m", lease, fetch: fake(status).fetch });
    await assert.rejects(r.open(), e => e.code === code && e.status === status);
  }
});

test("a server that ignores Range with a full 200 body is refused", async () => {
  const body = new Uint8Array(200000);
  const fetch = async () => ({ status: 200, headers: { get: k => (k === "content-length" ? "200000" : null) }, arrayBuffer: async () => body.buffer });
  await assert.rejects(new RangeReader({ mediaId: "m", lease, fetch }).open(), /plage demandée/);
});

test("container sniffing", () => {
  assert.equal(sniffContainer(Uint8Array.from([0, 0, 0, 24, 102, 116, 121, 112])), "mp4");
  assert.equal(sniffContainer(Uint8Array.from([0x1a, 0x45, 0xdf, 0xa3, 0, 0, 0, 0])), "mkv");
  assert.equal(sniffContainer(Uint8Array.from([82, 73, 70, 70, 0, 0, 0, 0])), "avi");
  assert.equal(sniffContainer(new Uint8Array(8)), "unknown");
});
