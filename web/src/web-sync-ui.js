"use strict";
// Web-only glue (ADR-0019), loaded after the shared UI. When the background synchronisation finds changes, the visible catalogue is
// refreshed quietly; the shared UI files are not modified. Never during playback, never while a command is running, and a user who
// has scrolled down is told instead of being jumped back to the top.
(function () {
  let timer = null, tries = 0;
  const connected = () => typeof state !== "undefined" && state.stage === "connected" && state.profile_id;
  const idle = () => !document.body.classList.contains("player-open") && document.body.getAttribute("aria-busy") !== "true";
  function refresh() {
    timer = null;
    if (!connected()) return;
    if (!idle()) { if (++tries < 8) timer = setTimeout(refresh, 1000); return; }   // busy or playing: try again shortly, then give up
    tries = 0;
    const scrolled = window.scrollY > 300 || (document.scrollingElement && document.scrollingElement.scrollTop > 300);
    if (scrolled) { if (typeof message === "function") message("Le catalogue a changé. Utilisez Actualiser pour l'afficher."); return; }
    if (typeof loadPage === "function") loadPage();
  }
  window.addEventListener("chill:catalog-updated", () => { clearTimeout(timer); tries = 0; timer = setTimeout(refresh, 300); });
})();
