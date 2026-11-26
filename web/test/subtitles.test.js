"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { packet, parseFile, Captions, matchTrack } = require("../src/subtitles.js");
const bytes = value => new TextEncoder().encode(value);

class Node {
  constructor(name) { this.tagName = name; this.localName = name.toLowerCase(); this.children = []; this.parentElement = null; this.hidden = false; this.textContent = ""; this.listeners = {}; this.track = name === "track" ? new TextTrack() : null; }
  append(node) { node.parentElement = this; this.children.push(node); }
  addTextTrack() { const track = new TextTrack(); (this.textTracks ||= []).push(track); return track; }
  remove() { if (this.parentElement) this.parentElement.children = this.parentElement.children.filter(x => x !== this); this.parentElement = null; }
  setAttribute() {} addEventListener(type, fn) { (this.listeners[type] ||= []).push(fn); } removeEventListener(type, fn) { this.listeners[type] = (this.listeners[type] || []).filter(x => x !== fn); }
  fire(type) { for (const fn of this.listeners[type] || []) fn(); }
}
class TextTrack { constructor() { this.cues = []; this.mode = "disabled"; } addCue(cue) { this.cues.push(cue); } removeCue(cue) { this.cues = this.cues.filter(x => x !== cue); } }
class Cue { constructor(startTime, endTime, text) { this.startTime = startTime; this.endTime = endTime; this.text = text; } getCueAsHTML() { return { textContent: this.text }; } }
const fakeDocument = () => ({ body: new Node("body"), createElement: name => new Node(name) });

test("embedded UTF8, ASS and WebVTT packets preserve timing and never expose markup", () => {
  assert.deepEqual(packet({ codecId: "S_TEXT/UTF8" }, { ms: 1250, durationMs: 8000, data: bytes("Bonjour &amp; <b>toi</b> <img>") }), [{ start: 1.25, end: 9.25, text: "Bonjour & toi <img>" }]);
  assert.deepEqual(packet({ codecId: "S_TEXT/ASS" }, { ms: 0, durationMs: 0, data: bytes("0,0,Default,Name,0,0,0,,{\\i1}Salut\\Nmonde\\h!") }), [{ start: 0, end: 5, text: "Salut\nmonde !" }]);
  assert.deepEqual(packet({ codecId: "S_TEXT/WEBVTT" }, { ms: 3000, durationMs: 500, data: bytes("Première ligne\nDeuxième ligne\nTroisième ligne") }), [{ start: 3, end: 3.5, text: "Première ligne\nDeuxième ligne\nTroisième ligne" }]);
  assert.deepEqual(packet({ codecId: "S_HDMV/PGS" }, { ms: 0, durationMs: 1, data: bytes("x") }), []);
});

test("sidecar parsers support plain SRT, VTT and ASS with bounded valid intervals", () => {
  assert.deepEqual(parseFile("1\n00:00:01,000 --> 00:00:02,500\nA &amp; B", "srt"), [{ start: 1, end: 2.5, text: "A & B" }]);
  assert.deepEqual(parseFile("WEBVTT\n\n00:00:03.000 --> 00:00:04.000 align:start\nVTT", "vtt"), [{ start: 3, end: 4, text: "VTT" }]);
  assert.deepEqual(parseFile("WEBVTT\n\n01:02.345 --> 01:04.000\nVTT court", "vtt"), [{ start: 62.345, end: 64, text: "VTT court" }]);
  assert.deepEqual(parseFile("Dialogue: 0,0:00:05.00,0:00:06.00,Default,,0,0,0,,{\\b1}ASS\\Ntexte", "ass"), [{ start: 5, end: 6, text: "ASS\ntexte" }]);
  assert.throws(() => parseFile("1\n00:00:02,000 --> 00:00:01,000\nno", "srt"), /Intervalle/);
  assert.throws(() => parseFile("1\n00:61:00,000 --> 00:62:00,000\nno", "srt"), /Horodatage/);
});

test("native captions escape cue HTML, deduplicate, clear on off and clean up their owned track", () => {
  const document = fakeDocument(), video = new Node("video"); document.body.append(video);
  const captions = new Captions({ video, document, VTTCue: Cue });
  captions.select({ language: "fr", name: "Français" });
  captions.add([{ start: 1, end: 2, text: "<img src=x onerror=1> &" }, { start: 1, end: 2, text: "<img src=x onerror=1> &" }]);
  const track = video.textTracks[0];
  assert.equal(track.mode, "showing"); assert.equal(track.cues.length, 1); assert.equal(track.cues[0].text, "&lt;img src=x onerror=1&gt; &amp;");
  captions.suspend(true); assert.equal(track.mode, "hidden"); captions.suspend(false); assert.equal(track.mode, "showing");
  captions.select(null); assert.equal(track.mode, "disabled"); assert.equal(track.cues.length, 0);
  captions.select({ language: "en" }); assert.equal(video.textTracks.length, 1, "language changes reuse the owned native track");
  captions.close();
});

test("audio/canvas fallback renders only textContent and follows seeking", () => {
  const document = fakeDocument(), host = new Node("div"), audio = new Node("audio"); host.append(audio); document.body.append(host);
  const captions = new Captions({ video: audio, document, VTTCue: Cue }); captions.select({ language: "fr" }); captions.add([{ start: 1, end: 3, text: "<b>littéral</b>" }]);
  audio.currentTime = 2; audio.fire("timeupdate"); const overlay = host.children[1];
  assert.equal(overlay.textContent, "littéral"); assert.equal(overlay.hidden, false);
  captions.suspend(true); assert.equal(overlay.hidden, true);
  captions.select({ language: "en", title: "English" });
  assert.equal(host.children.length, 2, "reselect replaces instead of accumulating overlays"); assert.equal(overlay.parentElement, null);
  captions.select(null); assert.equal(host.children.length, 1, "off removes the owned overlay");
  captions.close(); assert.equal(host.children.length, 1);
});

test("invalid numeric entities remain literal text instead of becoming executable markup", () => {
  const document = fakeDocument(), video = new Node("video"); document.body.append(video);
  const captions = new Captions({ video, document, VTTCue: Cue }); captions.select({ language: "fr" });
  captions.add([{ start: 0, end: 1, text: "&#99999999;<i>format</i>" }]);
  assert.equal(video.textTracks[0].cues[0].text, "&amp;#99999999;format"); captions.close();
});

test("a single untagged subtitle follows the previous episode's untagged choice", () => {
  const choice = { language: "und", title: "Sous-titres" };
  const track = { id: 9 };
  assert.equal(matchTrack([track], choice), track);
  assert.equal(matchTrack([track, { id: 10, language: "und" }], choice), null);
  assert.equal(matchTrack([{ ...track, language: "fr" }], choice), null);
  assert.equal(matchTrack([track], { language: "en" }), null);
  assert.equal(matchTrack([track], { off: true }), null);
});
