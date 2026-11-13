"""Explicit, identity-checked checkpoint for long read-only benchmark runs."""
import json
import os
from pathlib import Path
import tempfile


class Checkpoint:
    def __init__(self, output, identity, resume=False):
        self.path = Path(str(output) + '.checkpoint.json')
        self.identity = identity
        if self.path.exists():
            if not resume:
                raise ValueError(f'checkpoint exists; use --resume: {self.path}')
            data = json.loads(self.path.read_text(encoding='utf8'))
            if data.get('identity') != identity:
                raise ValueError('checkpoint identity differs from this run')
            self.rankings = data['rankings']
            self.measurements = data['measurements']
            if set(self.rankings) != {x['question_id'] for x in self.measurements} or len(self.rankings) != len(self.measurements):
                raise ValueError('checkpoint rankings and measurements mismatch')
        else:
            if resume:
                raise ValueError(f'no checkpoint to resume: {self.path}')
            self.rankings = {}
            self.measurements = []
        if not set(self.rankings).issubset(identity['question_ids']):
            raise ValueError('checkpoint contains an unknown question')

    def save(self, qid, ranking, measurement):
        if qid in self.rankings or qid not in self.identity['question_ids']:
            raise ValueError('duplicate or unknown checkpoint question')
        self.rankings[qid] = ranking
        self.measurements.append({'question_id': qid, **measurement})
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temp_path = None
        try:
            with tempfile.NamedTemporaryFile(mode='w', encoding='utf8', dir=self.path.parent, delete=False, suffix='.tmp') as f:
                temp_path = Path(f.name)
                json.dump({'identity': self.identity, 'rankings': self.rankings, 'measurements': self.measurements}, f, ensure_ascii=False)
                f.flush()
                os.fsync(f.fileno())
            os.replace(temp_path, self.path)
        finally:
            if temp_path is not None and temp_path.exists():
                temp_path.unlink()
