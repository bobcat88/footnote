#!/usr/bin/env node
// bench-gate: decide whether a candidate beats a baseline from repeated samples (lower is better: time, bytes, RSS).
// usage: node bench-gate.mjs <baseline> <candidate> [--min 0.02] [--alpha 0.05] [--unit s] [--json]
// Inputs: hyperfine --export-json files (first result), a JSON array of numbers, or one number per line.
// Exit 0 promote, 1 reject (regression), 3 inconclusive, 2 invalid input.
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export function load(text) {
  const t = text.trim();
  if (t.startsWith('{')) return JSON.parse(t).results[0].times;
  if (t.startsWith('[')) return JSON.parse(t);
  return t.split(/\s+/).map(Number);
}

const median = (xs) => { const s = [...xs].sort((a, b) => a - b), m = s.length >> 1; return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2; };

// Two-sided Mann-Whitney U. Exact by enumerating rank subsets up to 20 samples in total, normal approximation above.
export function mannWhitney(a, b) {
  const all = [...a.map((v) => [v, 0]), ...b.map((v) => [v, 1])].sort((x, y) => x[0] - y[0]);
  const ranks = new Array(all.length);
  for (let i = 0; i < all.length;) {
    let j = i; while (j + 1 < all.length && all[j + 1][0] === all[i][0]) j++;
    for (let k = i; k <= j; k++) ranks[k] = (i + j) / 2 + 1;
    i = j + 1;
  }
  const n1 = a.length, n2 = b.length, r1 = all.reduce((s, x, i) => s + (x[1] === 0 ? ranks[i] : 0), 0);
  const u = r1 - n1 * (n1 + 1) / 2, mean = n1 * n2 / 2, dev = Math.abs(u - mean);
  if (n1 + n2 <= 20) {
    let hits = 0, total = 0;
    const pick = (start, left, sum) => {
      if (left === 0) { total++; if (Math.abs(sum - n1 * (n1 + 1) / 2 - mean) >= dev - 1e-9) hits++; return; }
      for (let i = start; i <= ranks.length - left; i++) pick(i + 1, left - 1, sum + ranks[i]);
    };
    pick(0, n1, 0);
    return hits / total;
  }
  const z = (dev - 0.5) / Math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12);
  const erfc = (x) => { const t = 1 / (1 + 0.5 * x); return t * Math.exp(-x * x - 1.26551223 + t * (1.00002368 + t * (0.37409196 + t * (0.09678418 + t * (-0.18628806 + t * (0.27886807 + t * (-1.13520398 + t * (1.48851587 + t * (-0.82215223 + t * 0.17087277))))))))); };
  return Math.min(1, erfc(Math.max(z, 0) / Math.SQRT2));
}

export function gate(base, cand, { min = 0, alpha = 0.05 } = {}) {
  if (base.length < 3 || cand.length < 3) return { decision: 'invalid', reason: 'need at least 3 samples on each side' };
  if (![...base, ...cand].every(Number.isFinite)) return { decision: 'invalid', reason: 'non-numeric sample' };
  const mb = median(base), mc = median(cand), delta = (mc - mb) / mb, p = mannWhitney(base, cand);
  const r = { n: [base.length, cand.length], median: [mb, mc], worst: [Math.max(...base), Math.max(...cand)], delta, p };
  if (p >= alpha) return { ...r, decision: 'inconclusive', reason: `difference within noise (p=${p.toFixed(3)}); keep the simpler code or collect more runs` };
  if (delta > 0) return { ...r, decision: 'reject', reason: 'candidate is slower' };
  if (-delta < min) return { ...r, decision: 'inconclusive', reason: `real but below the --min ${(min * 100).toFixed(1)}% that justifies the change` };
  return { ...r, decision: 'promote', reason: 'faster beyond noise; check outputs matched before merging' };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2), flag = (n, d) => { const i = args.indexOf(n); return i < 0 ? d : args.splice(i, 2)[1]; };
  const min = Number(flag('--min', 0)), alpha = Number(flag('--alpha', 0.05)), unit = flag('--unit', ''), json = args.includes('--json');
  const [a, b] = args.filter((x) => !x.startsWith('--'));
  if (!a || !b) { console.error('usage: node bench-gate.mjs <baseline> <candidate> [--min 0.02] [--alpha 0.05] [--unit s] [--json]'); process.exit(2); }
  const r = gate(load(readFileSync(a, 'utf8')), load(readFileSync(b, 'utf8')), { min, alpha });
  if (json) console.log(JSON.stringify(r, null, 2));
  else if (r.decision === 'invalid') console.log(`invalid: ${r.reason}`);
  else {
    const f = (x) => `${+x.toPrecision(4)}${unit ? ' ' + unit : ''}`, pct = `${Math.abs(r.delta * 100).toFixed(1)}% ${r.delta <= 0 ? 'faster' : 'slower'}`;
    console.log(`| | runs | median | worst | vs baseline |\n|---|---|---|---|---|\n| baseline | ${r.n[0]} | ${f(r.median[0])} | ${f(r.worst[0])} | |\n| candidate | ${r.n[1]} | ${f(r.median[1])} | ${f(r.worst[1])} | ${pct} |`);
    console.log(`\n${r.decision.toUpperCase()} (p=${r.p.toFixed(4)}): ${r.reason}`);
  }
  process.exit({ promote: 0, reject: 1, invalid: 2, inconclusive: 3 }[r.decision]);
}
