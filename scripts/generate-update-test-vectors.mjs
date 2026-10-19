#!/usr/bin/env node
// Reproducible public-only fixtures for the production manifest verifier.
// The disposable TEST key never enters the checkout or the release signer.
import {mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync, copyFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join, resolve} from 'node:path';
import {execFileSync} from 'node:child_process';
const temporary = mkdtempSync(join(tmpdir(), 'chill-update-test-key-'));
const output = resolve('crates/chill-updates/tests/vectors');
const minisign = process.env.MINISIGN || 'minisign';
const hostileOnly = process.argv.includes('--hostile-only');
try {
  execFileSync(minisign, ['-G', '-W', '-s', join(temporary, 'test.key'), '-p', join(temporary, 'test.pub')], {stdio: 'ignore'});
  mkdirSync(output, {recursive: true});
  const keyId = /public key (\w+)/.exec(readFileSync(join(temporary, 'test.pub'), 'utf8'))[1];
  const base = {
    schema_version: 1, component: 'server', version: '0.7.0', release_sequence: 2,
    release_tag: 'server-v0.7.0', source_commit: 'a'.repeat(40),
    issued_at: '2026-10-18T00:00:00Z', expires_at: '2026-11-18T00:00:00Z', key_id: keyId,
    compatibility: {os_arch: ['linux-aarch64'], api: {min: '2026.10.14', max: '2026.10.14'},
      database: {current_schema: 3, target_schema: 3, read_min: 3, read_max: 3, write_min: 3, write_max: 3, rollback_min: 3, rollback_max: 3}},
    assets: [{target: 'linux-aarch64', url: 'https://github.com/ungeekrange-cmyk/chillcenter/releases/download/server-v0.7.0/test-server', byte_length: 7, sha256: 'b'.repeat(64), format: 'raw'}],
  };
  const cases = {
    valid: () => {}, expired: m => {m.expires_at = '2026-10-18T12:00:00Z';},
    future: m => {m.issued_at = '2026-10-21T00:00:00Z';},
    replay: m => {m.release_sequence = 1;},
    component: m => {m.component = 'web'; m.release_tag = 'web-v0.7.0';},
    key: m => {m.key_id = 'unknown';},
    target: m => {m.assets[0].target = 'linux-x86_64'; m.compatibility.os_arch = ['linux-x86_64'];},
    database: m => {m.compatibility.database.rollback_min = 4; m.compatibility.database.rollback_max = 4;},
    api: m => {m.compatibility.api.min = '2027.1.1'; m.compatibility.api.max = '2027.1.1';},
    url: m => {m.assets[0].url = 'https://example.invalid/server';},
    unknown: m => {m.command = 'not accepted';},
    downgrade: m => {m.version = '0.5.0'; m.release_tag = 'server-v0.5.0';},
    duplicate: () => {},
  };
  if (!hostileOnly) {
    copyFileSync(join(temporary, 'test.pub'), join(output, 'test.pub'));
    for (const [name, mutate] of Object.entries(cases)) {
      const manifest = structuredClone(base); mutate(manifest);
      let bytes = JSON.stringify(manifest, null, 2) + '\n';
      if (name === 'duplicate') bytes = bytes.replace('"schema_version": 1', '"schema_version": 1, "schema_version": 1');
      const path = join(output, `${name}.json`); writeFileSync(path, bytes);
      execFileSync(minisign, ['-S', '-s', join(temporary, 'test.key'), '-m', path, '-x', path + '.minisig', '-t', 'TEST ONLY production parser vector'], {stdio: 'ignore'});
    }
  }
  if (hostileOnly) {
    const hostile = {
      mac: {
        target: 'darwin-aarch64', format: 'zip',
        compatibility: {os_arch: ['darwin-aarch64']}, tauri_signature: 'dGVzdC1vbmx5',
      },
      web: {
        target: 'linux-aarch64', format: 'raw',
        compatibility: {os_arch: ['linux-aarch64']},
      },
      themes: {
        target: 'any', format: 'zip',
        compatibility: {os_arch: ['any'], renderer: {theme_format: 1, renderer_min: 1, renderer_max: 1}},
      },
    };
    copyFileSync(join(temporary, 'test.pub'), join(output, 'hostile.pub'));
    for (const [component, details] of Object.entries(hostile)) {
      const manifest = {
        ...base,
        component,
        release_tag: `${component}-v0.7.0`,
        compatibility: details.compatibility,
        assets: [{
          target: details.target,
          url: `https://github.com/ungeekrange-cmyk/chillcenter/releases/download/${component}-v0.7.0/test-${component}`,
          byte_length: 7,
          sha256: 'b'.repeat(64),
          format: details.format,
          ...(details.tauri_signature ? {tauri_signature: details.tauri_signature} : {}),
        }],
      };
      const path = join(output, `hostile-${component}-valid.json`);
      writeFileSync(path, JSON.stringify(manifest, null, 2) + '\n');
      execFileSync(minisign, ['-S', '-s', join(temporary, 'test.key'), '-m', path, '-x', path + '.minisig', '-t', 'TEST ONLY hostile input vector'], {stdio: 'ignore'});
    }
  }
} finally { rmSync(temporary, {recursive: true, force: true}); }
