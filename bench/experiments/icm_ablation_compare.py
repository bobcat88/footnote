"""Paired bootstrap for two ablation arms on exactly the same judged inputs."""
from __future__ import annotations
import argparse
import json
import random
import sys
from pathlib import Path

BENCH = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(BENCH), str(Path(__file__).resolve().parent)]
import fetch
import icm_ablation as suite
import metrics


def paired(a, b, qrels, resamples=10000):
    qids = sorted(q for q, gold in qrels.items() if any(value > 0 for value in gold.values()))
    if not qids or set(a) != set(b) or set(a) != set(qrels):
        raise ValueError('rankings and qrels must contain exactly the same nonempty query IDs')
    rng = random.Random(20261112)
    out = {}
    for metric, fn in [('ndcg@10', lambda r, g: metrics.ndcg_at_k(r, g, 10)),
                       ('recall@10', lambda r, g: metrics.recall_at_k(r, g, 10)),
                       ('recall@20', lambda r, g: metrics.recall_at_k(r, g, 20))]:
        delta = [fn(b[q], qrels[q]) - fn(a[q], qrels[q]) for q in qids]
        distribution = sorted(sum(rng.choices(delta, k=len(delta))) / len(delta) for _ in range(resamples))
        out[metric] = {'b_minus_a': sum(delta) / len(delta),
                       'bootstrap_95': [distribution[int(.025 * resamples)], distribution[int(.975 * resamples)]]}
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('file_a', type=Path)
    ap.add_argument('arm_a', help='fts5:cc, fts5:rrf, tantivy:cc or tantivy:rrf')
    ap.add_argument('file_b', type=Path)
    ap.add_argument('arm_b')
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--allow-partial', action='store_true', help='exploratory only; never report as full split')
    args = ap.parse_args()
    if args.output.exists():
        ap.error('output exists')
    data = [json.loads(p.read_text(encoding='utf8')) for p in (args.file_a, args.file_b)]
    a, b = data
    for item in data:
        if item.get('protocol') != 'icm-ablation-v1':
            ap.error('unsupported receipt format')
        if item.get('partial') and not args.allow_partial:
            ap.error('partial receipt; use --allow-partial only for exploration')
    if any(a.get(key) != b.get(key) for key in ('dataset', 'split', 'input_sha256', 'pool', 'result_limit', 'script_sha256')):
        ap.error('input, split, candidate pool or harness mismatch')
    corpus, queries, qrels = fetch.load(a['dataset'], a['split'])
    arms = []
    for item, name in zip(data, (args.arm_a, args.arm_b)):
        matches = [row for row in item['rows'] if f"{row['lexical']}:{row['fusion']}" == name]
        if len(matches) != 1:
            ap.error(f'unknown or ambiguous arm {name}')
        ranking = matches[0]['rankings']
        det = matches[0].get('determinism', {})
        if det.get('queries_checked', 0) < min(200, len(ranking)) or det.get('identical_rankings') is not True:
            ap.error('determinism missing, incomplete or failed')
        if suite.digest(ranking) != matches[0]['rankings_sha256']:
            ap.error('ranking hash mismatch')
        arms.append(ranking)
    if a['partial']:
        queries = {q: queries[q] for q in arms[0]}; qrels = {q: qrels[q] for q in arms[0]}
    if suite.digest({'corpus': corpus, 'queries': queries, 'qrels': qrels}) != a['input_sha256']:
        ap.error('current dataset does not match receipt')
    result = {'protocol': 'icm-ablation-paired-v1', 'dataset': a['dataset'], 'split': a['split'],
              'partial': a['partial'], 'queries': len(qrels),
              'a': {'file': str(args.file_a), 'arm': args.arm_a, 'model': a['model']},
              'b': {'file': str(args.file_b), 'arm': args.arm_b, 'model': b['model']},
              'note': 'Paired lab rankings only; not a same-machine ICM or shipped-product result.',
              'metrics': paired(*arms, qrels)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf8') as f:
        json.dump(result, f, indent=2)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
