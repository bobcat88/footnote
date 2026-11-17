"""Reranker abstention gate for memory recall (goal 9). See MEMORY-ABSTENTION.md for the rule, fixed before any data.

  collect  --split dev|heldout --output X.jsonl   hybrid top 3 with memory text per question (resumable, manifest-guarded)
  score    X.jsonl                                 adds the reranker logit to every hit (needs onnxruntime and tokenizers)
  analyse  X.jsonl [--threshold T]                 dev: choose T by the rule; held-out: judge a frozen T
"""
import argparse
import concurrent.futures
import hashlib
import json
import random
import sqlite3
import tempfile
from contextlib import closing
from pathlib import Path

import memory_cutoff_collect as old

ROOT = old.ROOT
RERANKER = ROOT / 'bench/.cache/models/mmarco-mMiniLMv2-L12'
RERANKER_SHA = {'model.onnx': '3e9a03ed1e966f7c5288dd4230e3d6a9bf5e3a170a06f1f4241c5bca12c6487c',
                'tokenizer.json': '62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626'}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def collect_one(q, other, directory):
    with tempfile.TemporaryDirectory(dir=directory) as temp:
        temp = Path(temp)
        db = temp / 'memory.db'
        batch = old.memories(q)
        if not batch:
            raise ValueError('no memories for ' + q['question_id'])
        for i in range(0, len(batch), 50):
            old.call(db, ['remember'], temp, json.dumps(batch[i:i + 50]))
        model_args = [v for m in old.MODELS for v in ('--model', str(m))]
        old.call(db, ['rebuild-embeddings', *model_args], temp)
        with closing(sqlite3.connect(db)) as c:
            sessions = dict(c.execute('SELECT c.id,m.session FROM memories m JOIN chunks c ON c.source_id=m.source_id'))

        def row(query, label, gold):
            text = old.query_text(query)
            hits = old.call(db, ['memory-recall', '--project', 'longmem', '--limit', '3', *model_args, '--', text],
                            temp)['memories']
            return {'question_id': query['question_id'], 'label': label, 'query': text,
                    'hits': [{'id': h['id'], 'score': h['score'], 'text': h['text'],
                              'relevant': sessions.get(h['id']) in gold} for h in hits]}

        own = 'abs' if q['question_id'].endswith('_abs') else 'positive'
        answer = set() if own == 'abs' else set(q['answer_session_ids'])
        return [row(q, own, answer), row(other, 'cross', set())]


def collect(args):
    data = sorted(json.loads(old.DATA.read_text(encoding='utf-8')), key=lambda q: q['question_id'])
    identity = {str(p.relative_to(ROOT)): sha(p) for p in
                [Path(__file__), Path(old.__file__), old.DATA, old.BIN,
                 *[m / n for m in old.MODELS for n in ('model.onnx', 'manifest.json', 'tokenizer.json')]]}
    identity |= {'split': args.split, 'limit': args.limit}
    manifest = args.output.with_suffix(args.output.suffix + '.manifest.json')
    if manifest.exists():
        if json.loads(manifest.read_text(encoding='utf-8')) != identity:
            raise ValueError('inputs changed since this collection began')
    elif args.output.exists():
        raise ValueError('refusing to resume an output without its manifest')
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text(json.dumps(identity, indent=2), encoding='utf-8')
    done = set()
    if args.output.exists():
        done = {json.loads(line)['source_id'] for line in args.output.read_text(encoding='utf-8').splitlines()}
    selected = [q for q in data if old.split(q['question_id']) == args.split]
    selected = selected[:args.limit] if args.limit else selected

    def run(q):
        # Cross negative: same split, answerable, none of its answer sessions in this database (as the earlier experiments).
        options = [x for x in data if x['question_id'] != q['question_id'] and old.split(x['question_id']) == args.split
                   and not set(x['answer_session_ids']) & set(q['haystack_session_ids'])
                   and not x['question_id'].endswith('_abs')]
        other = options[int(hashlib.sha256(q['question_id'].encode()).hexdigest(), 16) % len(options)]
        return {'source_id': q['question_id'], 'rows': collect_one(q, other, args.output.parent)}

    remaining = [q for q in selected if q['question_id'] not in done]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for index, result in enumerate(pool.map(run, remaining), len(done) + 1):
            with args.output.open('a', encoding='utf-8') as f:
                f.write(json.dumps(result) + '\n')
            print(f'{index}/{len(selected)} {result["source_id"]}', flush=True)


def reranker():
    import onnxruntime
    from tokenizers import Tokenizer
    for name, digest in RERANKER_SHA.items():
        if sha(RERANKER / name) != digest:
            raise ValueError(f'{name} does not match the pinned reranker')
    tok = Tokenizer.from_file(str(RERANKER / 'tokenizer.json'))
    tok.enable_truncation(512)
    session = onnxruntime.InferenceSession(str(RERANKER / 'model.onnx'), providers=['CPUExecutionProvider'])
    names = {i.name for i in session.get_inputs()}

    def logits(query, texts):
        import numpy as np
        if not texts:
            return []
        enc = [tok.encode(query, t) for t in texts]
        width = max(len(e.ids) for e in enc)
        pad = lambda rows: np.array([r + [0] * (width - len(r)) for r in rows], dtype=np.int64)
        feed = {'input_ids': pad([e.ids for e in enc]), 'attention_mask': pad([e.attention_mask for e in enc])}
        if 'token_type_ids' in names:
            feed['token_type_ids'] = pad([e.type_ids for e in enc])
        return [float(x) for x in session.run(None, feed)[0][:, 0]]
    return logits


def score(args):
    logits = reranker()
    records = [json.loads(line) for line in args.input.read_text(encoding='utf-8').splitlines()]
    for record in records:
        for row in record['rows']:
            for hit, value in zip(row['hits'], logits(row['query'], [h['text'] for h in row['hits']])):
                hit['logit'] = value
    out = args.input.with_suffix('.scored.jsonl')
    out.write_text(''.join(json.dumps(r) + '\n' for r in records), encoding='utf-8')
    print(f'{out}: {sum(len(r["rows"]) for r in records)} rows scored')


def measures(records, t):
    """Per family (source question): positives found, and no-answer rows that still inject (t=None: ungated)."""
    fam = []
    for record in records:
        f = {'pos': 0, 'pos_hit': 0, 'abs': 0, 'abs_inj': 0, 'cross': 0, 'cross_inj': 0}
        for row in record['rows']:
            kept = [h for h in row['hits'] if t is None or h['logit'] >= t]
            if row['label'] == 'positive':
                f['pos'] += 1
                f['pos_hit'] += any(h['relevant'] for h in kept)
            else:
                f[row['label']] += 1
                f[row['label'] + '_inj'] += bool(kept)
        fam.append(f)
    return fam


def rates(fam):
    s = {k: sum(f[k] for f in fam) for k in fam[0]}
    neg = s['abs'] + s['cross']
    return {'any3': s['pos_hit'] / s['pos'], 'neg_injection': (s['abs_inj'] + s['cross_inj']) / neg,
            'abs_injection': s['abs_inj'] / s['abs'] if s['abs'] else None,
            'cross_injection': s['cross_inj'] / s['cross'], 'positives': s['pos'], 'no_answer': neg}


def bootstrap(gated, ungated, key, seed=20261117, n=10_000):
    rng, values = random.Random(seed), []
    for _ in range(n):
        idx = [rng.randrange(len(gated)) for _ in gated]
        g, u = rates([gated[i] for i in idx]), rates([ungated[i] for i in idx])
        values.append(g[key] - u[key])
    values.sort()
    return [values[int(0.025 * n)], values[int(0.975 * n) - 1]]


def analyse(args):
    records = [json.loads(line) for line in args.input.read_text(encoding='utf-8').splitlines()]
    ungated = measures(records, None)
    base = rates(ungated)
    if args.threshold is None:
        # Dev rule: lowest no-answer injection with any@3 within 5 points of ungated. Ties: the lower threshold (keeps more).
        candidates = sorted({h['logit'] for r in records for row in r['rows'] for h in row['hits']})
        best = None
        for t in candidates:
            r = rates(measures(records, t))
            if base['any3'] - r['any3'] <= 0.05 + 1e-12 and (best is None or r['neg_injection'] < best[1]['neg_injection']):
                best = (t, r)
        t = best[0]
    else:
        t = args.threshold
    gated = measures(records, t)
    result = rates(gated)
    report = {'threshold': t, 'ungated': base, 'gated': result,
              'any3_loss_points': round(100 * (base['any3'] - result['any3']), 2),
              'any3_diff_ci95': bootstrap(gated, ungated, 'any3'),
              'neg_injection_diff_ci95': bootstrap(gated, ungated, 'neg_injection')}
    if args.threshold is not None:
        report['passes'] = result['neg_injection'] <= 0.50 and report['any3_loss_points'] <= 5
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    assert rates([{'pos': 2, 'pos_hit': 1, 'abs': 1, 'abs_inj': 1, 'cross': 1, 'cross_inj': 0}])['neg_injection'] == 0.5
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest='cmd', required=True)
    c = sub.add_parser('collect')
    c.add_argument('--split', choices=['dev', 'heldout'], required=True)
    c.add_argument('--output', type=Path, required=True)
    c.add_argument('--limit', type=int, default=0, help='first N questions, smoke tests only')
    c.add_argument('--workers', type=int, default=4)
    s = sub.add_parser('score')
    s.add_argument('input', type=Path)
    a = sub.add_parser('analyse')
    a.add_argument('input', type=Path)
    a.add_argument('--threshold', type=float)
    args = p.parse_args()
    {'collect': collect, 'score': score, 'analyse': analyse}[args.cmd](args)
