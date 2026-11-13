"""Score LongMemEval rankings on matching question IDs; never imply matching ingestion from scores alone."""
from __future__ import annotations
import argparse
import gzip
import hashlib
import json
import random
import sys
from pathlib import Path

BENCH = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(BENCH))
import fetch
import metrics


def read_json(path):
    opener = gzip.open if path.name.endswith('.gz') else open
    with opener(path, 'rt', encoding='utf-8') as stream:
        return json.load(stream)


def rankings_and_metadata(data):
    if not isinstance(data, dict):
        raise ValueError('ranking file must be a JSON object')
    if data.get('format') == 'icm-longmem-session-v1':
        return data['rankings'], {'type': 'icm-local-run', 'unit': 'raw sessions',
                                  'config': data.get('config'), 'binary_sha256': data.get('binary_sha256'),
                                  'dataset_pin': data.get('dataset_pin')}
    if data.get('format') == 'footnote-longmem-session-v1':
        return data['rankings'], {'type': 'footnote-run', 'unit': 'distinct sessions from source chunks',
                                  'config': data.get('config'), 'versions': data.get('versions'),
                                  'dataset_pin': data.get('dataset_pin')}
    if isinstance(data.get('results'), list):
        if data.get('dataset') != 'longmemeval' or data.get('mode') != 'recall':
            raise ValueError('unsupported vendor result')
        result = {}
        for row in data['results']:
            qid = row['query_id']
            if qid in result:
                raise ValueError(f'duplicate query: {qid}')
            result[qid] = row['ranked_ids']
        return result, {'type': 'vendor-reported', 'provider': data.get('memory_provider'),
                        'unit': data.get('unit'), 'provider_config': data.get('provider'),
                        'header': data.get('header'), 'source_gold': {x['query_id']: x['gold_ids'] for x in data['results']}}
    return data, {'type': 'rankings-only', 'unit': 'unverified'}


def score(rankings, questions, *, allow_subset=False, gold=None):
    qmap = {q['question_id']: q for q in questions}
    if len(qmap) != len(questions):
        raise ValueError('duplicate questions')
    unknown = set(rankings) - set(qmap)
    missing = set(qmap) - set(rankings)
    if unknown or (missing and not allow_subset) or not rankings:
        raise ValueError(f'query coverage mismatch: missing={len(missing)} unknown={len(unknown)}')
    measures = {}
    per_query = {}
    for qid, rank in rankings.items():
        if not isinstance(rank, list) or any(not isinstance(x, str) for x in rank):
            raise ValueError(f'invalid ranking for {qid}')
        q = qmap[qid]
        permitted = set(q['haystack_session_ids'])
        # ICM's harness prefixes session IDs with the question ID. Only strip a prefix when it maps to a real session.
        def canonical(value):
            prefix = qid + '_'
            return value[len(prefix):] if value not in permitted and value.startswith(prefix) and value[len(prefix):] in permitted else value
        rank = [canonical(value) for value in rank]
        if set(rank) - permitted:
            raise ValueError(f'foreign session in {qid}')
        expected = set(q['answer_session_ids'])
        if not expected or not expected.issubset(permitted):
            raise ValueError(f'invalid gold for {qid}')
        if gold is not None and {canonical(x) for x in gold.get(qid, [])} != expected:
            raise ValueError(f'gold mismatch for {qid}')
        qrel = {x: 1 for x in expected}
        a5, b5 = metrics.recall_any_all_at_k(rank, qrel, 5)
        a10, b10 = metrics.recall_any_all_at_k(rank, qrel, 10)
        per_query[qid] = {'any@5': a5, 'all@5': b5, 'any@10': a10, 'all@10': b10,
                          'ndcg@10_distinct': metrics.ndcg_at_k(metrics.dedupe(rank), qrel, 10),
                          'type': q['question_type'], 'abstention': qid.endswith('_abs')}
    groups = {'all': list(per_query), 'answerable': [q for q in per_query if not per_query[q]['abstention']],
              'abstention_tagged': [q for q in per_query if per_query[q]['abstention']]}
    for key in sorted({x['type'] for x in per_query.values()}):
        groups['type:' + key] = [q for q in per_query if per_query[q]['type'] == key]
    for group, qids in groups.items():
        measures[group] = {'questions': len(qids), **{m: (sum(per_query[q][m] for q in qids) / len(qids) if qids else None)
                 for m in ('any@5', 'all@5', 'any@10', 'all@10', 'ndcg@10_distinct')}}
    return {'coverage': {'expected': len(qmap), 'scored': len(rankings), 'missing': len(missing)},
            'metrics': measures, 'per_query': per_query}


def paired(a, b, resamples=10000, seed=20261112):
    if set(a) != set(b) or not a:
        raise ValueError('paired comparison requires identical nonempty query sets')
    rng = random.Random(seed)
    output = {}
    for key in ('any@5', 'all@5', 'any@10', 'all@10', 'ndcg@10_distinct'):
        diffs = [b[q][key] - a[q][key] for q in sorted(a)]
        samples = sorted(sum(rng.choices(diffs, k=len(diffs))) / len(diffs) for _ in range(resamples))
        output[key] = {'difference_b_minus_a': sum(diffs) / len(diffs),
                       'bootstrap_95': [samples[int(.025 * resamples)], samples[int(.975 * resamples)]]}
    return output


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('a', type=Path)
    ap.add_argument('--b', type=Path)
    ap.add_argument('--allow-subset', action='store_true', help='explicit exploratory subset; paired files still need identical query IDs')
    ap.add_argument('--intersection', action='store_true', help='explicitly compare common question IDs; requires --b and --allow-subset')
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    if args.output.exists():
        ap.error('output exists')
    if args.intersection and (not args.b or not args.allow_subset):
        ap.error('--intersection requires --b and --allow-subset')
    questions = fetch.load_longmemeval()
    inputs = []
    for path in (args.a, args.b):
        if path is not None:
            rankings, meta = rankings_and_metadata(read_json(path))
            local_pin = meta.get('dataset_pin')
            if local_pin is not None and local_pin != fetch.read_lock()['longmemeval_s']:
                raise ValueError(f'dataset pin mismatch: {path}')
            inputs.append((path, rankings, meta))
    counts = [len(rankings) for _, rankings, _ in inputs]
    if args.intersection:
        common = set(inputs[0][1]) & set(inputs[1][1])
        if not common:
            ap.error('no shared question IDs')
        inputs = [(path, {qid: ranks for qid, ranks in rankings.items() if qid in common}, meta)
                  for path, rankings, meta in inputs]
    outputs = []
    for path, rankings, meta in inputs:
        gold = meta.pop('source_gold', None)
        result = score(rankings, questions, allow_subset=args.allow_subset, gold=gold)
        result['source'] = {'file': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), **meta}
        outputs.append(result)
    report = {'protocol': 'longmemeval-session-scorer-v1', 'dataset_pin': fetch.read_lock()['longmemeval_s'],
              'note': 'Scores are paired only on identical IDs. Ingestion, models, versions, dates, corpus segmentation and timing are not verified by this scorer.',
              'raw_top_k_for_recall': True, 'intersection': args.intersection, 'original_query_counts': counts, 'systems': outputs}
    if len(outputs) == 2:
        report['paired'] = paired(outputs[0]['per_query'], outputs[1]['per_query'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as stream:
        json.dump(report, stream, indent=2, ensure_ascii=False)
    for system in outputs:
        print(system['source']['type'], system['coverage'], system['metrics']['all'])
    print('wrote', args.output)


if __name__ == '__main__':
    main()
