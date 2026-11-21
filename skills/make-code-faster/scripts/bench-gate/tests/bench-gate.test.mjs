import { test } from 'node:test';
import assert from 'node:assert/strict';
import { gate, load, mannWhitney } from '../bench-gate.mjs';

test('exact p-value matches the textbook value for full separation', () => {
  // 5 vs 5, no overlap: two-sided p = 2 / C(10,5) = 2/252
  assert.ok(Math.abs(mannWhitney([1, 2, 3, 4, 5], [6, 7, 8, 9, 10]) - 2 / 252) < 1e-12);
  assert.equal(mannWhitney([1, 2, 3], [1, 2, 3]), 1);
});

test('normal approximation is used for large samples and stays sane', () => {
  const a = Array.from({ length: 30 }, (_, i) => 100 + i), b = a.map((x) => x + 50);
  assert.ok(mannWhitney(a, b) < 1e-6);
});

test('clear win promotes, clear loss rejects, noise is inconclusive', () => {
  const base = [1.00, 1.02, 0.99, 1.01, 1.03];
  assert.equal(gate(base, [0.80, 0.81, 0.79, 0.82, 0.80]).decision, 'promote');
  assert.equal(gate(base, [1.20, 1.21, 1.19, 1.22, 1.20]).decision, 'reject');
  assert.equal(gate(base, [1.01, 0.99, 1.02, 1.00, 1.03]).decision, 'inconclusive');
});

test('a real but tiny win under --min is inconclusive', () => {
  const base = [1.000, 1.001, 1.002, 1.001, 1.000], cand = [0.990, 0.991, 0.990, 0.992, 0.991];
  assert.equal(gate(base, cand).decision, 'promote');
  assert.equal(gate(base, cand, { min: 0.05 }).decision, 'inconclusive');
});

test('fewer than 3 runs is invalid', () => {
  assert.equal(gate([1, 2], [1, 2, 3]).decision, 'invalid');
});

test('loads hyperfine JSON, arrays and plain lines', () => {
  assert.deepEqual(load('{"results":[{"command":"x","times":[0.1,0.2]}]}'), [0.1, 0.2]);
  assert.deepEqual(load('[1, 2]'), [1, 2]);
  assert.deepEqual(load('1\n2\n3\n'), [1, 2, 3]);
});
