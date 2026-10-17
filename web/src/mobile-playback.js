"use strict";
// Web-only mobile presentation adapter. It deliberately does not talk to the
// playback lease, decoder, cache or server: fullscreen and orientation are a
// browser presentation concern layered over the shared client player.
(function (root) {
  const PHONE_QUERY = "(max-width: 760px), (max-height: 500px) and (pointer: coarse)";
  const copy = {
    en: { rotate: "Rotate your phone", retry: "Retry full screen" },
    fr: { rotate: "Tournez votre téléphone", retry: "Réessayer le plein écran" },
  };
  function phoneViewport(win) { return Boolean(win?.matchMedia?.(PHONE_QUERY).matches); }
  function text(doc, key) { return (doc?.documentElement?.lang || "en").toLowerCase().startsWith("fr") ? copy.fr[key] : copy.en[key]; }

  function createController(deps) {
    const win = deps.window, doc = deps.document;
    const panel = () => deps.panel?.() || doc?.getElementById?.("player-panel");
    const video = () => deps.video?.() || doc?.querySelector?.("#native-video video");
    const actual = () => Boolean(doc?.fullscreenElement || video()?.webkitDisplayingFullscreen);
    let requested = false, generation = 0, hintWanted = false, lockingToken = null, lockingPromise = null;
    const landscape = () => Number(win?.innerWidth) > Number(win?.innerHeight);
    const current = token => token === generation && !panel()?.hidden;
    function setHint(visible) {
      hintWanted = visible;
      const hint = deps.hint?.(); if (!hint) return;
      // Orientation locking may be unavailable even when the user has already
      // rotated the phone. Guidance must never imply that playback is gated.
      hint.hidden = !visible || landscape();
      const label = hint.querySelector?.("[data-mobile-orientation-text]"); if (label) label.textContent = text(doc, "rotate");
      const retry = hint.querySelector?.("button"); if (retry) retry.textContent = text(doc, "retry");
    }
    function refreshLanguage() { if (hintWanted) setHint(true); }
    function refreshViewport() { setHint(hintWanted); }
    function unlock() { try { win?.screen?.orientation?.unlock?.(); } catch { /* browser-specific no-op */ } }
    function playbackOpen() { return Boolean(panel() && !panel().hidden); }
    async function lockLandscape(token) {
      if (lockingToken === token && lockingPromise) return lockingPromise;
      const lockOnce = async () => {
      if (!phoneViewport(win)) { setHint(false); return false; }
      if (!current(token) || !actual()) return false;
      const lock = win?.screen?.orientation?.lock;
      if (typeof lock !== "function") { if (current(token)) setHint(true); return false; }
      try {
        await lock.call(win.screen.orientation, "landscape");
        if (!current(token) || !actual()) { unlock(); return false; }
        setHint(false); return true;
      } catch { if (current(token) && actual()) setHint(true); return false; }
      };
      lockingToken = token;
      lockingPromise = lockOnce();
      try { return await lockingPromise; }
      finally { if (lockingToken === token) { lockingToken = null; lockingPromise = null; } }
    }
    function attemptNativeVideoFullscreen() {
      const el = video();
      if (!el || typeof el.webkitEnterFullscreen !== "function") return false;
      try { el.webkitEnterFullscreen(); } catch { return false; }
      // WebKit may enter asynchronously; a later event locks orientation. Do
      // not report success before the browser says that it is actually active.
      return actual();
    }
    function leaveNativeVideoFullscreen() {
      const el = video();
      if (el?.webkitDisplayingFullscreen && typeof el.webkitExitFullscreen === "function") {
        try { el.webkitExitFullscreen(); } catch { /* user/browser owns the exit */ }
      }
    }
    function leavePanelFullscreen() {
      if (doc?.fullscreenElement === panel() && typeof doc.exitFullscreen === "function") {
        Promise.resolve(doc.exitFullscreen()).catch(() => {});
      }
    }
    // A browser may grant a request after the player has already closed. Only
    // clean it up when that same panel is still hidden: a newly opened player
    // reuses the panel, so it must retain ownership of its own fullscreen.
    function cleanLateFullscreen(token) {
      if (token === generation || playbackOpen() || !actual()) return;
      unlock(); setHint(false); leavePanelFullscreen(); leaveNativeVideoFullscreen();
    }
    // Called directly from a user gesture. Do not await before requestFullscreen:
    // browsers reject requests made after an async player/API step.
    function requestFromGesture() {
      if (!phoneViewport(win) || !playbackOpen()) return Promise.resolve(false);
      const token = ++generation; requested = true;
      if (typeof panel()?.requestFullscreen !== "function") {
        // iPhone's video-only API may require the direct gesture stack too.
        // Once the video is ready, do not defer this fallback to a microtask.
        const entered = attemptNativeVideoFullscreen();
        if (entered) return lockLandscape(token).then(() => actual() && current(token));
        setHint(true); return Promise.resolve(false);
      }
      let request; try { request = panel()?.requestFullscreen?.(); } catch { request = null; }
      return Promise.resolve(request).catch(() => false).then(async () => {
        if (!current(token)) { cleanLateFullscreen(token); return false; }
        if (actual()) { await lockLandscape(token); return actual() && current(token); }
        const nativeActual = attemptNativeVideoFullscreen();
        if (nativeActual) { await lockLandscape(token); return actual() && current(token); }
        if (current(token)) setHint(true);
        return false;
      });
    }
    async function onFullscreenChange() {
      const token = generation;
      if (!playbackOpen()) { unlock(); setHint(false); leavePanelFullscreen(); leaveNativeVideoFullscreen(); return; }
      // A desktop user may use the existing fullscreen button. It is valid
      // browser behavior, but it must never acquire phone-only orientation
      // control or render the mobile guidance outside its media query.
      if (!phoneViewport(win)) { setHint(false); return; }
      if (actual()) { await lockLandscape(token); return; }
      unlock(); if (requested) setHint(false);
    }
    function stop() {
      generation++; requested = false; unlock(); setHint(false);
      lockingToken = null; lockingPromise = null;
      leavePanelFullscreen();
      leaveNativeVideoFullscreen();
    }
    return { requestFromGesture, onFullscreenChange, stop, refreshLanguage, refreshViewport, isActualFullscreen: actual, isPhone: () => phoneViewport(win) };
  }

  function install() {
    const doc = root.document;
    if (!doc || root.__CHILL_MOBILE_PLAYBACK_INSTALLED__) return;
    root.__CHILL_MOBILE_PLAYBACK_INSTALLED__ = true;
    const panel = doc.getElementById("player-panel"); if (!panel) return;
    const hint = doc.createElement("div");
    hint.id = "mobile-orientation-hint"; hint.hidden = true; hint.setAttribute("role", "status");
    hint.innerHTML = '<span data-mobile-orientation-text></span><button type="button"></button>';
    panel.append(hint);
    const retryPlay = doc.createElement("button");
    retryPlay.id = "web-play-retry"; retryPlay.type = "button"; retryPlay.hidden = true;
    retryPlay.setAttribute("aria-live", "polite");
    doc.getElementById("player-stage").append(retryPlay);
    retryPlay.addEventListener("click", () => {
      retryPlay.disabled = true;
      // Call before any await to retain the fresh trusted touch gesture.
      const pending = root.ChillWebPlayer.resumeFromGesture();
      if (controller.isPhone()) controller.requestFromGesture();
      Promise.resolve(pending).catch(() => false).finally(() => { retryPlay.disabled = false; });
    });
    root.addEventListener("chill:web-player-view", event => {
      retryPlay.textContent = doc.documentElement.lang.startsWith("fr") ? "▶ Toucher pour lancer la vidéo" : "▶ Tap to play video";
      retryPlay.hidden = panel.hidden || event.detail?.stage !== "player" || Boolean(event.detail.native?.error) || !event.detail.native?.play_blocked;
    });
    const controller = createController({ window: root, document: doc, panel: () => panel, hint: () => hint });
    controller.refreshLanguage(); root.ChillMobilePlayback = controller;
    hint.querySelector("button").addEventListener("click", () => controller.requestFromGesture());
    doc.addEventListener("fullscreenchange", () => { controller.onFullscreenChange(); });
    doc.addEventListener("webkitbeginfullscreen", () => { controller.onFullscreenChange(); }, true);
    doc.addEventListener("webkitendfullscreen", () => { controller.onFullscreenChange(); }, true);
    root.addEventListener?.("languagechange", () => controller.refreshLanguage());
    root.addEventListener?.("resize", () => controller.refreshViewport(), { passive: true });
    root.addEventListener?.("orientationchange", () => controller.refreshViewport(), { passive: true });
    new MutationObserver(() => { if (panel.hidden) { controller.stop(); retryPlay.hidden = true; } }).observe(panel, { attributes: true, attributeFilter: ["hidden"] });
    // `chillPlay` runs synchronously through its first await, opening the panel.
    // Request fullscreen immediately after that prefix, while activation remains.
    const original = root.chillPlay;
    if (typeof original === "function") root.chillPlay = function mobileChillPlay(...args) {
      const result = original.apply(this, args);
      if (controller.isPhone()) controller.requestFromGesture();
      return result;
    };
  }
  const api = { PHONE_QUERY, phoneViewport, text, createController, install };
  if (typeof module !== "undefined") module.exports = api;
  else { root.ChillMobilePlaybackCore = api; install(); }
})(typeof window !== "undefined" ? window : globalThis);
