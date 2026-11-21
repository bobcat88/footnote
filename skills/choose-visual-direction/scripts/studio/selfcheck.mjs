// Self-check for the studio's colour and contrast maths (the <lib> block of studio.template.html).
// Run: node selfcheck.mjs   (no dependencies)
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';

const src = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'studio.template.html'), 'utf8');
const block = src.slice(src.indexOf('// <lib>'), src.indexOf('// </lib>'));
assert.ok(block.length > 100, 'lib block found');
const LIB = new Function(block + '; return LIB;')();
const near = (a, b, tol, msg) => assert.ok(Math.abs(a - b) <= tol, `${msg}: ${a} vs ${b}`);

// WCAG contrast reference values
near(LIB.contrast('#000000', '#ffffff'), 21, 0.01, 'black on white');
near(LIB.contrast('#ffffff', '#ffffff'), 1, 0.001, 'white on white');
near(LIB.contrast('#777777', '#ffffff'), 4.48, 0.02, 'grey 777 on white');

// OKLCH conversion: extremes and a known mid grey
assert.equal(LIB.oklchToHex(1, 0, 0), '#ffffff');
assert.equal(LIB.oklchToHex(0, 0, 0), '#000000');
const g = LIB.hexToRgb(LIB.oklchToHex(0.6, 0, 0));
assert.ok(g[0] === g[1] && g[1] === g[2], 'zero chroma stays grey');

// Every theme, warmth and tint the sliders allow must keep text readable
for (const theme of ['light', 'dark']) for (const warmth of [-1, 0, 1]) for (const tint of [0, 3, 6]) {
  const P = LIB.palette({ theme, warmth, tint, accent: '#2552c9' });
  for (const c of LIB.checks(P).filter((x) => /Text|Muted/.test(x.name))) assert.ok(c.pass, `${theme} warmth ${warmth} tint ${tint}: ${c.name} ${c.ratio}`);
}

// Button text picks the better of white and near-black on any accent
for (const accent of ['#ffe14d', '#2552c9', '#f05a1e', '#7fc8ff', '#043f2e']) {
  const P = LIB.palette({ theme: 'light', warmth: 0, tint: 2, accent });
  const other = P.ink === '#ffffff' ? '#101114' : '#ffffff';
  assert.ok(LIB.contrast(P.ink, accent) >= LIB.contrast(other, accent), `ink for ${accent}`);
}

// Type scale grows with the ratio and respects the caps
const small = LIB.typeScale(1.125, 'app'), big = LIB.typeScale(1.5, 'page');
assert.ok(small.h1 < big.h1 && big.h1 <= 88 && LIB.typeScale(1.5, 'app').h1 <= 56);
assert.deepEqual(LIB.spacing(1), [4, 8, 12, 16, 24, 32, 48, 64]);
console.log('studio selfcheck: all passed');
