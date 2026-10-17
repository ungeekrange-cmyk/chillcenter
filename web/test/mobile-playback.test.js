"use strict";
const assert = require("node:assert/strict");
const test = require("node:test");
const { createController, phoneViewport, text } = require("../src/mobile-playback.js");
const { WebPlayer } = require("../src/player-web.js");
function deferred() { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; }
function make({ phone = true, fullscreen = "resolve", lock = "resolve", native = false, canvas = false, lang = "en" } = {}) {
  const events = []; let actual = false, panelHidden = false;
  const labels = { textContent: "" }, retry = { textContent: "" };
  const hint = { hidden: true, querySelector: selector => selector === "button" ? retry : labels };
  const delayed = fullscreen === "deferred" ? deferred() : null;
  const video = canvas ? null : { webkitDisplayingFullscreen: false, webkitEnterFullscreen: native ? () => { events.push("native-enter"); video.webkitDisplayingFullscreen = true; } : undefined, webkitExitFullscreen: native ? () => { events.push("native-exit"); video.webkitDisplayingFullscreen = false; } : undefined };
  const panel = { get hidden() { return panelHidden; }, requestFullscreen() { events.push("request"); if (fullscreen === "throw") throw new Error("denied"); if (fullscreen === "reject") return Promise.reject(new Error("denied")); if (delayed) return delayed.promise.then(() => { actual = true; }); return Promise.resolve().then(() => { actual = true; }); } };
  const doc = { documentElement: { lang }, get fullscreenElement() { return actual ? panel : null; }, exitFullscreen() { actual = false; events.push("exit"); return Promise.resolve(); }, getElementById: () => panel, querySelector: () => video };
  const orientation = { lock() { events.push("lock"); return lock === "reject" ? Promise.reject(new Error("no")) : Promise.resolve(); }, unlock() { events.push("unlock"); } };
  const win = { matchMedia: () => ({ matches: phone }), screen: { orientation } };
  const controller = createController({ window: win, document: doc, panel: () => panel, video: () => video, hint: () => hint });
  return { controller, events, hint, labels, retry, delayed, set actual(v) { actual = v; }, set hidden(v) { panelHidden = v; }, get actual() { return actual; } };
}
test("phone predicate is standard and desktop never requests fullscreen", async () => { assert.equal(phoneViewport({ matchMedia: () => ({ matches: false }) }), false); const desktop = make({ phone: false }); assert.equal(await desktop.controller.requestFromGesture(), false); assert.deepEqual(desktop.events, []); });
test("a desktop fullscreenchange never locks orientation or shows mobile guidance", async () => { const desktop = make({ phone: false }); desktop.actual = true; await desktop.controller.onFullscreenChange(); assert.equal(desktop.events.includes("lock"), false); assert.equal(desktop.hint.hidden, true); });
test("request happens before lock and only reports success after actual fullscreen", async () => { const ok = make(); assert.equal(await ok.controller.requestFromGesture(), true); assert.deepEqual(ok.events.slice(0, 2), ["request", "lock"]); const noActual = make({ fullscreen: "reject", canvas: true }); assert.equal(await noActual.controller.requestFromGesture(), false); assert.equal(noActual.hint.hidden, false); });
test("late fullscreen resolution after close is exited and cannot lock or revive its hint", async () => { const race = make({ fullscreen: "deferred" }); const pending = race.controller.requestFromGesture(); race.hidden = true; race.controller.stop(); race.delayed.resolve(); assert.equal(await pending, false); await Promise.resolve(); assert.equal(race.actual, false); assert.ok(race.events.includes("exit")); assert.equal(race.hint.hidden, true); assert.equal(race.events.includes("lock"), false); });
test("a late fullscreenchange while the panel is hidden exits the panel", async () => { const race = make(); race.hidden = true; race.actual = true; await race.controller.onFullscreenChange(); await Promise.resolve(); assert.equal(race.actual, false); assert.ok(race.events.includes("exit")); });
test("orientation refusal is guidance, and native video fallback is real only when active", async () => { const refused = make({ lock: "reject" }); assert.equal(await refused.controller.requestFromGesture(), true); assert.equal(refused.hint.hidden, false); const native = make({ fullscreen: "reject", native: true }); assert.equal(await native.controller.requestFromGesture(), true); assert.ok(native.events.includes("native-enter")); native.controller.stop(); assert.ok(native.events.includes("native-exit")); });
test("browser exit unlocks without re-entering and labels follow language", async () => { const fr = make({ lang: "fr" }); await fr.controller.requestFromGesture(); fr.actual = false; await fr.controller.onFullscreenChange(); assert.ok(fr.events.includes("unlock")); assert.equal(text({ documentElement: { lang: "en" } }, "retry"), "Retry full screen"); });
test("WebPlayer status exposes the real browser fullscreen state", () => { const player = new WebPlayer({ isFullscreen: () => true }); player.info = { media: { title: "Test" }, preferences: {} }; player.mediaId = "m"; player.lease = {}; player.engine = { snapshot: () => ({ loaded: true, paused: false, tracks: [] }), externalSubtitles: () => [] }; assert.equal(player.view().native.fullscreen, true); });
