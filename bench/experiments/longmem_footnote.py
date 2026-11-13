"""Run shipped Footnote over isolated user-session haystacks. No ICM configuration is touched."""
from __future__ import annotations
import argparse
import datetime as dt
import json
import hashlib
import platform
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

BENCH = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(BENCH), str(Path(__file__).resolve().parent)]
import engines
import fetch
import metrics
from benchmark_checkpoint import Checkpoint


class IsolatedFootnote(engines.ShippedEnsemble):
    def __init__(self, state):
        super().__init__()
        self.state = state

    def _start(self):
        self.state.mkdir(parents=True, exist_ok=True)
        env = {k: v for k, v in os.environ.items() if k != 'FOOTNOTE_SCAN_LIMIT'}
        env['BORG_STATE_DIR'] = str(self.state)
        self._proc = subprocess.Popen([str(engines.EXE), '--db', str(self.db), 'mcp'], stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
                                      encoding='utf-8', bufsize=1, env=env)
        self._rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {},
                                 'clientInfo': {'name': 'icm-comparison', 'version': '1'}})
        self._rpc('notifications/initialized', notify=True)


def user_sessions(question):
    ids, sessions = question['haystack_session_ids'], question['haystack_sessions']
    if len(ids) != len(sessions):
        raise ValueError('session ID/content count mismatch')
    corpus = {}
    for sid, turns in zip(ids, sessions):
        if not re.fullmatch(r'[A-Za-z0-9_-]+', sid):
            raise ValueError('unsafe session ID')
        text = '\n'.join(str(turn['content']) for turn in turns if turn['role'] == 'user')
        if not text.strip():
            continue
        item = ('', text)
        if sid in corpus and corpus[sid] != item:
            raise ValueError('duplicate session has different contents')
        corpus[sid] = item
    if not set(question['answer_session_ids']).issubset(corpus):
        raise ValueError('gold session absent after user-turn filtering')
    return corpus


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--split', choices=('all', 'dev', 'heldout'), default='all')
    ap.add_argument('--max-questions', type=int, default=0, help='partial smoke run; 0 selects the whole chosen split')
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--resume', action='store_true', help='resume an identity-matching checkpoint')
    args = ap.parse_args()
    if args.max_questions < 0 or args.output.exists():
        ap.error('use a nonnegative question limit and a new output path')
    questions = fetch.load_longmemeval()
    total = len(questions)
    if args.split != 'all':
        questions = [q for q in questions if fetch.is_dev(q['question_id']) == (args.split == 'dev')]
    if args.max_questions:
        questions = questions[:args.max_questions]
    if not engines.EXE.is_file():
        ap.error('Footnote release executable is missing')
    engines.WORK.mkdir(parents=True, exist_ok=True)
    identity = {'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'binary_sha256': engines.sha256_file(engines.EXE), 'dataset_pin': fetch.read_lock()['longmemeval_s'],
                'question_ids': [q['question_id'] for q in questions], 'split': args.split,
                'model_sha256': {str(d): {f: engines.sha256_file(d / f) for f in ('model.onnx', 'tokenizer.json', 'manifest.json')}
                                 for d in engines.ShippedEnsemble.model_dirs},
                'helper_sha256': engines.sha256_file(engines.REPO / 'tools' / 'embed.py'),
                'adapter_sha256': engines.sha256_file(Path(engines.__file__)),
                'checkpoint_sha256': engines.sha256_file(Path(__file__).with_name('benchmark_checkpoint.py'))}
    checkpoint = Checkpoint(args.output, identity, args.resume)
    results, measurements = checkpoint.rankings, checkpoint.measurements
    with tempfile.TemporaryDirectory(prefix='headtohead-', dir=engines.WORK) as tmp:
        root = Path(tmp)
        for index, question in enumerate(questions, 1):
            qid = question['question_id']
            if not re.fullmatch(r'[A-Za-z0-9_-]+', qid):
                raise ValueError('unsafe question ID')
            if qid in results:
                print(f'{index}/{len(questions)} resumed', flush=True)
                continue
            corpus = user_sessions(question)
            engine = IsolatedFootnote(root / 'state')
            try:
                started = time.perf_counter()
                build = engine.prepare(f'{root.name}/{qid}', corpus)
                build_ms = (time.perf_counter() - started) * 1000
                started = time.perf_counter()
                result = metrics.dedupe(engine.search(qid, question['question'], 20))
                query_ms = (time.perf_counter() - started) * 1000
                if set(result) - set(corpus):
                    raise RuntimeError(f'foreign document in result for {qid}')
                checkpoint.save(qid, result, {'sessions': len(corpus), 'chunks': build['chunks'],
                                              'build_ms': build_ms, 'cold_query_ms': query_ms})
            finally:
                engine.close()
            print(f'{index}/{len(questions)}', flush=True)
        version_probe = IsolatedFootnote(root / 'state')
        versions = version_probe.versions()
    out = {'format': 'footnote-longmem-session-v1', 'rankings': results, 'questions_total': total,
           'partial': len(questions) != total, 'split': args.split, 'created_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
           'dataset_pin': fetch.read_lock()['longmemeval_s'], 'versions': versions,
           'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'platform': platform.platform(),
           'config': {'input': 'user turns joined with newline, one source file per session',
                      'search': version_probe.config, 'result_unit': 'distinct session IDs',
                      'dates': 'not supplied', 'abstention_tagged': 'included',
                      'timing': 'cold first MCP query per fresh index; includes startup',
                      'caveat': 'Footnote chunks sources; ICM indexes sessions as one memory, except oversized sessions'},
           'measurements': measurements}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as f:
        json.dump(out, f, ensure_ascii=False, indent=2)
    print('wrote', args.output)


if __name__ == '__main__':
    main()
