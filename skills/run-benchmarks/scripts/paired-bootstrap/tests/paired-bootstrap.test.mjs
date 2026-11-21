import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pairedBootstrap, parseScores } from '../paired-bootstrap.mjs';

const m = (o) => new Map(Object.entries(o));
const ids = Array.from({ length: 40 }, (_, i) => `q${i}`);

test('all four input formats parse to the same map', () => {
  const want = [['q1', 0.5], ['q2', 1]];
  for (const t of ['{"q1":0.5,"q2":1}', '[{"id":"q1","score":0.5},{"id":"q2","score":1}]', '{"id":"q1","score":0.5}\n{"id":"q2","score":1}', 'id,score\nq1,0.5\nq2\t1\n'])
    assert.deepEqual([...parseScores(t)], want, t);
});

test('a consistent gain is better, a consistent loss is worse, noise is inconclusive', () => {
  const a = m(Object.fromEntries(ids.map((k, i) => [k, (i % 5) / 10])));
  const up = m(Object.fromEntries(ids.map((k, i) => [k, (i % 5) / 10 + 0.05 + (i % 3) / 100])));
  const down = m(Object.fromEntries(ids.map((k, i) => [k, (i % 5) / 10 - 0.05])));
  const noise = m(Object.fromEntries(ids.map((k, i) => [k, (i % 5) / 10 + (i % 2 ? 0.1 : -0.1)])));
  assert.equal(pairedBootstrap(a, up).decision, 'better');
  assert.equal(pairedBootstrap(a, down).decision, 'worse');
  assert.equal(pairedBootstrap(a, noise).decision, 'inconclusive');
  assert.equal(pairedBootstrap(a, up, { min: 0.2 }).decision, 'inconclusive');
  assert.deepEqual(pairedBootstrap(a, up), pairedBootstrap(a, up), 'same seed, same interval');
});

test('mismatched ids are invalid, never silently dropped', () => {
  const r = pairedBootstrap(m({ q1: 1, q2: 1 }), m({ q1: 1, q3: 1 }));
  assert.equal(r.decision, 'invalid');
  assert.deepEqual([r.onlyA, r.onlyB], [['q2'], ['q3']]);
});
