import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { loadPlaywright, launch } from '../../slop-scan/slop-scan.mjs';
import { runProfile } from '../design-check.mjs';

let b;
before(async () => { b = await launch(await loadPlaywright()); });
after(() => b?.close());
const fx = (n) => fileURLToPath(new URL(`../fixtures/${n}`, import.meta.url));
const run = (n, o) => runProfile(b, fx(n), { profile: 'generic', cards: 'none', ...o });
const ids = (r) => r.findings.map((f) => f.id).filter((i) => i !== 'K0-trends-stale');

test('clean CSS passes', async () => assert.deepEqual(ids(await run('css-good.html', { siteType: 'tech' })), []));
test('bad CSS trips K1, K3, K4, K5', async () => {
  const got = ids(await run('css-bad.html'));
  for (const id of ['K1-layout-anim', 'K3-colors', 'K4-vh', 'K5-font-display', 'K5-img-size']) assert.ok(got.includes(id), `${id} missing: ${got}`);
});
test('WebGL with fallback, reduced motion and pause passes on a tech site', async () => assert.deepEqual(ids(await run('webgl-good.html', { siteType: 'tech' })).filter((i) => i.startsWith('K7')), []));
test('WebGL gate: FAIL on a content site, WARNs when incomplete', async () => {
  const r = await run('webgl-bad.html', { siteType: 'content' });
  assert.equal(r.findings.find((f) => f.id === 'K7-webgl-site-type').severity, 'FAIL');
  for (const id of ['K7-webgl-fallback', 'K7-webgl-reduced', 'K7-webgl-pause']) assert.ok(ids(r).includes(id), id);
});
