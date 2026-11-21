import { test } from 'node:test';
import assert from 'node:assert/strict';
import { irScore, parseQrels, parseRun, scoreQuery } from '../ir-score.mjs';

const close = (a, b) => assert.ok(Math.abs(a - b) < 1e-9, `${a} != ${b}`);

test('nDCG, recall and MRR on a hand-checked query', () => {
  // ranked: d2 (rel 1), d9 (0), d1 (rel 2); ideal: d1, d2
  const s = scoreQuery(['d2', 'd9', 'd1'], { d1: 2, d2: 1 }, 10);
  close(s.ndcg, (1 + 2 / 2) / (2 + 1 / Math.log2(3)));
  close(s.recall, 1);
  close(s.mrr, 1);
  close(scoreQuery(['d9', 'd1'], { d1: 1 }, 1).recall, 0);
});

test('TREC files parse; the run is ordered by score, not by its rank column', () => {
  const q = parseQrels('q1 0 d1 2\nq1 0 d2 1\nq2 0 d5 0\nq3 0 d7 1\n');
  const r = parseRun('q1 Q0 d2 1 0.2 x\nq1 Q0 d1 2 0.9 x\nq9 Q0 d1 1 1 x\n');
  assert.deepEqual(r.q1, ['d1', 'd2']);
  const out = irScore(q, r, 10);
  assert.equal(out.queries, 2, 'q2 has no relevant doc and is excluded');
  assert.deepEqual(out.missing, ['q3']);
  assert.equal(out.extra, 1);
  close(out.per.q1.ndcg, 1);
  close(out.per.q3.ndcg, 0);
});
