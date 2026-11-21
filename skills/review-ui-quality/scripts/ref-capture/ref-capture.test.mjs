// Run: node --test ref-capture.test.mjs   (needs playwright-core and Chrome, see slop-scan.mjs)
// Two fixtures prove the tier logic both ways: plain CSS must NOT be called scripted or webgl,
// and a page with GSAP, a WebGL canvas, shaders and a rAF loop must be called webgl.
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { join, dirname } from 'node:path';
import { loadPlaywright } from '../slop-scan/slop-scan.mjs';
import { capture, launchGpu } from './ref-capture.mjs';

const FX = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');
let g, out;
before(async () => { g = await launchGpu(await loadPlaywright()); out = mkdtempSync(join(tmpdir(), 'ref-capture-')); });
after(async () => { await g?.browser.close(); });
// waitMs must outlast the fixture's 0.4 s entrance animation, or the first frame is mid-fade and idleMotion is correctly true
const run = (f) => capture(g.browser, pathToFileURL(join(FX, f)).href, out, { shots: 2, waitMs: 1200, engine: g.engine });

test('plain CSS page: tier css, tokens and easing read correctly, no libraries, no idle motion', async () => {
  const r = await run('ref-css.html');
  assert.equal(r.error, null);
  assert.equal(r.tier, 'css');
  assert.deepEqual(r.libs, {});
  assert.equal(r.idleMotion, false);
  const t = r.tokens;
  assert.equal(t.scheme, 'dark');
  assert.equal(t.bodyBg, '#101418');
  assert.ok(t.backgrounds.some(([c]) => c === '#1b222a'), 'card background');
  assert.ok(t.radii.some(([c]) => c === '14px'));
  assert.ok(t.transitions.some(([c]) => c.includes('cubic-bezier(0.22, 1, 0.36, 1)') && c.includes('0.6s')), JSON.stringify(t.transitions));
  assert.ok(t.keyframes.includes('rise'));
  assert.equal(t.reducedMotionRule, true);
  assert.deepEqual(r.pageTransitions, []);
});

test('GSAP + WebGL + shaders + rAF loop: tier webgl, libraries and View Transitions named', async () => {
  const r = await run('ref-scripted.html');
  assert.equal(r.error, null);
  assert.equal(r.tier, 'webgl');
  for (const k of ['gsap', 'shaders', 'viewTransitions']) assert.ok(k in r.libs, `${k} in ${JSON.stringify(r.libs)}`);
  assert.ok(r.canvasContexts.some((c) => /webgl/.test(c)));
  assert.ok(r.rafPerSec > 20, `rAF/s ${r.rafPerSec}`);
  assert.ok(r.pageTransitions.includes('view-transitions-api'));
});

test('regression (umbrel, avaros): shader and View Transition strings in a bundle are not WebGL and not a page transition', async () => {
  const r = await run('ref-sig-only.html');
  assert.equal(r.tier, 'css');
  assert.deepEqual(r.pageTransitions, []);
  assert.ok('shaders' in r.libs && 'viewTransitions' in r.libs, 'the signatures are still reported as capability');
  assert.equal(r.viewTransitionsInBundle, true);
  assert.equal(r.viewTransitionCalls, 0);
});

test('regression (ventriloc): a Rive runtime makes the tier scripted, not css', async () => {
  const r = await run('ref-rive-sig.html');
  assert.equal(r.tier, 'scripted');
  assert.ok('rive' in r.libs);
});

test('regression (enpowertrading): a "Confirm" consent button unlocks a page that reports 1080px until it is clicked', async () => {
  const r = await run('ref-consent.html');
  assert.equal(r.consentClicked, true);
  assert.ok(r.tokens.scrollHeight > 3000, `scrollHeight ${r.tokens.scrollHeight}`);
});

test('regression (enpowertrading): a page that stays 1080px until the first wheel input is warmed up, then scrolled', async () => {
  const r = await run('ref-wheel-unlock.html');
  assert.ok(r.tokens.scrollHeight > 3000, `scrollHeight ${r.tokens.scrollHeight}`);
  assert.ok(r.scrollY.some((y) => y > 0), JSON.stringify(r.scrollY));
});

test('regression (ventriloc): --hide removes a loader that never clears, and the result says so', async () => {
  const stuck = await run('ref-loader.html');
  assert.ok(stuck.tokens.backgrounds.some(([c]) => c === '#aa0000'), 'the loader covers the page without --hide');
  const freed = await capture(g.browser, pathToFileURL(join(FX, 'ref-loader.html')).href, out, { shots: 2, waitMs: 1200, engine: g.engine, hide: '.site-loader' });
  assert.ok(!freed.tokens.backgrounds.some(([c]) => c === '#aa0000'), 'the loader is gone with --hide');
  assert.equal(freed.hidden, '.site-loader');
});

test('the engine is real Chrome with a GPU renderer, not the software fallback', async () => {
  const r = await run('ref-scripted.html');
  assert.equal(g.engine, 'chrome');
  assert.doesNotMatch(r.renderer, /SwiftShader/i, r.renderer);
});

test('the hero frames and the cursor frame are written', async () => {
  const r = await run('ref-css.html');
  assert.deepEqual(r.shots.slice(0, 4), ['01-hero.jpg', '01b-hero-plus-0.9s.jpg', '01c-hero-cursor.jpg', '02.jpg']);
});
