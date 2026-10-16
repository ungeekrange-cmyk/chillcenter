import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cp, lstat, mkdir, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { assertPublicText, buildPublicSite, PUBLIC_FILES } from './build-public-site.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
async function fixture(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'chill-public-site-test-'));
  // Cleanup owns only this test's freshly created directory, never a supplied path.
  t.after(() => rm(directory, { recursive: true, force: true }));
  for (const source of Object.values(PUBLIC_FILES)) {
    await mkdir(path.dirname(path.join(directory, source)), { recursive: true });
    await cp(path.join(root, source), path.join(directory, source));
  }
  return directory;
}

test('public build is deterministic and contains only the reviewed allowlist', async t => {
  const directory = await fixture(t);
  await writeFile(path.join(directory, 'private-notes.txt'), 'Not a publication input');
  const first = await buildPublicSite(directory);
  assert.deepEqual(await buildPublicSite(directory), first);
  const output = path.join(directory, 'dist/public');
  const files = [];
  async function walk(relative = '') {
    for (const entry of await readdir(path.join(output, relative), { withFileTypes: true })) {
      const name = path.posix.join(relative, entry.name);
      if (entry.isDirectory()) await walk(name); else files.push(name);
    }
  }
  await walk();
  assert.deepEqual(files.sort(), [...Object.keys(PUBLIC_FILES), 'manifest.json'].sort());
  for (const [name, hash] of Object.entries(first.files)) {
    assert.equal(createHash('sha256').update(await readFile(path.join(output, name))).digest('hex'), hash);
  }
});

test('privacy guard refuses private data without echoing its value', () => {
  for (const sample of ['/Users/person/private', '/private/tmp/test', '/home/person/data', '192.168.4.22', '10.3.2.1', '172.31.2.4', '-----BEGIN PRIVATE KEY-----', 'ghp_' + 'a'.repeat(30), 'github_pat_' + 'a'.repeat(30), 'sk-' + 'a'.repeat(30)]) {
    assert.throws(() => assertPublicText(sample, 'fixture.html'), error => {
      assert.match(error.message, /fixture.html: rejected/);
      assert.ok(!error.message.includes(sample));
      return true;
    });
  }
  assertPublicText('https://chillcenter.local:8443/api/2026-10-14', 'example');
});

test('rejected source leaves an existing artifact unchanged', async t => {
  const directory = await fixture(t);
  await buildPublicSite(directory);
  const target = path.join(directory, 'dist/public/index.html');
  const before = await readFile(target, 'utf8');
  await writeFile(path.join(directory, 'site/index.html'), '192.168.3.25');
  await assert.rejects(buildPublicSite(directory), /private network/);
  assert.equal(await readFile(target, 'utf8'), before);
});

test('unmanaged output is preserved and prevents accidental publication', async t => {
  const directory = await fixture(t);
  await mkdir(path.join(directory, 'dist/public'), { recursive: true });
  const unexpected = path.join(directory, 'dist/public/notes.txt');
  await writeFile(unexpected, 'keep me');
  await assert.rejects(buildPublicSite(directory), /Unmanaged output/);
  assert.equal(await readFile(unexpected, 'utf8'), 'keep me');
});

test('symlinks in sources and output ancestors are refused', async t => {
  const directory = await fixture(t);
  await symlink(path.join(directory, 'site'), path.join(directory, 'dist'));
  await assert.rejects(buildPublicSite(directory), /Symlink/);
  assert.ok((await lstat(path.join(directory, 'site/index.html'))).isFile());
  const second = await fixture(t);
  const source = path.join(second, 'site/index.html');
  await rm(source);
  await symlink(path.join(second, 'site/api.html'), source);
  await assert.rejects(buildPublicSite(second), /Symlink/);
});

test('HTML local links resolve inside the artifact; no tracking or active external resource', async t => {
  const directory = await fixture(t);
  await buildPublicSite(directory);
  const output = path.join(directory, 'dist/public');
  for (const file of ['index.html', 'api.html', '404.html']) {
    const html = await readFile(path.join(output, file), 'utf8');
    assert.match(html, /<html lang="fr">/);
    assert.match(html, /name="viewport"/);
    assert.doesNotMatch(html, /<script\b|<iframe\b|\son\w+=/i);
    for (const [, attribute, value] of html.matchAll(/\b(href|src)="([^"]+)"/g)) {
      if (value.startsWith('https://')) {
        assert.equal(attribute, 'href');
        assert.ok(['github.com', 'ungeekrange-cmyk.github.io'].includes(new URL(value).hostname));
        continue;
      }
      assert.ok(!value.startsWith('/') && !value.includes('..'));
      const [relative, anchor] = value.split('#');
      const target = relative || file;
      assert.ok((await lstat(path.join(output, target))).isFile(), value);
      if (anchor) assert.ok((await readFile(path.join(output, target), 'utf8')).includes(`id="${anchor}"`));
    }
  }
});

test('workflow remains manual, opt-in, default-branch-only and uploads only dist/public', async () => {
  const workflow = await readFile(path.join(root, '.github/workflows/pages.yml'), 'utf8');
  assert.match(workflow, /workflow_dispatch:/);
  assert.doesNotMatch(workflow, /^\s+(?:push|pull_request|schedule):/m);
  assert.match(workflow, /inputs\.publish/);
  assert.match(workflow, /github\.event\.repository\.default_branch/);
  assert.match(workflow, /path: dist\/public/);
  assert.match(workflow, /persist-credentials: false/);
  for (const [, action] of workflow.matchAll(/uses: ([^\s]+)/g)) assert.match(action, /^actions\/[\w-]+@[a-f0-9]{40}$/);
  assert.doesNotMatch(workflow, /secrets\.|enablement: true|write-all/);
});
