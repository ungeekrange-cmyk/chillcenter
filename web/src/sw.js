"use strict";
// App-shell cache only. API and media requests are never cached or intercepted:
// the server stays the source of truth and leases cannot be replayed offline.
const CACHE = "chillcenter-__BUILD__";
const SHELL = __SHELL__;
self.addEventListener("install", e => { e.waitUntil(caches.open(CACHE).then(c => c.addAll(SHELL)).then(() => self.skipWaiting())); });
self.addEventListener("activate", e => { e.waitUntil(caches.keys().then(keys => Promise.all(keys.filter(k => k.startsWith("chillcenter-") && k !== CACHE).map(k => caches.delete(k)))).then(() => self.clients.claim())); });
self.addEventListener("fetch", e => {
  const url = new URL(e.request.url);
  if (e.request.method !== "GET" || e.request.headers.has("range") || url.origin !== location.origin || url.pathname.startsWith("/api/")) return;
  // Navigation always checks the installed server release first. An old worker
  // must not freeze an installed app at the HTML version in its cache.
  if (e.request.mode === "navigate") {
    e.respondWith(fetch(e.request, { cache: "no-store" }).catch(() => caches.open(CACHE).then(c => c.match("./"))));
    return;
  }
  const shell = new Set(SHELL.map(path => new URL(path, location.href).href));
  if (!shell.has(url.href)) return;
  e.respondWith(caches.open(CACHE).then(c => c.match(e.request)).then(hit => hit || fetch(e.request)));
});
