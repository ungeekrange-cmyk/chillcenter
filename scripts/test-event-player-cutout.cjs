"use strict";
// CSS/native-surface contract only: no decoder, playback, provider or real server.
const assert = require("node:assert/strict");
const path = require("node:path");
const { chromium } = require(process.env.CHILL_PREVIEW_NODE_MODULES
  ? path.join(process.env.CHILL_PREVIEW_NODE_MODULES, "playwright") : "playwright");

(async () => {
  const browser = await chromium.launch({
    headless: true,
    executablePath: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    args: ["--disable-background-networking"],
  });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.route(/^https?:/, route => route.abort());
    await page.addInitScript(() => {
      window.fixtureAmbiance = "christmas";
      window.__TAURI__ = { core: { invoke: async (_, { command }) => {
        if (command.action === "boot") return {
          stage: "connected", server_id: "cutout-server", profile_id: "cutout-profile",
          profiles: [{ id: "cutout-profile", name: "Fixture", revision: 1 }],
          library: { revision: 1 }, theme: "dark", servers: [],
        };
        if (command.action === "catalog") return { items: [], has_more: false };
        if (command.action === "sync_catalog") return { changed: false };
        if (command.action === "resolve_appearance") return {
          ambiance: window.fixtureAmbiance,
          appearance: { mode: "manual", manual: window.fixtureAmbiance },
        };
        throw Error("Unexpected command: " + command.action);
      } } };
    });
    await page.goto("file://" + path.resolve(__dirname, "../ui/connected/index.html"));
    await page.evaluate(() => window.chillReady);
    const scene = () => page.locator(".workspace").evaluate(element => {
      const css = getComputedStyle(element, "::before");
      return { content: css.content, display: css.display, image: css.backgroundImage };
    });
    for (const ambiance of ["halloween", "christmas"]) {
      for (const tone of ["light", "dark"]) {
        await page.evaluate(async ([ambiance, tone]) => {
          window.fixtureAmbiance = ambiance;
          state.theme = tone;
          document.documentElement.dataset.theme = tone;
          await refreshAppearance();
        }, [ambiance, tone]);
        assert.ok((await scene()).image.includes(ambiance + ".png"));
        // Exercise the same lifecycle classes used by chillPlay/renderPlayer/hidePlayer.
        for (const fullscreen of [false, true]) {
          await page.evaluate(fullscreen => {
            document.body.classList.add("player-open");
            document.body.classList.toggle("player-fullscreen", fullscreen);
          }, fullscreen);
          const actual = await scene();
          assert.ok(actual.content === "none" || actual.display === "none",
            `${ambiance}/${tone}/${fullscreen ? "fullscreen" : "right-pane"}: decoration still paints over native video`);
          assert.equal(actual.image, "none", "No event image may occupy the native cutout");
          assert.equal(await page.locator(".workspace").evaluate(element => getComputedStyle(element).backgroundColor),
            "rgba(0, 0, 0, 0)", "Native cutout must remain transparent, not black-painted DOM");
        }
        await page.evaluate(() => document.body.classList.remove("player-open", "player-fullscreen"));
        assert.ok((await scene()).image.includes(ambiance + ".png"), "Closing restores decoration without changing the preference");
      }
    }
    assert.deepEqual(errors, []);
    console.log("PASS event/native cutout: 2 events × 2 tones × 2 player layouts; decoration restored on close. No playback or server touched.");
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
