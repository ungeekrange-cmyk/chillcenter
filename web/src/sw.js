"use strict";
// App-shell cache only. API and media requests are never cached or intercepted:
// the server stays the source of truth and leases cannot be replayed offline.
const CACHE = "chillcenter-__BUILD__";
const SHELL = __SHELL__;
self.addEventListener("install", e => { e.waitUntil(caches.open(CACHE).then(c => c.addAll(SHELL)).then(() => self.skipWaiting())); });
self.addEventListener("activate", e => { e.waitUntil(caches.keys().then(keys => Promise.all(keys.filter(k => k !== CACHE).map(k => caches.delete(k)))).then(() => self.clients.claim())); });
self.addEventListener("fetch", e => {
  const url = new URL(e.request.url);
  if (e.request.method !== "GET" || url.origin !== location.origin || url.pathname.startsWith("/api/")) return;
  e.respondWith(caches.match(e.request).then(hit => hit || fetch(e.request)));
});
