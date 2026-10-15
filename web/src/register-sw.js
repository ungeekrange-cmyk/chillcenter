"use strict";
// Service Worker registration is best effort: browsers refuse it on an untrusted certificate.
if ("serviceWorker" in navigator && location.protocol === "https:") {
  addEventListener("load", () => { navigator.serviceWorker.register("sw.js").catch(() => {}); });
}
