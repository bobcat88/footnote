#!/usr/bin/env node
// ir-score: scores a retrieval run against relevance judgments: nDCG@k (graded, linear gain), Recall@k, MRR@k.
// usage: node ir-score.mjs --qrels <file> --run <file> [--k 10] [--out per-query.json] [--metric ndcg|recall|mrr] [--json]
// qrels: TREC "qid 0 docid rel" or JSON {"qid": {"docid": rel}}. run: TREC "qid Q0 docid rank score tag" or JSON {"qid": ["docid", ...]}.
// A judged query missing from the run scores 0 and is listed. --out writes {qid: score} for paired-bootstrap.
// Exit 2 when queries are missing, so a broken run can't pass as a weak one.
import { readFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const lines = (t) => t.replace(/^﻿/, '').split(/\r?\n/).map((l) => l.trim().split(/\s+/)).filter((p) => p[0]);

export function parseQrels(text) {
  if (text.trim().startsWith('{')) return JSON.parse(text);
  const q = {};
  for (const [qid, , doc, rel] of lines(text)) (q[qid] ??= {})[doc] = +rel;
  return q;
}

export function parseRun(text) {
  if (text.trim().startsWith('{')) return JSON.parse(text);
  const r = {};
  for (const [qid, , doc, rank, score] of lines(text)) (r[qid] ??= []).push({ doc, rank: +rank, score: +score });
  // TREC order: score descending, ties by docid descending (trec_eval convention); rank column is ignored.
  for (const k in r) r[k] = r[k].sort((a, b) => b.score - a.score || (a.doc < b.doc ? 1 : -1)).map((x) => x.doc);
  return r;
}

export function scoreQuery(ranked, judged, k) {
  const top = [...new Set(ranked)].slice(0, k), rel = (d) => judged[d] ?? 0;
  const dcg = top.reduce((s, d, i) => s + rel(d) / Math.log2(i + 2), 0);
  const ideal = Object.values(judged).filter((g) => g > 0).sort((a, b) => b - a).slice(0, k).reduce((s, g, i) => s + g / Math.log2(i + 2), 0);
  const relevant = Object.values(judged).filter((g) => g > 0).length;
  const first = top.findIndex((d) => rel(d) > 0);
  return { ndcg: ideal ? dcg / ideal : 0, recall: relevant ? top.filter((d) => rel(d) > 0).length / relevant : 0, mrr: first < 0 ? 0 : 1 / (first + 1) };
}

export function irScore(qrels, run, k = 10) {
  const per = {}, missing = [];
  for (const qid of Object.keys(qrels)) {
    if (!Object.values(qrels[qid]).some((g) => g > 0)) continue; // no relevant document: excluded, as trec_eval does
    if (!run[qid]) missing.push(qid);
    per[qid] = scoreQuery(run[qid] ?? [], qrels[qid], k);
  }
  const ids = Object.keys(per), avg = (m) => (ids.length ? ids.reduce((s, q) => s + per[q][m], 0) / ids.length : 0);
  return { k, queries: ids.length, missing, extra: Object.keys(run).filter((q) => !qrels[q]).length, ndcg: avg('ndcg'), recall: avg('recall'), mrr: avg('mrr'), per };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const a = process.argv.slice(2), opt = (key) => { const i = a.indexOf(key); return i < 0 ? undefined : a[i + 1]; };
  if (!opt('--qrels') || !opt('--run')) { console.error('usage: ir-score.mjs --qrels <file> --run <file> [--k 10] [--out per-query.json] [--metric ndcg] [--json]'); process.exit(2); }
  const k = +(opt('--k') ?? 10), metric = opt('--metric') ?? 'ndcg';
  const r = irScore(parseQrels(readFileSync(opt('--qrels'), 'utf8')), parseRun(readFileSync(opt('--run'), 'utf8')), k);
  if (opt('--out')) writeFileSync(opt('--out'), JSON.stringify(Object.fromEntries(Object.entries(r.per).map(([q, s]) => [q, s[metric]])), null, 1));
  if (a.includes('--json')) console.log(JSON.stringify(r, null, 2));
  else console.log(`| queries | nDCG@${k} | Recall@${k} | MRR@${k} | missing | extra |\n|---|---|---|---|---|---|\n| ${r.queries} | ${r.ndcg.toFixed(4)} | ${r.recall.toFixed(4)} | ${r.mrr.toFixed(4)} | ${r.missing.length} | ${r.extra} |${r.missing.length ? `\nMISSING from run (scored 0): ${r.missing.slice(0, 20).join(' ')}` : ''}`);
  process.exit(r.missing.length ? 2 : 0);
}
