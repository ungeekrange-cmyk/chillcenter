// Chromium proof for the built, installable web shell.  It only serves files
// from web/dist on loopback and never contacts a household server or profile.
const assert = require("node:assert/strict");
const fs = require("node:fs"), fsp = require("node:fs/promises"), http = require("node:http"), path = require("node:path");
const { chromium } = require(process.env.CHILL_PREVIEW_NODE_MODULES ? path.join(process.env.CHILL_PREVIEW_NODE_MODULES, "playwright") : "playwright");
const root = path.resolve(__dirname, "../web/dist");
const chrome = process.env.CHILL_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

function type(file) { return ({ ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".webmanifest": "application/manifest+json", ".png": "image/png", ".svg": "image/svg+xml", ".wasm": "application/wasm" })[path.extname(file)] || "application/octet-stream"; }
function pngSize(bytes) { assert.equal(bytes.toString("ascii", 1, 4), "PNG"); return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) }; }
function listen(server) { return new Promise(resolve => server.listen(0, "127.0.0.1", resolve)); }
function close(server) { return new Promise(resolve => server.close(resolve)); }
async function until(check, message) {
  const end = Date.now() + 20000;
  while (Date.now() < end) { const value = await check(); if (value) return value; await new Promise(resolve => setTimeout(resolve, 100)); }
  throw new Error(message);
}

(async () => {
  const manifest = JSON.parse(await fsp.readFile(path.join(root, "manifest.webmanifest"), "utf8"));
  const icons = new Map(manifest.icons.map(icon => [icon.sizes, icon]));
  for (const size of ["192x192", "512x512"]) {
    const icon = icons.get(size); assert(icon, `manifest lacks ${size} icon`);
    const pixel = pngSize(await fsp.readFile(path.join(root, icon.src)));
    assert.deepEqual(pixel, { width: Number(size.slice(0, 3)), height: Number(size.slice(4, 7)) });
  }
  assert([...icons.values()].some(icon => String(icon.purpose || "").includes("maskable")), "manifest lacks a maskable icon");
  const index = await fsp.readFile(path.join(root, "index.html"), "utf8");
  assert.match(index, /apple-touch-icon/); assert.match(index, /apple-mobile-web-app-capable/);
  assert.match(index, /pwa-install\.js\?v=/); assert.match(index, /pwa-install\.css\?v=/);
  const worker = await fsp.readFile(path.join(root, "sw.js"), "utf8");
  assert.match(worker, /navigate/); assert.doesNotMatch(worker, /"themes\//); assert.doesNotMatch(worker, /"vendor\//);
  assert.match(index, /register-sw\.js\?v=/);

  let marker = "first";
  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url, "http://fixture.invalid");
    if (url.pathname.startsWith("/api/") || url.pathname.startsWith("/themes/") || url.pathname.includes("decorations")) { res.statusCode = 404; return res.end("fixture absent"); }
    if (url.pathname === "/" || url.pathname === "/index.html") {
      res.setHeader("content-type", "text/html");
      return res.end((await fsp.readFile(path.join(root, "index.html"), "utf8")).replace("</body>", `<p id="fixture-marker">${marker}</p></body>`));
    }
    if (url.pathname === "/pwa-fixture.html") {
      res.setHeader("content-type", "text/html");
      return res.end("<!doctype html><html lang=\"en\"><head><link rel=\"stylesheet\" href=\"/pwa-install.css\"></head><body><header class=\"app-topbar\"></header><script src=\"/pwa-install.js\"></script></body></html>");
    }
    const file = path.resolve(root, decodeURIComponent(url.pathname.slice(1)));
    if (!file.startsWith(root + path.sep)) { res.statusCode = 404; return res.end(); }
    try { res.setHeader("content-type", type(file)); res.end(await fsp.readFile(file)); }
    catch { res.statusCode = 404; res.end("missing"); }
  });
  await listen(server);
  const origin = `http://127.0.0.1:${server.address().port}`;
  let browser;
  try {
    browser = await chromium.launch({ executablePath: chrome, headless: true, args: ["--disable-background-networking", "--no-first-run"] });
    const context = await browser.newContext();
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await page.goto(origin + "/", { waitUntil: "domcontentloaded" });
    await until(() => page.evaluate(async () => (await navigator.serviceWorker.ready).active && true), "service worker was not ready in 20s");
    await page.reload({ waitUntil: "domcontentloaded" });
    await until(() => page.evaluate(() => Boolean(navigator.serviceWorker.controller)), "service worker did not control page");
    const appManifest = await cdp.send("Page.getAppManifest");
    assert(appManifest.manifest || appManifest.url, "CDP did not expose app manifest");
    const installability = await cdp.send("Page.getInstallabilityErrors");
    // Playwright's disposable context is incognito, which Chrome accurately
    // reports as non-installable. It is not an app-manifest defect; reject
    // every other installability problem the browser identifies.
    const relevantInstallability = (installability.installabilityErrors || []).filter(error => error.errorId !== "in-incognito");
    assert.deepEqual(relevantInstallability, [], JSON.stringify(installability));
    const cached = await page.evaluate(async () => {
      const names = await caches.keys(); const values = await Promise.all(names.map(async name => ({ name, urls: (await (await caches.open(name)).keys()).map(request => new URL(request.url).pathname) })));
      return values;
    });
    assert(cached.some(cache => cache.urls.includes("/index.html") || cache.urls.includes("/")), "shell was not cached");
    assert(!cached.some(cache => cache.urls.some(url => url.startsWith("/api/") || url.includes("film"))), "API or film entered shell cache");
    marker = "second";
    await page.goto(origin + "/", { waitUntil: "domcontentloaded" });
    assert.equal(await page.locator("#fixture-marker").textContent(), "second", "controlled navigation served cached old HTML");

    await page.goto(origin + "/pwa-fixture.html");
    await page.evaluate(() => {
      const prompt = new Event("beforeinstallprompt", { cancelable: true }); let calls = 0;
      Object.defineProperties(prompt, { prompt: { value: () => { calls++; return Promise.resolve(); } }, userChoice: { value: Promise.resolve({ outcome: "dismissed" }) } });
      window.__pwaPromptCalls = () => calls; window.dispatchEvent(prompt);
    });
    await page.getByRole("button", { name: "Install app" }).click();
    assert.equal(await page.evaluate(() => window.__pwaPromptCalls()), 1, "install prompt was not trusted-button initiated");
    await page.evaluate(() => { const prompt = new Event("beforeinstallprompt", { cancelable: true }); Object.defineProperties(prompt, { prompt: { value: () => Promise.reject(new Error("fixture rejection")) }, userChoice: { value: Promise.resolve({ outcome: "dismissed" }) } }); window.dispatchEvent(prompt); });
    await page.getByRole("button", { name: "Install app" }).click();
    await page.getByRole("dialog").waitFor();
    await page.keyboard.press("Escape");
    await until(() => page.evaluate(() => document.activeElement?.id === "pwa-install"), "dialog did not return focus");
    await page.evaluate(() => window.dispatchEvent(new Event("appinstalled")));
    assert.equal(await page.locator("#pwa-install").isHidden(), true, "installed app control remains visible");

    const ios = await browser.newContext({ userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Version/17.0 Mobile/15E148 Safari/604.1" });
    const iosPage = await ios.newPage(); await iosPage.goto(origin + "/pwa-fixture.html");
    await iosPage.getByRole("button", { name: "Install app" }).click();
    await assert.doesNotReject(() => iosPage.getByText("Share, then Add to Home Screen").waitFor());
    await ios.close();
    console.log("PASS PWA: manifest PNG/maskable icons, Apple metadata, secure-loopback service worker and installability, shell-only cache, network-fresh controlled navigation, trusted prompt/retry/error dialog, keyboard focus, installed hide, iOS instructions.");
  } finally { await browser?.close(); await close(server); }
})().catch(error => { console.error(error); process.exitCode = 1; });
