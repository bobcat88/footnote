#!/usr/bin/env node
// paired-bootstrap: is system B better than A on the same items? Pairs per-item scores by id, resamples items
// with a fixed seed and reports the mean difference (B - A) with its confidence interval.
// usage: node paired-bootstrap.mjs <a> <b> [--min 0] [--resamples 10000] [--seed 1] [--level 0.95] [--json]
// Inputs: JSON {"id": score} / JSON [{id, score}] / JSONL / CSV or TSV "id,score". Higher score = better.
// Exit 0 better, 1 worse, 3 inconclusive, 2 invalid (ids differ, empty, unreadable).
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export function parseScores(text) {
  const t = text.replace(/^﻿/, '').trim();
  const map = new Map();
  const put = (id, s) => { if (id !== undefined && s !== '' && Number.isFinite(+s)) map.set(String(id), +s); };
  if (t.startsWith('{') && !t.includes('\n{')) for (const [id, s] of Object.entries(JSON.parse(t))) put(id, s);
  else if (t.startsWith('[')) for (const r of JSON.parse(t)) put(r.id, r.score);
  else if (t.startsWith('{')) for (const l of t.split(/\r?\n/).filter(Boolean)) { const r = JSON.parse(l); put(r.id, r.score); }
  else for (const l of t.split(/\r?\n/)) { const [id, s] = l.split(/[,\t]/); put(id?.trim(), s?.trim()); } // a header row is not numeric and drops out
  return map;
}

const rng = (seed) => () => { seed |= 0; seed = (seed + 0x6d2b79f5) | 0; let x = Math.imul(seed ^ (seed >>> 15), 1 | seed); x = (x + Math.imul(x ^ (x >>> 7), 61 | x)) ^ x; return ((x ^ (x >>> 14)) >>> 0) / 4294967296; };
const mean = (xs) => xs.reduce((s, x) => s + x, 0) / xs.length;

export function pairedBootstrap(a, b, { min = 0, resamples = 10000, seed = 1, level = 0.95 } = {}) {
  const onlyA = [...a.keys()].filter((k) => !b.has(k)), onlyB = [...b.keys()].filter((k) => !a.has(k));
  if (onlyA.length || onlyB.length || !a.size) return { decision: 'invalid', msg: `ids differ: ${onlyA.length} only in A, ${onlyB.length} only in B${a.size ? '' : ' (no scores)'}`, onlyA: onlyA.slice(0, 10), onlyB: onlyB.slice(0, 10) };
  const ids = [...a.keys()], d = ids.map((k) => b.get(k) - a.get(k)), n = d.length, r = rng(seed);
  const means = Array.from({ length: resamples }, () => { let s = 0; for (let i = 0; i < n; i++) s += d[Math.floor(r() * n)]; return s / n; }).sort((x, y) => x - y);
  const lo = means[Math.floor(((1 - level) / 2) * resamples)], hi = means[Math.min(resamples - 1, Math.floor(((1 + level) / 2) * resamples))];
  const decision = lo > min ? 'better' : hi < 0 ? 'worse' : 'inconclusive';
  return { decision, n, meanA: mean(ids.map((k) => a.get(k))), meanB: mean(ids.map((k) => b.get(k))), diff: mean(d), lo, hi, level, min, resamples, seed, wins: d.filter((x) => x > 0).length, losses: d.filter((x) => x < 0).length };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const a = process.argv.slice(2), opt = (k, dflt) => { const i = a.indexOf(k); return i < 0 ? dflt : +a.splice(i, 2)[1]; };
  const json = a.includes('--json') && a.splice(a.indexOf('--json'), 1);
  const o = { min: opt('--min', 0), resamples: opt('--resamples', 10000), seed: opt('--seed', 1), level: opt('--level', 0.95) };
  if (a.length !== 2) { console.error('usage: paired-bootstrap.mjs <a> <b> [--min 0] [--resamples 10000] [--seed 1] [--level 0.95] [--json]'); process.exit(2); }
  let r;
  try { r = pairedBootstrap(parseScores(readFileSync(a[0], 'utf8')), parseScores(readFileSync(a[1], 'utf8')), o); }
  catch (e) { r = { decision: 'invalid', msg: e.message }; }
  const f = (x) => x.toFixed(4);
  if (json) console.log(JSON.stringify(r, null, 2));
  else if (r.decision === 'invalid') console.log(`INVALID: ${r.msg}`);
  else console.log(`| n | A | B | B - A | ${Math.round(r.level * 100)}% interval | wins / losses | decision |\n|---|---|---|---|---|---|---|\n| ${r.n} | ${f(r.meanA)} | ${f(r.meanB)} | ${r.diff >= 0 ? '+' : ''}${f(r.diff)} | [${f(r.lo)}, ${f(r.hi)}] | ${r.wins} / ${r.losses} | **${r.decision}** |\nPaired bootstrap, ${r.resamples} resamples, seed ${r.seed}, min effect ${r.min}; one comparison, no multiplicity correction.`);
  process.exit({ better: 0, worse: 1, inconclusive: 3, invalid: 2 }[r.decision]);
}
