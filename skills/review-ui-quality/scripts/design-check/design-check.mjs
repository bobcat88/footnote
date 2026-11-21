#!/usr/bin/env node
// design-check: one CLI for every design language. Engine (engine.mjs) + a profile (profiles/<name>.json) + its extras.
// usage: node design-check.mjs <url|file> [--profile generic|apple|material] [--input touch|pointer] [--site-type tech|...]
//        [--grid 4] [--cards "article,.card"] [--perf] [--out report.json]
// New design language = new profiles/<name>.json (and an extras/<name>.mjs only for rules that need code).
import { pathToFileURL, fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { readFileSync, writeFileSync, statSync } from 'node:fs';
import { loadPlaywright, launch } from '../slop-scan/slop-scan.mjs';
import { check } from './engine.mjs';
import { cssExtras } from './extras/css.mjs';
import { glassFallback } from './extras/apple.mjs';
import { tokenChecks } from './extras/material.mjs';

const EXTRAS = { css: cssExtras, apple: glassFallback, material: tokenChecks };
const TRENDS = fileURLToPath(new URL('../../../write-modern-css/references/design-trends.md', import.meta.url));
const NOT_RUN = { apple: 'Liquid Glass fidelity (system material, judged by eye)', material: 'Expressive shape morphing and springs (Android only), colour roles, canonical layouts' };

export const loadProfile = (name) => JSON.parse(readFileSync(fileURLToPath(new URL(`./profiles/${name}.json`, import.meta.url)), 'utf8'));

// Trend data goes stale; the rule "last 12 months" is enforced by date, not by memory.
export function trendsStale(file = TRENDS, now = Date.now()) {
  const m = readFileSync(file, 'utf8').match(/researched (\d{4}-\d{2}-\d{2})/i);
  if (!m) return 'design-trends.md has no "researched YYYY-MM-DD" line';
  const days = Math.floor((now - Date.parse(m[1])) / 864e5);
  return days > 90 ? `design-trends.md is ${days} days old (researched ${m[1]}); refresh it before calling anything "current"` : null;
}

export async function runProfile(browser, target, { profile = 'generic', input, siteType, perf = false, grid, cards = 'article,[class*=card],li' } = {}) {
  const p = loadProfile(profile), opts = { siteType, perf, ...p };
  const extra = async (page) => { const out = []; for (const x of p.extras) out.push(...await EXTRAS[x](page, opts)); return out; };
  const r = await check(browser, target, { grid: grid ?? p.grid, minTarget: p.minTarget, input: input ?? p.input, cards, extra });
  const stale = trendsStale();
  if (stale) r.findings.push({ id: 'K0-trends-stale', severity: 'WARN', source: '12-month trend rule', msg: stale });
  r.profile = profile;
  r.notRun.unshift('"feel good": polish, hierarchy and taste stay with the QLoop critic');
  if (NOT_RUN[profile]) r.notRun.unshift(NOT_RUN[profile]);
  return r;
}

async function main() {
  const a = process.argv.slice(2), VAL = /^--(profile|input|site-type|grid|cards|out)$/;
  const flag = (k, d) => { const i = a.indexOf('--' + k); return i >= 0 && a[i + 1] && !a[i + 1].startsWith('--') ? a[i + 1] : d; };
  const target = a.find((x, i) => !x.startsWith('--') && !VAL.test(a[i - 1] || ''));
  if (!target) { console.error('usage: node design-check.mjs <url|file> [--profile generic|apple|material] [--input touch|pointer] [--site-type ...] [--perf] [--out f.json]'); process.exit(2); }
  let browser;
  try {
    browser = await launch(await loadPlaywright());
    const r = await runProfile(browser, target, { profile: flag('profile', 'generic'), input: flag('input'), siteType: flag('site-type'), perf: a.includes('--perf'), grid: flag('grid') && Number(flag('grid')), cards: flag('cards') });
    const json = JSON.stringify(r, null, 2);
    if (flag('out')) writeFileSync(flag('out'), json + '\n'); else console.log(json);
    console.error(`design-check[${r.profile}]: ${r.fail} FAIL, ${r.findings.length - r.fail} WARN, ${r.notRun.length} NOT RUN`);
    process.exitCode = r.fail ? 1 : 0;
  } catch (e) { console.error('design-check error: ' + e.message); process.exitCode = 2; } finally { if (browser) await browser.close(); }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main();
