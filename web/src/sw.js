"use strict";
// App-shell cache only. API and media requests are never cached or intercepted:
// the server stays the source of truth and leases cannot be replayed offline.
const CACHE = "chillcenter-__BUILD__";
const SHELL = __SHELL__;
const CODECS = __CODECS__;
const assets = new Set([...SHELL, ...CODECS].map(path => new URL(path, location.href).href));
self.addEventListener("message", e => {
  if (e.data !== "prepare-downloads" || !e.ports[0]) return;
  const prepare = async () => {
    try {
      const cache = await caches.open(CACHE);
      const missing = [];
      for (const url of CODECS) if (!await cache.match(url)) missing.push(url);
      if (missing.length) await cache.addAll(missing);
      await cache.put(prepared, new Response("ready"));
      e.ports[0].postMessage({ ready: true });
    } catch { e.ports[0].postMessage({ ready: false }); }
  };
  e.waitUntil(prepare());
});
const prepared = new URL("offline-ready", location.href).href;
self.addEventListener("install", e => {
  e.waitUntil((async () => {
    const previous = await caches.keys();
    const cache = await caches.open(CACHE);
    await cache.addAll(SHELL);
    for (const old of previous.filter(key => key.startsWith("chillcenter-"))) {
      if (await (await caches.open(old)).match(prepared)) {
        await cache.addAll(CODECS);
        await cache.put(prepared, new Response("ready"));
        break;
      }
    }
    await self.skipWaiting();
  })());
});
self.addEventListener("activate", e => {
  e.waitUntil((async () => {
    const previous = (await caches.keys()).filter(key => key.startsWith("chillcenter-") && key !== CACHE);
    for (const key of previous.slice(0, -1)) await caches.delete(key);
    await self.clients.claim();
  })());
});
self.addEventListener("fetch", e => {
  const url = new URL(e.request.url);
  if (e.request.method !== "GET" || e.request.headers.has("range") || url.origin !== location.origin || url.pathname.startsWith("/api/")) return;
  // Navigation always checks the installed server release first. An old worker
  // must not freeze an installed app at the HTML version in its cache.
  if (e.request.mode === "navigate") {
    e.respondWith(fetch(e.request, { cache: "no-store", signal: AbortSignal.timeout(4000) }).then(response => {
      if ([502, 503, 504].includes(response.status)) throw new Error("Server unavailable");
      return response;
    }).catch(() => caches.open(CACHE).then(c => c.match("./"))));
    return;
  }
  if (assets.has(url.href)) {
    e.respondWith(caches.open(CACHE).then(c => c.match(e.request)).then(hit => hit || fetch(e.request)));
  } else if (/^[a-f0-9]{12}$/.test(url.searchParams.get("v") || "")) {
    // An already open tab can still ask for a versioned decoder during
    // activation. Only previously cached assets can be served here.
    e.respondWith(caches.match(e.request).then(hit => hit || fetch(e.request)));
  }
});
