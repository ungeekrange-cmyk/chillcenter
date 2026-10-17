"use strict";
const assert = require("node:assert/strict");
const test = require("node:test");
const { createController, phoneViewport, text, bindFullscreenButton, HINT_MS } = require("../src/mobile-playback.js");
const { WebPlayer } = require("../src/player-web.js");
function deferred() { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; }
function make({ phone = true, fullscreen = "resolve", lock = "resolve", native = false, canvas = false, lang = "en" } = {}) {
  const events = []; let actual = false, panelHidden = false;
  const labels = { textContent: "" }, retry = { textContent: "" };
  const hint = { hidden: true, querySelector: selector => selector === "button" ? retry : labels };
  const delayed = fullscreen === "deferred" ? deferred() : null;
  const video = canvas ? null : { webkitDisplayingFullscreen: false, webkitEnterFullscreen: native ? () => { events.push("native-enter"); video.webkitDisplayingFullscreen = true; } : undefined, webkitExitFullscreen: native ? () => { events.push("native-exit"); video.webkitDisplayingFullscreen = false; } : undefined };
  const panel = { get hidden() { return panelHidden; }, requestFullscreen() { events.push("request"); if (fullscreen === "throw") throw new Error("denied"); if (fullscreen === "reject") return Promise.reject(new Error("denied")); if (delayed) return delayed.promise.then(() => { actual = true; }); return Promise.resolve().then(() => { actual = true; }); } };
  if(fullscreen === "absent")delete panel.requestFullscreen;
  const doc = { documentElement: { lang }, get fullscreenElement() { return actual ? panel : null; }, exitFullscreen() { actual = false; events.push("exit"); return Promise.resolve(); }, getElementById: () => panel, querySelector: selector => selector.endsWith("canvas") ? (canvas ? {} : null) : video };
  const orientation = { lock() { events.push("lock"); return lock === "reject" ? Promise.reject(new Error("no")) : Promise.resolve(); }, unlock() { events.push("unlock"); } };
  const win = { innerWidth:390, innerHeight:844, matchMedia: () => ({ matches: phone }), screen: { orientation } };
  const controller = createController({ window: win, document: doc, panel: () => panel, video: () => video, hint: () => hint });
  return { controller, events, hint, labels, retry, delayed, win, set actual(v) { actual = v; }, set hidden(v) { panelHidden = v; }, get actual() { return actual; } };
}
test("phone predicate is standard and desktop never requests fullscreen", async () => { assert.equal(phoneViewport({ matchMedia: () => ({ matches: false }) }), false); const desktop = make({ phone: false }); assert.equal(await desktop.controller.requestFromGesture(), false); assert.deepEqual(desktop.events, []); });
test("a desktop fullscreenchange never locks orientation or shows mobile guidance", async () => { const desktop = make({ phone: false }); desktop.actual = true; await desktop.controller.onFullscreenChange(); assert.equal(desktop.events.includes("lock"), false); assert.equal(desktop.hint.hidden, true); });
test("request happens before lock and only reports success after actual fullscreen", async () => { const ok = make(); assert.equal(await ok.controller.requestFromGesture(), true); assert.deepEqual(ok.events.slice(0, 2), ["request", "lock"]); const noActual = make({ fullscreen: "reject", canvas: true }); assert.equal(await noActual.controller.requestFromGesture(), false); assert.equal(noActual.hint.hidden, false); });
test("late fullscreen resolution after close is exited and cannot lock or revive its hint", async () => { const race = make({ fullscreen: "deferred" }); const pending = race.controller.requestFromGesture(); race.hidden = true; race.controller.stop(); race.delayed.resolve(); assert.equal(await pending, false); await Promise.resolve(); assert.equal(race.actual, false); assert.ok(race.events.includes("exit")); assert.equal(race.hint.hidden, true); assert.equal(race.events.includes("lock"), false); });
test("a late fullscreenchange while the panel is hidden exits the panel", async () => { const race = make(); race.hidden = true; race.actual = true; await race.controller.onFullscreenChange(); await Promise.resolve(); assert.equal(race.actual, false); assert.ok(race.events.includes("exit")); });
test("orientation refusal is guidance, and native video fallback is real only when active", async () => { const refused = make({ lock: "reject" }); assert.equal(await refused.controller.requestFromGesture(), true); assert.equal(refused.hint.hidden, false); const native = make({ fullscreen: "reject", native: true }); assert.equal(await native.controller.requestFromGesture(), true); assert.ok(native.events.includes("native-enter")); native.controller.stop(); assert.ok(native.events.includes("native-exit")); });
test("browser exit unlocks without re-entering and labels follow language", async () => { const fr = make({ lang: "fr" }); await fr.controller.requestFromGesture(); fr.actual = false; await fr.controller.onFullscreenChange(); assert.ok(fr.events.includes("unlock")); assert.equal(text({ documentElement: { lang: "en" } }, "retry"), "Retry full screen"); });
test("WebPlayer status exposes the real browser fullscreen state", () => { const player = new WebPlayer({ isFullscreen: () => true }); player.info = { media: { title: "Test" }, preferences: {} }; player.mediaId = "m"; player.lease = {}; player.engine = { snapshot: () => ({ loaded: true, paused: false, tracks: [] }), externalSubtitles: () => [] }; assert.equal(player.view().native.fullscreen, true); });
test("rotation guidance disappears in landscape after fullscreen succeeds but orientation locking is refused", async () => {
 const phone=make({lock:"reject"});await phone.controller.requestFromGesture();
 assert.equal(phone.hint.hidden,false);
 phone.win.innerWidth=844;phone.win.innerHeight=390;phone.controller.refreshViewport();
 assert.equal(phone.hint.hidden,true);assert.deepEqual(phone.events.slice(0,2),["request","lock"]);
 phone.win.innerWidth=390;phone.win.innerHeight=844;phone.controller.refreshViewport();assert.equal(phone.hint.hidden,false);
 phone.controller.stop();phone.controller.refreshViewport();assert.equal(phone.hint.hidden,true);
});
test("unavailable canvas fullscreen is reported honestly, not as a rotation requirement", async () => {
 const phone=make({fullscreen:"absent",canvas:true});phone.win.innerWidth=844;phone.win.innerHeight=390;
 assert.equal(await phone.controller.requestFromGesture(),false);
 assert.equal(phone.controller.isActualFullscreen(),false);assert.equal(phone.hint.hidden,false);
 assert.match(phone.labels.textContent,/unavailable/);assert.equal(phone.retry.hidden,true);
 phone.controller.stop();assert.equal(phone.hint.hidden,true);
});
test("disabled panel fullscreen immediately uses iPhone video API, without waiting for a queued command",async()=>{
 let calls=0;const video={webkitDisplayingFullscreen:false,webkitEnterFullscreen(){calls++;this.webkitDisplayingFullscreen=true;},webkitExitFullscreen(){this.webkitDisplayingFullscreen=false;}};
 const controller=createController({window:{matchMedia:()=>({matches:true})},document:{fullscreenEnabled:false},panel:()=>({hidden:false,requestFullscreen(){throw Error("must not run");}}),video:()=>video});
 const pending=controller.setFullscreen(true);assert.equal(calls,1);assert.equal(await pending,true);
 await controller.setFullscreen(false);assert.equal(controller.isActualFullscreen(),false);
});
test("fullscreen click capture runs inside the touch stack and leaves desktop handler alone",()=>{
 let listener,capture,prevented=0,stopped=0,calls=[];let phone=true,actual=false;
 const button={addEventListener(type,fn,option){assert.equal(type,"click");listener=fn;capture=option;}};
 bindFullscreenButton(button,{isPhone:()=>phone,isActualFullscreen:()=>actual,setFullscreen:on=>calls.push(on)},()=>calls.push("active"));
 const event={preventDefault(){prevented++;},stopImmediatePropagation(){stopped++;}};
 listener(event);assert.equal(capture,true);assert.deepEqual(calls,[true,"active"]);
 actual=true;listener(event);assert.equal(calls[2],false);phone=false;listener(event);
 assert.equal(calls.length,4);assert.equal(prevented,2);assert.equal(stopped,2);
});
test("the video-only iPhone fullscreen fallback is requested synchronously in the gesture",async()=>{
 let calls=0;const video={webkitDisplayingFullscreen:false,webkitEnterFullscreen(){calls++;this.webkitDisplayingFullscreen=true;}};
 const controller=createController({window:{innerWidth:844,innerHeight:390,matchMedia:()=>({matches:true}),screen:{}},document:{},panel:()=>({hidden:false}),video:()=>video});
 const promise=controller.requestFromGesture();assert.equal(calls,1);assert.equal(await promise,true);
});
test("an unprepared iPhone video keeps a visible fresh-touch action in landscape and clears it on native entry",async()=>{
 let video=null,calls=0;const label={textContent:""},button={};
 const hint={hidden:true,querySelector:selector=>selector==="button"?button:label};
 const controller=createController({window:{innerWidth:844,innerHeight:390,matchMedia:()=>({matches:true})},document:{fullscreenEnabled:false},panel:()=>({hidden:false}),video:()=>video,hint:()=>hint});
 assert.equal(await controller.requestFromGesture(),false);assert.equal(hint.hidden,false);assert.equal(button.hidden,false);assert.match(label.textContent,/Tap to open/);
 video={webkitDisplayingFullscreen:false,webkitEnterFullscreen(){calls++;}};
 controller.refreshMedia();const pending=controller.requestFromGesture();assert.equal(calls,1);
 assert.equal(await pending,false,"an asynchronous native request is not reported as success yet");
 video.webkitDisplayingFullscreen=true;await controller.onFullscreenChange();
 assert.equal(controller.isActualFullscreen(),true);assert.equal(hint.hidden,true);
});

test("fullscreen notice expires once at 2.5s despite status, orientation and language refreshes",async()=>{
 let now=0,next=0;const timers=new Map(),label={},retry={},hint={hidden:true,querySelector:s=>s==="button"?retry:label};
 const tick=ms=>{now+=ms;for(const [id,t] of timers)if(t.at<=now){timers.delete(id);t.fn();}};
 const ctl=createController({window:{innerWidth:844,innerHeight:390,matchMedia:()=>({matches:true})},document:{querySelector:s=>s.endsWith("canvas")?{}:null},panel:()=>({hidden:false}),hint:()=>hint,setTimeout:(fn,ms)=>{timers.set(++next,{fn,at:now+ms});return next;},clearTimeout:id=>timers.delete(id)});
 assert.equal(HINT_MS,2500);await ctl.requestFromGesture();assert.equal(hint.hidden,false);
 tick(1000);ctl.refreshMedia();ctl.refreshViewport();ctl.refreshLanguage();
 tick(1499);assert.equal(hint.hidden,false);tick(1);assert.equal(hint.hidden,true);
 for(let i=0;i<8;i++){tick(500);ctl.refreshMedia();ctl.refreshViewport();ctl.refreshLanguage();assert.equal(hint.hidden,true);}
 await ctl.requestFromGesture();assert.equal(hint.hidden,false,"only a new user attempt rearms the notice");
 ctl.stop();assert.equal(timers.size,0);tick(4000);assert.equal(hint.hidden,true);
});
test("WebKit presentation fullscreen is tried synchronously when enterFullscreen refuses",async()=>{
 const calls=[];const video={webkitPresentationMode:"inline",webkitEnterFullscreen(){calls.push("enter");throw Error("refused");},webkitSupportsPresentationMode:mode=>mode==="fullscreen",webkitSetPresentationMode(mode){calls.push(mode);this.webkitPresentationMode=mode;}};
 const ctl=createController({window:{matchMedia:()=>({matches:true})},document:{fullscreenEnabled:false},panel:()=>({hidden:false}),video:()=>video});
 const pending=ctl.requestFromGesture();assert.deepEqual(calls,["enter","fullscreen"]);assert.equal(await pending,true);
 ctl.stop();assert.equal(video.webkitPresentationMode,"inline");assert.equal(ctl.isActualFullscreen(),false);
});
test("legacy WebKit video spelling works and a standard request asks to hide browser navigation",async()=>{
 const video={webkitDisplayingFullscreen:false,webkitEnterFullScreen(){this.webkitDisplayingFullscreen=true;},webkitExitFullScreen(){this.webkitDisplayingFullscreen=false;}};
 const ctl=createController({window:{matchMedia:()=>({matches:true})},document:{},panel:()=>({hidden:false}),video:()=>video});
 assert.equal(await ctl.requestFromGesture(),true);ctl.stop();assert.equal(video.webkitDisplayingFullscreen,false);
 let options;const panel={hidden:false,requestFullscreen(value){options=value;}};
 const standard=createController({window:{matchMedia:()=>({matches:true})},document:{},panel:()=>panel});
 await standard.requestFromGesture();assert.deepEqual(options,{navigationUI:"hide"});standard.stop();
});
