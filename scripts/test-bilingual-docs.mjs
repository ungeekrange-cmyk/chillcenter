import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { assertPublicText } from './build-public-site.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const pairs = [
  ['README.md', 'README.en.md'],
  ['CONTRIBUTING.md', 'CONTRIBUTING.en.md'],
  ['SECURITY.md', 'SECURITY.en.md'],
  ['docs/README.md', 'docs/README.en.md'],
  ['docs/architecture/OVERVIEW.md', 'docs/architecture/OVERVIEW.en.md'],
  ['docs/api/INTEGRATION-FR.md', 'docs/api/INTEGRATION-EN.md'],
  ['docs/updates/GUIDE-FR.md', 'docs/updates/GUIDE-EN.md'],
  ['themes/README.md', 'themes/README.en.md'],
  ['docs/releases/REL-V0.6.md', 'docs/releases/REL-V0.6.en.md'],
  ['docs/releases/REL-WEB-MOBILE-001.md', 'docs/releases/REL-WEB-MOBILE-001.en.md'],
  ['docs/releases/REL-WEB-MOBILE-002.md', 'docs/releases/REL-WEB-MOBILE-002.en.md'],
  ['docs/releases/REL-WEB-MOBILE-003.md', 'docs/releases/REL-WEB-MOBILE-003.en.md'],
  ['docs/releases/REL-WEB-MOBILE-004.md', 'docs/releases/REL-WEB-MOBILE-004.en.md'],
  ['docs/architecture/decisions/ADR-0028-web-mp3-fallback.md', 'docs/architecture/decisions/ADR-0028-web-mp3-fallback.en.md'],
  ['docs/architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.md', 'docs/architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.en.md'],
  ['docs/architecture/decisions/ADR-0026-mobile-web-presentation.md', 'docs/architecture/decisions/ADR-0026-mobile-web-presentation.en.md'],
  ['docs/publication/GITHUB-REPOSITORY.md', 'docs/publication/GITHUB-REPOSITORY.en.md'],
  ['docs/publication/GITHUB-PAGES.md', 'docs/publication/GITHUB-PAGES.en.md'],
];
const read = file => readFile(path.join(root, file), 'utf8');
const unique = values => [...new Set(values)].sort();

// These checks guard navigation and machine-readable values, not prose accuracy.
// Human review still owns equivalent meaning, safety caveats and historical scope.
for (const [fr, en] of pairs) {
  test(`FR/EN navigation, local links and privacy: ${fr}`, async () => {
    for (const [file, counterpart, language] of [[fr, en, 'Français'], [en, fr, 'English']]) {
      const text = await read(file);
      assert.ok(text.includes(`**${language}**`), `${file}: missing current-language marker`);
      const sibling = path.posix.relative(path.posix.dirname(file), counterpart);
      assert.ok(text.includes(`](${sibling})`), `${file}: missing counterpart link`);
      assert.equal((text.match(/^```/gm) ?? []).length % 2, 0, `${file}: unclosed code fence`);
      assertPublicText(text, file);
      assert.doesNotMatch(text, /-----BEGIN CERTIFICATE-----/, `${file}: embedded certificate`);
      for (const [, target] of text.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)) {
        if (/^[a-z][a-z\d+.-]*:/i.test(target) || target.startsWith('#')) continue;
        const resolved = path.resolve(root, path.dirname(file), target.split('#')[0]);
        assert.ok(resolved.startsWith(root), `${file}: link escapes repository`);
        assert.ok((await stat(resolved)).isFile(), `${file}: broken local link`);
      }
    }
  });
}

function examples(text) {
  return [...text.matchAll(/^```([^\n]*)\n([\s\S]*?)^```/gm)].map(([, language, body]) => [
    language,
    // Only the explanatory Python comment is translated; executable examples stay identical.
    body.split('\n').map(line => language === 'python' ? line.replace(/\s+#.*$/, '') : line).join('\n'),
  ]);
}

test('API examples and endpoint references are identical in both languages', async () => {
  const fr = await read('docs/api/INTEGRATION-FR.md');
  const en = await read('docs/api/INTEGRATION-EN.md');
  assert.deepEqual(examples(fr), examples(en));
  const endpoints = text => unique([...text.matchAll(/`((?:GET|POST|PUT|DELETE|PATCH) \/[^`]+)`/g)].map(match => match[1]));
  assert.ok(endpoints(fr).length >= 20, 'Endpoint inventory must not silently become empty');
  assert.deepEqual(endpoints(fr), endpoints(en));
});

test('both API guides retain compatibility, lease safety and synchronization limits', async () => {
  for (const file of ['docs/api/INTEGRATION-FR.md', 'docs/api/INTEGRATION-EN.md']) {
    const text = await read(file);
    for (const literal of [
      '/api/2026-10-14', '/api/v1', 'Sec-Fetch-Site: same-origin',
      'x-chill-confirm-delete', '?selector=key', '?selector=title',
      'additional_video_roots', 'known_revision', 'deleted_ids', 'season_metadata',
      'SYNC_SNAPSHOT_NOT_FOUND', 'SYNC_SNAPSHOT_CHANGED', 'IMPORT_PLAN_STALE',
      'MEDIA_FILE_MISSING', 'MEDIA_UNAVAILABLE', 'LEASE_INVALID',
      'IMPORT_MULTIPLE_LIBRARY_LOCATIONS_UNSUPPORTED', 'ADR0022',
      'uint64', '2^53 - 1', 'position_ms == duration_ms',
    ]) assert.ok(text.includes(literal), `${file}: missing ${literal}`);
  }
});

test('v0.6 translations retain exact provenance, executable hashes and open bug IDs', async () => {
  const fr = await read('docs/releases/REL-V0.6.md');
  const en = await read('docs/releases/REL-V0.6.en.md');
  for (const pattern of [/\b[a-f0-9]{40,64}\b/g, /\bBUG\d{4}\b/g]) {
    assert.deepEqual(unique(fr.match(pattern) ?? []), unique(en.match(pattern) ?? []));
  }
  for (const text of [fr, en]) {
    assert.ok(text.includes('0.6.0') && text.includes('130/130'));
    assert.ok(text.includes('Chill Center-v0.6.app'));
    assert.ok(text.includes('7eb9dd979f473f1dc09309bdc314fe93c9890d8eabf6a42203c7a5232787e3f7'));
  }
});

test('both documentation indexes list every translated reference pair', async () => {
  for (const index of ['docs/README.md', 'docs/README.en.md']) {
    const text = await read(index);
    for (const pair of pairs.filter(([fr]) => fr !== 'docs/README.md')) {
      for (const file of pair) {
        assert.ok(text.includes(`](${path.posix.relative('docs', file)})`), `${index}: missing ${file}`);
      }
    }
  }
});
