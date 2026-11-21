// Run: node --test slop-scan.test.mjs   (needs playwright-core, see slop-scan.mjs)
// Every check is proven both ways: a known-bad page must fire exactly that check, and a known-clean
// page must produce no finding at all, so a check that never fires (or always fires) fails here.
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { loadPlaywright, launch, scan } from './slop-scan.mjs';

const FX = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');
const opts = { sampleMs: 600, settleMs: 150 };
const SAVE = [{ name: 'save', do: [{ fill: ['#email', 'a@b.co'] }, { click: '#go' }] }];
let browser;
before(async () => { browser = await launch(await loadPlaywright()); });
after(async () => { await browser?.close(); });

const run = (file, extra = {}) => scan(browser, join(FX, file), { ...opts, ...extra });
const fails = (r) => [...new Set(r.findings.filter((f) => f.grade === 'FAIL').map((f) => f.id))];

async function bad(file, id, extra) {
  const r = await run(file, extra);
  assert.deepEqual(fails(r), [id], `${file}: expected only ${id}, got ${JSON.stringify(r.findings)}`);
  return r;
}
async function clean(file, extra) {
  const r = await run(file, extra);
  assert.deepEqual(r.findings, [], `${file}: expected no findings, got ${JSON.stringify(r.findings)}`);
  return r;
}

test('SL-01 fake success: fires with no call, passes with a request or an IPC call', async () => {
  await bad('sl01-bad.html', 'SL-01', { steps: SAVE });
  await clean('sl01-clean-fetch.html', { steps: SAVE });
  await clean('sl01-clean-ipc.html', { steps: SAVE });
});
test('SL-01 reports NOT RUN when no steps are supplied', async () => {
  const r = await clean('sl01-bad.html');
  assert.ok(r.notRun.some((n) => n.id === 'SL-01'));
});
test('real-world patterns (screen-reader-only text, off-screen scroller, fading nav) stay clean', async () => { await clean('realworld-clean.html'); });
test('SL-02 transition: all', async () => { await bad('sl02-bad.html', 'SL-02'); await clean('sl02-clean.html'); });
test('SL-03 input states', async () => {
  const r = await bad('sl03-bad.html', 'SL-03');
  const d = r.findings.map((f) => f.detail).join('\n');
  assert.match(d, /height/); assert.match(d, /on focus/); assert.match(d, /disabled by class only/);
  await clean('sl03-clean.html');
});
test('SL-04 css animation must pause on hover', async () => { await bad('sl04-css-bad.html', 'SL-04'); await clean('sl04-css-clean.html'); });
test('SL-04 js carousel: FAIL if it keeps moving, WARN if it pauses but has no control', async () => {
  await bad('sl04-js-bad.html', 'SL-04');
  const w = await run('sl04-js-hoveronly.html');
  assert.deepEqual(w.findings.map((f) => [f.id, f.grade]), [['SL-04', 'WARN']]);
  await clean('sl04-js-clean.html');
});
test('SL-05 placeholder names', async () => { await bad('sl05-bad.html', 'SL-05'); await clean('sl05-clean.html'); });
test('SL-31 content hidden when scripts are off', async () => { await bad('sl31-bad.html', 'SL-31'); await clean('sl31-clean.html'); });
test('SL-32 container gutters at 1920', async () => { await bad('sl32-bad.html', 'SL-32'); await clean('sl32-clean.html'); await clean('sl32-skiplink.html'); });
test('SL-33 traffic-light window bar, and grey pagination dots stay clean', async () => { await bad('sl33-bad.html', 'SL-33'); await clean('sl33-clean.html'); });
test('scope app skips the web-only checks', async () => {
  const r = await run('sl32-bad.html', { scope: 'app' });
  assert.deepEqual(r.findings, []);
  assert.deepEqual(r.skipped.map((s) => s.id), ['SL-31', 'SL-32', 'SL-33']);
});
