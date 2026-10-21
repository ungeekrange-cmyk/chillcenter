import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('legacy slot drop-in changes only recovery and executable selection', async () => {
  const text = await readFile(new URL('../deploy/updates/40-legacy-update-slots.conf', import.meta.url), 'utf8');
  const lines = text.split('\n').filter(line => line && !line.startsWith('#'));
  assert.deepEqual(lines, [
    '[Service]',
    ...['server', 'web', 'themes'].map(component => `ExecStartPre=+/usr/lib/chillcenter/chill-update --config /etc/chillcenter/updates.json recover ${component}`),
    'ExecStart=',
    'ExecStart=/usr/lib/chillcenter/server/current/payload --config /etc/chillcenter/server.json',
  ]);
});
