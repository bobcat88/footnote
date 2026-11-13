"""Run local ICM in an isolated keyword-only configuration over LongMemEval sessions."""
from __future__ import annotations
import argparse
import datetime as dt
import hashlib
import json
import os
import platform
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

BENCH = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(BENCH), str(Path(__file__).resolve().parent)]
import fetch
import longmem_footnote as shared
from benchmark_checkpoint import Checkpoint


def split_utf8(text, max_bytes=60000):
    remaining = text.encode('utf-8')
    while remaining:
        end = min(max_bytes, len(remaining))
        while end and end < len(remaining) and remaining[end] & 0xc0 == 0x80:
            end -= 1
        if not end:
            raise ValueError('limit too small for a UTF-8 character')
        yield remaining[:end].decode('utf-8')
        remaining = remaining[end:]


def post(opener, base, path, payload):
    request = urllib.request.Request(base + path + '?format=json', data=json.dumps(payload).encode('utf8'),
                                     headers={'Content-Type': 'application/json'})
    with opener.open(request, timeout=120) as response:
        return json.load(response)


def run_one(binary, root, question):
    corpus = shared.user_sessions(question)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    cfg = root / 'config.toml'
    cfg.write_text('[extraction.summarizer]\nprovider = "none"\n', encoding='utf8')
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith('ICM_')}
    env.update({'ICM_CONFIG': str(cfg), 'HF_HUB_OFFLINE': '1', 'TRANSFORMERS_OFFLINE': '1'})
    cmd = [str(binary), '--db', str(root / 'memory.db'), '--no-embeddings', 'serve', '--http', f'127.0.0.1:{port}']
    with (root / 'server.log').open('wb') as log:
        proc = subprocess.Popen(cmd, cwd=str(root), env=env, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                                creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        try:
            deadline = time.monotonic() + 120
            while True:
                if proc.poll() is not None:
                    log.flush()
                    detail = (root / 'server.log').read_text(encoding='utf8', errors='replace')[-1500:]
                    raise RuntimeError(f'ICM exited with code {proc.returncode}: {detail}')
                try:
                    with opener.open(base + '/health', timeout=2) as response:
                        if response.status == 200:
                            break
                except (urllib.error.URLError, OSError):
                    if time.monotonic() >= deadline:
                        raise TimeoutError('ICM server did not become healthy')
                    time.sleep(.1)
            # Do not silently label an old binary as v2.
            try:
                post(opener, base, '/recall', {'query': 'capability probe', 'engine': '__invalid_engine__'})
            except urllib.error.HTTPError as exc:
                if exc.code != 400:
                    raise
            else:
                raise RuntimeError('ICM did not reject an invalid engine; v2 support unverified')
            mapping = {}
            start = time.perf_counter()
            for sid, (_, text) in corpus.items():
                for part in split_utf8(text):
                    stored = post(opener, base, '/store', {'topic': 'benchmark', 'content': part})
                    if not isinstance(stored, list) or not stored or 'id' not in stored[0]:
                        raise ValueError('unexpected ICM store response')
                    mid = stored[0]['id']
                    if mid in mapping:
                        raise ValueError('ICM merged memory IDs; session mapping would be ambiguous')
                    mapping[mid] = sid
            ingest_ms = (time.perf_counter() - start) * 1000
            start = time.perf_counter()
            rows = post(opener, base, '/recall', {'query': question['question'], 'limit': 50, 'engine': 'v2'})
            recall_ms = (time.perf_counter() - start) * 1000
            if not isinstance(rows, list):
                raise ValueError('unexpected ICM recall response')
            ranked = []
            for row in rows:
                if row['id'] not in mapping:
                    raise ValueError('unknown memory ID from ICM')
                ranked.append(mapping[row['id']])
            return ranked, {'sessions': len(corpus), 'units': len(mapping), 'ingest_ms': ingest_ms,
                            'warm_query_ms': recall_ms, 'database_bytes': (root / 'memory.db').stat().st_size,
                            'wal_bytes': (root / 'memory.db-wal').stat().st_size if (root / 'memory.db-wal').exists() else 0,
                            'shm_bytes': (root / 'memory.db-shm').stat().st_size if (root / 'memory.db-shm').exists() else 0}
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    proc.kill(); proc.wait(timeout=10)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--binary', type=Path, required=True, help='path to a trusted ICM executable')
    ap.add_argument('--split', choices=('all', 'dev', 'heldout'), default='all')
    ap.add_argument('--max-questions', type=int, default=0)
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--resume', action='store_true', help='resume an identity-matching checkpoint')
    args = ap.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file() or args.max_questions < 0 or args.output.exists():
        ap.error('provide an existing binary, nonnegative limit and a new output path')
    questions = fetch.load_longmemeval()
    total = len(questions)
    if args.split != 'all':
        questions = [q for q in questions if fetch.is_dev(q['question_id']) == (args.split == 'dev')]
    if args.max_questions:
        questions = questions[:args.max_questions]
    identity = {'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'dataset_pin': fetch.read_lock()['longmemeval_s'],
                'question_ids': [q['question_id'] for q in questions], 'split': args.split,
                'mode': 'keyword-only-v2', 'limit': 50,
                'session_adapter_sha256': hashlib.sha256(Path(shared.__file__).read_bytes()).hexdigest(),
                'checkpoint_sha256': hashlib.sha256(Path(__file__).with_name('benchmark_checkpoint.py').read_bytes()).hexdigest()}
    checkpoint = Checkpoint(args.output, identity, args.resume)
    rankings, measurements = checkpoint.rankings, checkpoint.measurements
    with tempfile.TemporaryDirectory(prefix='footnote-icm-eval-') as tmp:
        base = Path(tmp)
        for n, question in enumerate(questions, 1):
            if question['question_id'] in rankings:
                print(f'{n}/{len(questions)} resumed', flush=True)
                continue
            folder = base / str(n)
            folder.mkdir()
            ranking, measurement = run_one(binary, folder, question)
            checkpoint.save(question['question_id'], ranking, measurement)
            print(f'{n}/{len(questions)}', flush=True)
    data = {'format': 'icm-longmem-session-v1', 'rankings': rankings, 'questions_total': total,
            'partial': len(questions) != total, 'split': args.split, 'created_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
            'dataset_pin': fetch.read_lock()['longmemeval_s'],
            'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
            'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'platform': platform.platform(),
            'config': {'mode': 'keyword-only', 'engine': 'v2', 'limit': 50, 'dates': 'not supplied',
                       'input': 'user turns joined with newline; split sessions over 60000 UTF-8 bytes',
                       'result_unit': 'raw session IDs, including duplicates for split sessions',
                       'ingestion': 'isolated HTTP server and database per question; explicit empty config, no embeddings'},
            'measurements': measurements}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print('wrote', args.output)


if __name__ == '__main__':
    main()
