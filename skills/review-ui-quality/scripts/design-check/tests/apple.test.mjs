import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { loadPlaywright, launch } from '../../slop-scan/slop-scan.mjs';
import { runProfile } from '../design-check.mjs';

let b;
before(async () => { b = await launch(await loadPlaywright()); });
after(() => b?.close());
const fx = (n) => fileURLToPath(new URL(`../fixtures/${n}`, import.meta.url));
const run = (n) => runProfile(b, fx(n), { profile: 'apple', cards: 'none' });

test('glass without a reduced-transparency override fails AP1', async () => {
  assert.ok((await run('glass-bad.html')).findings.some((f) => f.id === 'AP1-glass-fallback' && f.severity === 'FAIL'));
});
test('glass with an override passes AP1', async () => {
  assert.ok(!(await run('glass-good.html')).findings.some((f) => f.id === 'AP1-glass-fallback'));
});
test('one override rule with a selector list covers each glass selector', async () => {
  assert.ok(!(await run('glass-good-list.html')).findings.some((f) => f.id === 'AP1-glass-fallback'));
});
