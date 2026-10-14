#!/usr/bin/env node
// Builds the PWA from the Mac client's UI (ui/connected) plus the web bridge (web/src).
// ui/connected is never modified: the web client follows it at every build (ADR-0012).
// Usage: node scripts/build-web.mjs [outDir]   (default: web/dist)
import { createHash } from "node:crypto";
import { cpSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ui = join(root, "ui/connected");
const src = join(root, "web/src");
const out = resolve(process.argv[2] || join(root, "web/dist"));
const webScripts = ["json64.js", "watch-clock.js", "engines.js", "player-web.js", "bridge.js", "register-sw.js"];

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
cpSync(ui, out, {
  recursive: true,
  filter: path => !/(\/qa\.js|-qa\.js|\.test\.js)$/.test(path) && !path.endsWith("/tests"),
});
for (const name of [...webScripts, "web.css", "manifest.webmanifest"]) cpSync(join(src, name), join(out, name));

// Device wording: this is no longer "this Mac".
const patch = (file, from, to) => {
  const path = join(out, file);
  const text = readFileSync(path, "utf8");
  if (!text.includes(from)) throw new Error(`Texte attendu absent de ${file} : ${from}`);
  writeFileSync(path, text.replaceAll(from, to));
};
patch("appearance.js", "sur ce Mac uniquement", "sur cet appareil uniquement");

let html = readFileSync(join(out, "index.html"), "utf8");
const head = `<meta name="theme-color" content="#10151c"><link rel="manifest" href="manifest.webmanifest"><link rel="stylesheet" href="web.css">`
  + webScripts.map(name => `<script src="${name}"></script>`).join("");
if (!html.includes("<title>")) throw new Error("index.html sans <title>");
html = html.replace("<title>", head + "<title>").replace("Apparence de ce Mac", "Apparence de cet appareil").replace("Le Mac ne monte pas ce chemin.", "Cet appareil ne monte pas ce chemin.");
writeFileSync(join(out, "index.html"), html);

const files = [];
(function walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path); else files.push(relative(out, path).split("\\").join("/"));
  }
})(out);
files.sort();
const build = createHash("sha256");
for (const f of files) build.update(f).update(readFileSync(join(out, f)));
const id = build.digest("hex").slice(0, 12);
const shell = ["./", ...files.filter(f => f !== "sw.js" && !f.endsWith(".md"))];
writeFileSync(join(out, "sw.js"), readFileSync(join(src, "sw.js"), "utf8").replace("__BUILD__", id).replace("__SHELL__", JSON.stringify(shell)));
writeFileSync(join(out, "build.json"), JSON.stringify({ build: id, source: "ui/connected + web/src", files: files.length + 1 }, null, 2) + "\n");
console.log(`web: ${files.length + 1} fichiers, build ${id} -> ${out}`);
