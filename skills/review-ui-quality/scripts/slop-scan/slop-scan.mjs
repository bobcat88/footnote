#!/usr/bin/env node
// slop-scan: the mechanical Floor checks of references/slop-patterns.md, run at 1920x1080.
//   SL-01 fake success   SL-02 transition: all   SL-03 input states   SL-04 auto-advance without pause
//   SL-05 placeholder names   SL-31 hidden at rest (web)   SL-32 container gutters (web)   SL-33 fake window bar (web)
// Not here on purpose: SL-30 (invented proof) is a judgement, so the critic owns it.
//
//   node slop-scan.mjs <url|file> [--scope app|web] [--steps steps.json] [--out slop.json] [--sample-ms 2500]
//
// steps.json drives SL-01: [{"name":"save","do":[{"fill":["#email","a@b.co"]},{"click":"#save"}]}]
// SL-01 counts network calls (fetch, xhr, ping, form posts) and, for Tauri mocks, window.__slopIpcCalls:
// have the mockIPC handler do `window.__slopIpcCalls = (window.__slopIpcCalls || 0) + 1`.
// Only dependency: playwright-core, found via SLOP_SCAN_PLAYWRIGHT, the project, or the global @playwright/cli.
// Exit code: 0 clean, 1 at least one FAIL, 2 tooling or usage error.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { inPage } from './checks.mjs';

const VIEWPORT = { width: 1920, height: 1080 };
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const SUCCESS = /\b(saved|sent|submitted|thank(?:s| you)|you(?:'|’)re on the list|reserved|booked|subscribed|success(?:ful(?:ly)?)?|done)\b/gi;
const CALL_TYPES = new Set(['fetch', 'xhr', 'ping', 'eventsource', 'document']);

export async function loadPlaywright() {
  const roots = [process.env.SLOP_SCAN_PLAYWRIGHT, process.cwd(),
    process.env.APPDATA && join(process.env.APPDATA, 'npm', 'node_modules', '@playwright', 'cli'),
    '/usr/local/lib/node_modules/@playwright/cli', '/usr/lib/node_modules/@playwright/cli'].filter(Boolean);
  for (const root of roots) for (const name of ['playwright-core', 'playwright']) {
    try {
      const m = await import(pathToFileURL(createRequire(join(root, 'noop.js')).resolve(name)).href);
      const pw = m.default?.chromium ? m.default : m;
      if (pw.chromium) return pw;
    } catch { /* try the next root */ }
  }
  throw new Error('playwright-core not found: set SLOP_SCAN_PLAYWRIGHT to a folder whose node_modules holds it, or install it in the project');
}

export async function launch(pw) {
  try { return await pw.chromium.launch(); } catch { return await pw.chromium.launch({ channel: 'chrome' }); }
}

async function settle(page, settleMs) {
  await page.waitForTimeout(settleMs);
  const h = await page.evaluate(() => document.documentElement.scrollHeight);
  for (let y = 0; y < Math.min(h, 12000); y += 700) { await page.evaluate((v) => scrollTo(0, v), y); await page.waitForTimeout(Math.min(120, settleMs)); }
  await page.evaluate(() => scrollTo(0, 0));
  await page.waitForTimeout(settleMs);
}

export async function scan(browser, target, { scope = 'web', steps = [], sampleMs = 2500, settleMs = 600 } = {}) {
  const url = /^[a-z][a-z0-9+.-]*:\/\//i.test(target) ? target : pathToFileURL(resolve(target)).href;
  const web = scope === 'web';
  const findings = [], notRun = [], skipped = [];
  const add = (id, grade, detail, element) => findings.push({ id, tier: 'Floor', grade, detail, element });
  const ctx = await browser.newContext({ viewport: VIEWPORT });
  const page = await ctx.newPage();
  const calls = [];
  page.on('request', (r) => { if (CALL_TYPES.has(r.resourceType())) calls.push(r.url()); });
  try {
    await page.goto(url, { waitUntil: 'load' });
    await settle(page, settleMs);
    const c = await page.evaluate(inPage, { mode: 'collect', web });
    for (const f of c.sl02) add('SL-02', 'FAIL', f.detail, f.el);
    for (const f of c.sl03) add('SL-03', 'FAIL', f.detail, f.el);
    for (const m of c.sl05) add('SL-05', 'FAIL', `placeholder name or cliché in rendered text: "${m}"`, 'body');
    for (const f of c.sl32) add('SL-32', 'FAIL', f.detail, f.el);
    for (const f of c.sl33) add('SL-33', 'FAIL', f.detail, f.el);

    // SL-31: text visible with scripts on but invisible with scripts off (a reveal that needs JavaScript)
    if (web) {
      const on = await page.evaluate(inPage, { mode: 'vis' });
      const ctx2 = await browser.newContext({ viewport: VIEWPORT, javaScriptEnabled: false });
      try {
        const p2 = await ctx2.newPage();
        await p2.goto(url, { waitUntil: 'load' });
        await p2.waitForTimeout(settleMs);
        const off = new Map((await p2.evaluate(inPage, { mode: 'vis' })).map((e) => [e.p, e.v]));
        const hidden = on.filter((e) => e.v && off.has(e.p) && off.get(e.p) === false);
        if (hidden.length) add('SL-31', 'FAIL', `${hidden.length} text element(s) visible with scripts on but hidden (opacity 0) with scripts off`, hidden.slice(0, 3).map((e) => e.p).join(' | '));
      } catch (e) { notRun.push({ id: 'SL-31', reason: String(e.message).split('\n')[0] }); } finally { await ctx2.close(); }
    } else skipped.push({ id: 'SL-31', reason: 'scope app' }, { id: 'SL-32', reason: 'scope app' }, { id: 'SL-33', reason: 'scope app' });

    // SL-03: a field's box or border width must not change on focus
    for (let i = 0; i < c.fieldCount; i++) {
      const before = await page.evaluate(inPage, { mode: 'measure', i });
      await page.evaluate(inPage, { mode: 'focus', i });
      const after = await page.evaluate(inPage, { mode: 'measure', i });
      await page.evaluate(inPage, { mode: 'blur', i });
      const moved = Math.abs(before.w - after.w) > 0.5 || Math.abs(before.h - after.h) > 0.5 || before.bw.some((v, k) => Math.abs(v - after.bw[k]) > 0.1);
      if (moved) add('SL-03', 'FAIL', 'box size or border width changes on focus (build the ring with outline, not border)', before.el);
    }

    // SL-04: CSS animations that keep running on hover, and JS carousels that keep advancing on hover
    for (let i = 0; i < c.cssCands.length; i++) {
      const { x, y } = await page.evaluate(inPage, { mode: 'rect', kind: 'css', i });
      await page.mouse.move(x, y); await wait(300);
      if ((await page.evaluate(inPage, { mode: 'anim', i })) !== 'paused') add('SL-04', 'FAIL', 'infinite animation with text does not pause on hover', c.cssCands[i].el);
      await page.mouse.move(0, 0);
    }
    if (c.jsCands.length) {
      const sig = (i) => page.evaluate(inPage, { mode: 'sig', i });
      const t0 = await Promise.all(c.jsCands.map((_, i) => sig(i)));
      await wait(sampleMs);
      const t1 = await Promise.all(c.jsCands.map((_, i) => sig(i)));
      for (let i = 0; i < c.jsCands.length; i++) {
        if (t0[i] === t1[i]) continue; // not auto-advancing
        const { x, y } = await page.evaluate(inPage, { mode: 'rect', kind: 'js', i });
        await page.mouse.move(x, y); await wait(400);
        const h0 = await sig(i); await wait(sampleMs); const h1 = await sig(i);
        if (h0 !== h1) add('SL-04', 'FAIL', 'content auto-advances and keeps moving while hovered', c.jsCands[i].el);
        else if (!c.jsCands[i].hasPause) add('SL-04', 'WARN', 'pauses on hover but has no visible pause control (needed if it moves past 5 seconds)', c.jsCands[i].el);
        await page.mouse.move(0, 0);
      }
    }

    // SL-01: success text with no request and no IPC call behind it (runs last: it changes the page)
    if (!steps.length) notRun.push({ id: 'SL-01', reason: 'no --steps supplied' });
    for (const s of steps) {
      const before = await page.evaluate(inPage, { mode: 'page' });
      const from = calls.length;
      for (const a of s.do) {
        if (a.fill) await page.fill(a.fill[0], a.fill[1]);
        else if (a.click) await page.click(a.click);
        else if (a.press) await page.keyboard.press(a.press);
      }
      await page.waitForTimeout(s.settle ?? 700);
      const after = await page.evaluate(inPage, { mode: 'page' });
      const seen = (t) => (t.match(SUCCESS) || []).map((m) => m.toLowerCase());
      const b = seen(before.text), fresh = seen(after.text).filter((m) => { const k = b.indexOf(m); if (k >= 0) { b.splice(k, 1); return false; } return true; });
      const made = calls.length - from + (after.ipc - before.ipc);
      if (fresh.length && made === 0) add('SL-01', 'FAIL', `step "${s.name || '?'}" shows "${[...new Set(fresh)].join('", "')}" but made no request and no IPC call`, s.name || '');
    }
  } finally { await ctx.close(); }
  return { target: url, viewport: '1920x1080', scope, findings, notRun, skipped, floorFail: findings.filter((f) => f.grade === 'FAIL').length };
}

async function main() {
  const a = process.argv.slice(2);
  const flag = (k, d) => { const i = a.indexOf('--' + k); return i >= 0 && a[i + 1] ? a[i + 1] : d; };
  const target = a.find((x, i) => !x.startsWith('--') && !['--scope', '--steps', '--out', '--sample-ms'].includes(a[i - 1]));
  if (!target) { console.error('usage: node slop-scan.mjs <url|file> [--scope app|web] [--steps steps.json] [--out slop.json] [--sample-ms 2500]'); process.exit(2); }
  let browser;
  try {
    browser = await launch(await loadPlaywright());
    const steps = flag('steps') ? JSON.parse(readFileSync(flag('steps'), 'utf8')) : [];
    const report = await scan(browser, target, { scope: flag('scope', 'web'), steps, sampleMs: Number(flag('sample-ms', 2500)) });
    const json = JSON.stringify(report, null, 2);
    if (flag('out')) writeFileSync(flag('out'), json + '\n'); else console.log(json);
    console.error(`slop-scan: ${report.floorFail} FAIL, ${report.findings.length - report.floorFail} WARN, ${report.notRun.length} NOT RUN`);
    process.exitCode = report.floorFail ? 1 : 0;
  } catch (e) { console.error('slop-scan error: ' + e.message); process.exitCode = 2; } finally { if (browser) await browser.close(); }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main();
