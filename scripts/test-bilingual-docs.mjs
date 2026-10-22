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
  ['docs/implementation/COMPONENT-UPDATES-20261019.md', 'docs/implementation/COMPONENT-UPDATES-20261019.en.md'],
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

test('README distinguishes source versions from immutable published download packages', async () => {
  const versions = JSON.parse(await read('versions.json'));
  // Public receipts verified against GitHub v0.7.3, not inferred from a bump.
  // Keep independent component versions; unchanged themes stay at 0.7.1.
  const published = {mac:'0.7.3',server:'0.7.3',web:'0.7.3',themes:'0.7.1'};
  const packages = [
    `Chill-Center-${published.mac}-macos-arm64.dmg`,
    `Chill-Center-server-${published.server}-linux-aarch64.tar.gz`,
    `Chill-Center-web-${published.web}-install.zip`,
    `Chill-Center-themes-${published.themes}-install.zip`,
    `Chill-Center-${published.mac}-corresponding-sources.tar.gz`,
  ];
  for (const file of ['README.md', 'README.en.md']) {
    const text = await read(file);
    for (const name of packages) assert.ok(text.includes(`/${name})`), `${file}: stale package ${name}`);
    assert.ok(text.includes(`/releases/tag/v${published.mac})`), `${file}: stale release link`);
    const downloads = [...text.matchAll(/https:\/\/github\.com\/ungeekrange-cmyk\/chillcenter\/releases\/download\/([^/]+)\/([^\s)]+)/g)];
    assert.deepEqual([...new Set(downloads.map(match => match[1]))], ['v0.7.3'], `${file}: obsolete download release`);
    assert.deepEqual([...new Set(downloads.map(match => match[2]))].sort(), [...packages].sort(), `${file}: unexpected package inventory`);
    assert.ok(text.includes('GPL-3.0-or-later'), `${file}: missing player license scope`);
    for (const component of ['mac','server','web','themes']) {
      const label = {mac:'Mac',server:file==='README.md'?'serveur':'server',web:'web',themes:file==='README.md'?'thèmes':'themes'}[component];
      assert.ok(text.includes(`${label} ${versions[component]}`), `${file}: stale ${component} source version`);
      if (versions[component]!==published[component]) assert.ok(!text.includes(`/releases/download/v${versions[component]}/`),`${file}: unpublished download link`);
    }
  }
});

test('source component versions agree with crates, lockfile, Tauri, plist and release reservations', async () => {
  const versions=JSON.parse(await read('versions.json')), reservations=JSON.parse(await read('updates/sequences.json'));
  const tauri=JSON.parse(await read('apps/desktop/tauri.conf.json')),plist=await read('apps/desktop/Info.plist'),lock=await read('Cargo.lock');
  assert.equal(tauri.version,versions.mac);
  assert.equal(/<key>CFBundleShortVersionString<\/key><string>([^<]+)<\/string>/.exec(plist)?.[1],versions.mac);
  assert.equal(tauri.bundle.macOS.minimumSystemVersion,'15.0');
  for (const [component,manifest,name] of [['mac','apps/desktop/Cargo.toml','chillcenter-desktop'],['server','crates/chill-server/Cargo.toml','chill-server']]) {
    assert.equal(/^version = "([^"]+)"$/m.exec(await read(manifest))?.[1],versions[component]);
    assert.ok(lock.includes(`name = "${name}"\nversion = "${versions[component]}"`));
  }
  for (const component of ['mac','server','web','themes']) {
    assert.equal(reservations[component].version,versions[component]);
    assert.equal(reservations[component].sequence,component==='themes'?2:4);
  }
});

test('platform prerequisites distinguish published bundles, untested systems and server hosts', async () => {
  for (const [file, pending] of [
    ['README.md', 'non réalisés'], ['README.en.md', 'not yet been tested'],
    ['docs/updates/GUIDE-FR.md', 'non réalisés'],
    ['docs/updates/GUIDE-EN.md', 'not yet been tested'],
  ]) {
    const text = await read(file);
    for (const literal of ['macOS 27', 'macOS 15 Sequoia', '13.0', 'ARM64',
      'glibc 2.28', 'Debian 13', 'systemd', 'Python 3.10+', 'Minisign', 'curl', pending]) {
      assert.ok(text.includes(literal), `${file}: missing prerequisite or scope ${literal}`);
    }
    assert.doesNotMatch(text, /glibc 2\.31/, `${file}: stale binary floor`);
  }
  for (const file of ['scripts/install-server.sh', 'scripts/package-linux.sh']) {
    const comments = (await read(file)).split('\n').filter(line => line.startsWith('#')).join('\n');
    for (const literal of ['ARM64', '2.28', 'Debian 13', 'README.en.md#installation-prerequisites']) {
      assert.ok(comments.includes(literal), `${file}: missing prerequisite comment ${literal}`);
    }
  }
  const comments = (await read('scripts/package-macos.sh')).split('\n').filter(line => line.startsWith('#')).join('\n');
  for (const literal of ['macOS 27.0', '15/26/27', '13.0', 'never forge Mach-O minima']) {
    assert.ok(comments.includes(literal), `Mac packaging: missing scope ${literal}`);
  }
});

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
