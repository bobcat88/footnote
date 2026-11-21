import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { loadPlaywright, launch } from '../../slop-scan/slop-scan.mjs';
import { runProfile } from '../design-check.mjs';

let b;
before(async () => { b = await launch(await loadPlaywright()); });
after(() => b?.close());
const fx = (n) => fileURLToPath(new URL(`../fixtures/${n}`, import.meta.url));
const run = (n) => runProfile(b, fx(n), { profile: 'material', cards: 'none' });

test('on-token page passes', async () => assert.deepEqual((await run('m3-good.html')).findings.filter((f) => f.id !== 'K0-trends-stale'), []));
test('off-token page trips M1-M4 and the 48 px target', async () => {
  const ids = (await run('m3-bad.html')).findings.map((f) => f.id);
  for (const id of ['M1-shape', 'M2-type', 'M3-duration', 'M3-easing', 'M4-disabled', 'C1-target']) assert.ok(ids.includes(id), `${id} missing: ${ids}`);
});
