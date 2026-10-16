"use strict";
// Optional local visual proof; Playwright is supplied by the existing test environment.
const assert = require("node:assert/strict");
const { mkdir } = require("node:fs/promises");
const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { chromium } = require(process.env.CHILL_PREVIEW_NODE_MODULES
  ? path.join(process.env.CHILL_PREVIEW_NODE_MODULES, "playwright") : "playwright");

(async () => {
  const evidence = process.env.CHILL_PUBLIC_SITE_EVIDENCE;
  if (!evidence || !path.isAbsolute(evidence)) throw Error("Set an absolute CHILL_PUBLIC_SITE_EVIDENCE directory");
  await mkdir(evidence, { recursive: true });
  const browser = await chromium.launch({
    headless: true,
    executablePath: process.env.CHILL_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    args: ["--disable-background-networking"],
  });
  try {
    const page = await browser.newPage();
    const errors = [], external = [];
    page.on("pageerror", error => errors.push(error.message));
    page.on("request", request => {
      if (/^https?:/.test(request.url())) external.push(request.url());
    });
    for (const tone of ["light", "dark"]) {
      await page.emulateMedia({ colorScheme: tone, reducedMotion: "reduce" });
      for (const width of [320, 390, 760, 1440]) {
        await page.setViewportSize({ width, height: 1000 });
        for (const file of ["index.html", "api.html", "404.html"]) {
          await page.goto(pathToFileURL(path.resolve(__dirname, "../dist/public/", file)).href);
          await page.waitForLoadState("load");
          assert.equal(await page.locator("h1").count(), 1);
          assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${tone}/${width}/${file}: overflow`);
          assert.ok(await page.locator("img").evaluateAll(images => images.every(image => image.complete && image.naturalWidth > 0)));
          if (file === "index.html" && [390, 1440].includes(width)) {
            await page.screenshot({ path: path.join(evidence, `${tone}-${width}.png`), fullPage: true });
          }
        }
      }
    }
    await page.goto(pathToFileURL(path.resolve(__dirname, "../dist/public/index.html")).href);
    await page.keyboard.press("Tab");
    assert.equal(await page.evaluate(() => document.activeElement.textContent), "Aller au contenu");
    await page.keyboard.press("Enter");
    assert.equal(new URL(page.url()).hash, "#contenu");
    await page.getByRole("link", { name: "Explorer l’API", exact: true }).click();
    assert.match(page.url(), /api\.html$/);
    assert.deepEqual(errors, []);
    assert.deepEqual(external, [], "the public presentation loads no remote resources");
    console.log("PASS public site: 24 page/tone/width combinations, local images, keyboard skip, API navigation, no external requests.");
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
