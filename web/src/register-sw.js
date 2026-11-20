"use strict";
// Keep the worker and shell on the page's immutable build. A browser exception
// for a self-signed certificate does not necessarily grant Service Worker trust.
(function () {
  const version = document.currentScript ? new URL(document.currentScript.src).search : "";
  const status = (state, reason) => dispatchEvent(new CustomEvent("chill:pwa-status", { detail: { state, reason } }));
  addEventListener("load", async () => {
    if (!isSecureContext || !("serviceWorker" in navigator)) { status("unavailable", "https"); return; }
    try {
      const registration = await navigator.serviceWorker.register("sw.js" + version, { updateViaCache: "none" });
      const observe = worker => {
        if (!worker) return;
        worker.addEventListener("statechange", () => {
          if (worker.state === "activated") status("ready");
          if (worker.state === "redundant") status("unavailable", "certificate-or-network");
        });
      };
      observe(registration.installing);
      registration.addEventListener("updatefound", () => observe(registration.installing));
      if (registration.active) status("ready");
    } catch { status("unavailable", "certificate-or-network"); }
  }, { once: true });
})();
