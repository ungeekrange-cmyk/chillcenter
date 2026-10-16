#!/usr/bin/env node
// Publish a reviewed subset, never the repository root or the internal docs tree.
import { createHash } from 'node:crypto';
import { lstat, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = fileURLToPath(new URL('../', import.meta.url));
export const PUBLIC_FILES = Object.freeze({
  'index.html': 'site/index.html',
  'api.html': 'site/api.html',
  '404.html': 'site/404.html',
  'styles.css': 'site/styles.css',
  '.nojekyll': 'site/.nojekyll',
  'assets/logo.svg': 'icons/branding/vector/default.svg',
  'api/openapi-2026-10-14.json': 'docs/api/openapi-2026-10-14.json',
  'LICENSE.txt': 'LICENSE',
});
const manifestName = 'manifest.json';
const privacyRules = [
  ['personal filesystem path', /\/(?:Users|home)\/|\/private\/(?:tmp|var)\/|\/srv\/volumes\//i],
  ['private network address', /\b(?:192\.168\.\d{1,3}\.\d{1,3}|10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})\b/],
  ['private key', /-----BEGIN (?:[A-Z]+ )*PRIVATE KEY-----/],
  ['credential-shaped value', /\b(?:gh[pousr]_[A-Za-z0-9]{25,}|github_pat_[A-Za-z0-9_]{25,}|sk-[A-Za-z0-9_-]{24,})\b/],
];

export function assertPublicText(text, name) {
  for (const [label, rule] of privacyRules) {
    // Report the category and file only: diagnostics must not echo a secret.
    if (rule.test(text)) throw new Error(`${name}: rejected ${label}`);
  }
  if (text.includes('\0')) throw new Error(`${name}: unexpected binary content`);
}

async function statIfPresent(file) {
  try { return await lstat(file); }
  catch (error) { if (error.code === 'ENOENT') return null; throw error; }
}

async function rejectSymlinkComponents(root, relative) {
  let current = root;
  for (const segment of relative.split('/')) {
    current = path.join(current, segment);
    const stat = await statIfPresent(current);
    if (stat?.isSymbolicLink()) throw new Error(`Symlink refused: ${relative}`);
  }
}

async function validateExistingOutput(root, output) {
  const allowed = new Set([...Object.keys(PUBLIC_FILES), manifestName]);
  async function walk(directory, prefix = '') {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const name = prefix + entry.name;
      if (entry.isSymbolicLink()) throw new Error(`Output symlink refused: ${name}`);
      if (entry.isDirectory()) {
        if (![...allowed].some(file => file.startsWith(name + '/'))) {
          throw new Error(`Unmanaged output directory: ${name}`);
        }
        await walk(path.join(directory, entry.name), name + '/');
      } else if (!entry.isFile() || !allowed.has(name)) {
        throw new Error(`Unmanaged output file: ${name}`);
      }
    }
  }
  await rejectSymlinkComponents(root, 'dist/public');
  const stat = await statIfPresent(output);
  if (stat && !stat.isDirectory()) throw new Error('dist/public must be a directory');
  if (stat) await walk(output);
}

// The CLI has a fixed destination. The root argument is only a seam for fixture tests.
export async function buildPublicSite(root = projectRoot) {
  const output = path.join(root, 'dist/public');
  await validateExistingOutput(root, output);
  const staged = new Map();
  for (const [destination, source] of Object.entries(PUBLIC_FILES)) {
    await rejectSymlinkComponents(root, source);
    if (!(await lstat(path.join(root, source))).isFile()) throw new Error(`Not a file: ${source}`);
    const bytes = await readFile(path.join(root, source));
    assertPublicText(bytes.toString('utf8'), source);
    staged.set(destination, bytes);
  }
  const api = JSON.parse(staged.get('api/openapi-2026-10-14.json').toString('utf8'));
  if (!api.openapi || !api.paths || !api.servers?.some(server => server.url.endsWith('/api/2026-10-14'))) {
    throw new Error('Unexpected public API contract');
  }
  // All inputs pass before writing; no recursive deletion or catch-all copy is used.
  const manifest = { format: 1, files: {} };
  for (const [destination, bytes] of staged) {
    const target = path.join(output, destination);
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, bytes);
    manifest.files[destination] = createHash('sha256').update(bytes).digest('hex');
  }
  await writeFile(path.join(output, manifestName), JSON.stringify(manifest, null, 2) + '\n');
  return manifest;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 2) throw new Error('No arguments accepted; output is fixed to dist/public');
    const manifest = await buildPublicSite();
    console.log(`Public site prepared: ${Object.keys(manifest.files).length} allowlisted files + manifest. Nothing published.`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
