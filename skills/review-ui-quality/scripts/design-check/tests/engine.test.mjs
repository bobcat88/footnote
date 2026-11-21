import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { loadPlaywright, launch } from '../../slop-scan/slop-scan.mjs';
import { check } from '../engine.mjs';

let b;
before(async () => { b = await launch(await loadPlaywright()); });
after(() => b?.close());
const fx = (n) => fileURLToPath(new URL(`../fixtures/${n}`, import.meta.url));

test('good card passes with no finding', async () => {
  const r = await check(b, fx('good.html'));
  assert.deepEqual(r.findings, []);
});

test('aligned icon list passes, misaligned one trips A7-A9', async () => {
  const good = await check(b, fx('nav-good.html'), { cards: 'none' });
  assert.deepEqual(good.findings.filter((f) => /^C[789]/.test(f.id)), []);
  const bad = (await check(b, fx('nav-bad.html'), { cards: 'none' })).findings.map((f) => f.id);
  for (const id of ['C7-icon-size', 'C8-icon-centre', 'C9-icon-gap']) assert.ok(bad.includes(id), `${id} missing: ${bad}`);
});

test('bad card trips every check', async () => {
  const r = await check(b, fx('bad.html'), { input: 'touch' });
  const ids = r.findings.map((f) => f.id);
  for (const id of ['C1-target', 'C2-reduced-motion', 'C3-focus', 'C4-contrast', 'C5-anchors', 'C6-spacing']) assert.ok(ids.includes(id), `${id} missing: ${ids}`);
  assert.ok(r.fail > 0);
});

test('sidebar: one axis and name hierarchy pass, centred misaligned one trips C10 and C11', async () => {
  const good = await check(b, fx('side-good.html'), { cards: 'none' });
  assert.deepEqual(good.findings.filter((f) => /^C1[01]/.test(f.id)), []);
  const bad = (await check(b, fx('side-bad.html'), { cards: 'none' })).findings.map((f) => f.id);
  for (const id of ['C10-nav-axis', 'C11-hierarchy']) assert.ok(bad.includes(id), `${id} missing: ${bad}`);
});

test('designed controls pass; default and stateless controls trip C12-C14', async () => {
  const good = await check(b, fx('controls-good.html'), { cards: 'none' });
  assert.deepEqual(good.findings, []);
  const bad = (await check(b, fx('controls-bad.html'), { cards: 'none' })).findings.map((f) => f.id);
  for (const id of ['C12-default-control', 'C13-states', 'C14-transition']) assert.ok(bad.includes(id), `${id} missing: ${bad}`);
});

test('C1: pointer input skips comfort size; inline and spaced small targets are exempt; crowded ones fail', async () => {
  const w = (await check(b, fx('c1-target.html'), { cards: 'none' })).findings.filter((f) => f.id === 'C1-target');
  assert.ok(w.every((f) => f.severity === 'FAIL' && /crowd/.test(f.msg)), JSON.stringify(w));
  assert.equal(w.length, 2);
  const t = (await check(b, fx('c1-target.html'), { cards: 'none', input: 'touch' })).findings.filter((f) => f.severity === 'WARN' && f.id === 'C1-target');
  assert.ok(t.length >= 1);
});
