"""Controlled lexical/fusion ablation. Outputs lab-only results, not product or ICM claims.

Run with the local lab environment and tantivy==0.27.0. See ICM-SUITE.md for limits.
"""
from __future__ import annotations
import argparse
import datetime as dt
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import sqlite3
import subprocess
import sys
import tempfile
import time

import numpy as np
import tantivy

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
sys.path[:0] = [str(BENCH), str(BENCH / 'lab'), str(BENCH / 'dbbake')]
import fetch
import lab
import metrics
import models
from track_a import words, doc_text

POOL = 100
K = 20
FUSIONS = ('cc', 'rrf')


def digest(value):
    data = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode('utf-8')
    return hashlib.sha256(data).hexdigest()


def text_corpus(corpus):
    return {key: doc_text(value) for key, value in corpus.items()}


class Lexical:
    def __init__(self, name, folder, corpus):
        self.name = name
        self.ids = list(corpus)
        self.positions = {key: i for i, key in enumerate(self.ids)}
        if name == 'fts5':
            self.db = sqlite3.connect(folder / 'index.db')
            self.db.execute("CREATE VIRTUAL TABLE t USING fts5(id UNINDEXED, body, tokenize='porter unicode61 remove_diacritics 2')")
            self.db.executemany('INSERT INTO t(id,body) VALUES (?,?)', corpus.items())
            self.db.commit()
        elif name == 'tantivy':
            schema = tantivy.SchemaBuilder()
            schema.add_text_field('id', stored=True, tokenizer_name='raw')
            schema.add_text_field('body', stored=False, tokenizer_name='en_stem')
            self.index = tantivy.Index(schema.build(), path=str(folder))
            writer = self.index.writer(heap_size=200_000_000, num_threads=1)
            for key, body in corpus.items():
                writer.add_document(tantivy.Document(id=key, body=body))
            writer.commit()
            self.index.reload()
            self.searcher = self.index.searcher()
        else:
            raise ValueError(name)

    def scores(self, question):
        result = np.zeros(len(self.ids), dtype=np.float64)
        terms = words(question)
        if not terms:
            return result
        if self.name == 'fts5':
            expr = ' OR '.join('"' + t + '"' for t in terms)
            rows = self.db.execute('SELECT id,bm25(t) AS score FROM t WHERE t MATCH ? ORDER BY score LIMIT ?',
                                   (expr, min(len(self.ids), 1000)))
            for key, score in rows:
                result[self.positions[key]] = -score
        else:
            query = self.index.parse_query(' '.join(terms), ['body'])
            for score, address in self.searcher.search(query, min(len(self.ids), 1000)).hits:
                self_id = self.searcher.doc(address)['id'][0]
                result[self.positions[self_id]] = score
        return result

    def close(self):
        if self.name == 'fts5':
            self.db.close()


def vectors(corpus, queries, model):
    # Fail closed: the upstream registry downloads missing model files automatically.
    spec = models.MODELS[model]
    folder = (models.REPO / spec['local']) if 'local' in spec else (models.CACHE / model)
    if not all((folder / f).is_file() for f in ('model.onnx', 'tokenizer.json')):
        raise RuntimeError(f'model files absent at {folder}; provision separately before running')
    locked = models.read_lock().get(model)
    if 'local' not in spec and (not locked or any(f not in locked.get('files', {}) for f in ('model.onnx', 'tokenizer.json'))):
        raise RuntimeError(f'model {model} is not hash-locked; refusing automatic provisioning')
    engine = lab.LabEngine('ablation', {'chunker': {'kind': 'whole'}, 'lexical': {'on': False},
                                        'dense': {'model': model}, 'fusion': {'kind': 'dense'}})
    started = time.perf_counter()
    info = engine.prepare('ablation', corpus)
    engine.warm_queries(list(queries.values()))
    elapsed = time.perf_counter() - started
    manifest = {f: models.sha256_file(folder / f) for f in ('model.onnx', 'tokenizer.json')}
    return engine.emb, engine._qcache, {'files_sha256': manifest, 'prepare_s': round(elapsed, 3), 'build': info}


def run_arms(corpus, queries, qrels, doc_vectors, query_vectors):
    if list(corpus) != list(dict.fromkeys(corpus)) or doc_vectors.shape[0] != len(corpus):
        raise ValueError('document/vector mismatch')
    ids = list(corpus)
    if set(queries) != set(qrels) or not queries:
        raise ValueError('queries and judgments must have exactly the same IDs')
    rows = []
    for backend in ('fts5', 'tantivy'):
        with tempfile.TemporaryDirectory(prefix='footnote-ablation-') as tmp:
            path = Path(tmp)
            start = time.perf_counter()
            engine = Lexical(backend, path, text_corpus(corpus))
            build_s = time.perf_counter() - start
            try:
                results = {kind: {} for kind in FUSIONS}
                lexical_times, dense_times = [], []
                for qid, query in queries.items():
                    t = time.perf_counter(); lex = engine.scores(query); lexical_times.append((time.perf_counter() - t) * 1000)
                    t = time.perf_counter(); dense = doc_vectors @ query_vectors[query]; dense_times.append((time.perf_counter() - t) * 1000)
                    for fusion in FUSIONS:
                        cfg = {'kind': fusion, 'pool': POOL, 'k': 60, 'alpha': 0.55, 'norm': 'zscore', 'dense_from_lexical': 0}
                        result = [ids[int(i)] for i in lab.fuse(lex, dense, cfg, K)]
                        if len(result) != len(set(result)) or not set(result).issubset(corpus):
                            raise RuntimeError('invalid ranking')
                        results[fusion][qid] = result
                checks = {kind: True for kind in FUSIONS}
                check_items = list(queries.items())[:200]
                for qid, query in check_items:
                    lex = engine.scores(query)
                    dense = doc_vectors @ query_vectors[query]
                    for fusion in FUSIONS:
                        cfg = {'kind': fusion, 'pool': POOL, 'k': 60, 'alpha': 0.55, 'norm': 'zscore', 'dense_from_lexical': 0}
                        repeated = [ids[int(i)] for i in lab.fuse(lex, dense, cfg, K)]
                        checks[fusion] &= repeated == results[fusion][qid]
                size = sum(f.stat().st_size for f in path.rglob('*') if f.is_file())
                for fusion, rankings in results.items():
                    rows.append({'lexical': backend, 'fusion': fusion, 'metrics': metrics.evaluate(rankings, qrels),
                                 'rankings': rankings, 'rankings_sha256': digest(rankings),
                                 'determinism': {'queries_checked': len(check_items), 'identical_rankings': checks[fusion]},
                                 'lexical_p50_ms': float(np.median(lexical_times)), 'lexical_p95_ms': float(np.percentile(lexical_times, 95)),
                                 'dense_p50_ms': float(np.median(dense_times)), 'dense_p95_ms': float(np.percentile(dense_times, 95)),
                                 'lexical_build_s': build_s, 'lexical_artifact_bytes': size})
            finally:
                engine.close()
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--dataset', default='scifact', choices=('scifact', 'nfcorpus', 'alloprof', 'syntec', 'mix-fr-en'))
    ap.add_argument('--split', default='dev', choices=('dev', 'heldout', 'test'))
    ap.add_argument('--model', default='bekko-a8m', choices=sorted(models.MODELS))
    ap.add_argument('--max-queries', type=int, default=0, help='partial smoke run, zero = all queries')
    ap.add_argument('--output', required=True, type=Path, help='new output file; never overwritten')
    args = ap.parse_args()
    if args.max_queries < 0:
        ap.error('--max-queries must be nonnegative')
    if args.output.exists():
        ap.error('output exists; choose a new path')
    corpus, queries, qrels = fetch.load(args.dataset, args.split)
    if args.max_queries:
        queries = dict(list(queries.items())[:args.max_queries]); qrels = {key: qrels[key] for key in queries}
    dv, qv, model_info = vectors(corpus, queries, args.model)
    rows = run_arms(corpus, queries, qrels, dv, qv)
    try:
        commit = subprocess.run(['rtk', 'git', 'rev-parse', 'HEAD'], cwd=BENCH.parent, capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = None
    data = {'protocol': 'icm-ablation-v1', 'created_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
            'scope': 'lab ablation; ranking-only arm timings exclude query encoding, MCP and fusion; not ICM or shipped binary',
            'dataset': args.dataset, 'split': args.split, 'partial': bool(args.max_queries), 'documents': len(corpus), 'queries': len(queries),
            'input_sha256': digest({'corpus': corpus, 'queries': queries, 'qrels': qrels}), 'dataset_pin': fetch.dataset_pin(args.dataset),
            'model': args.model, 'model_info': model_info, 'pool': POOL, 'result_limit': K,
            'fusion': {'cc_alpha': 0.55, 'cc_norm': 'zscore', 'rrf_k': 60, 'dense_candidates': 'full scan'},
            'versions': {'python': platform.python_version(), 'numpy': np.__version__, 'tantivy': importlib.metadata.version('tantivy'),
                         'sqlite': sqlite3.sqlite_version, 'os': platform.platform(), 'cpu': platform.processor(), 'logical_cpus': os.cpu_count(), 'commit': commit},
            'script_sha256': models.sha256_file(Path(__file__)), 'rows': rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    for row in rows:
        print(row['lexical'], row['fusion'], 'nDCG@10', round(row['metrics']['ndcg@10'], 4), 'recall@10', round(row['metrics']['recall@10'], 4))
    print('wrote', args.output)


if __name__ == '__main__':
    main()
