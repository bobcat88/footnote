"""Inventory explicitly supplied local artifact paths; no inference about equal stored content."""
from __future__ import annotations
import argparse
import datetime as dt
import json
import os
from pathlib import Path


def measure(paths):
    entries = []
    seen = set()
    for label, supplied in paths:
        root = Path(supplied)
        if root.is_symlink() or not root.exists():
            raise ValueError(f'missing or symlinked artifact: {root}')
        root = root.resolve(strict=True)
        files = []
        if root.is_file():
            files.append(root)
        elif root.is_dir():
            def fail(error):
                raise error
            for base, dirs, names in os.walk(root, followlinks=False, onerror=fail):
                if any((Path(base) / name).is_symlink() for name in dirs):
                    raise ValueError(f'symlinked directory inside artifact: {base}')
                files.extend(Path(base) / name for name in names)
        else:
            raise ValueError(f'unsupported artifact: {root}')
        count = total = 0
        for file in files:
            if file.is_symlink():
                raise ValueError(f'symlink inside artifact: {file}')
            resolved = file.resolve(strict=True)
            if resolved in seen:
                raise ValueError(f'overlapping artifact file: {file}')
            seen.add(resolved)
            size = file.stat().st_size
            total += size
            count += 1
        entries.append({'label': label, 'path': str(root), 'files': count, 'bytes': total})
    return entries


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--artifact', action='append', required=True, metavar='LABEL=PATH')
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    if args.output.exists():
        ap.error('output exists')
    artifacts = []
    for pair in args.artifact:
        label, sep, path = pair.partition('=')
        if not sep or not label or not path:
            ap.error('artifact must be LABEL=PATH')
        artifacts.append((label, path))
    if len({label for label, _ in artifacts}) != len(artifacts):
        ap.error('artifact labels must be unique')
    entries = measure(artifacts)
    out = {'protocol': 'artifact-footprint-v1', 'created_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
           'note': 'File sizes only. Include DB, WAL, indexes, executable and model files separately. This does not measure peak RAM, allocation, temporary files, equivalent stored content, or concurrent writes.',
           'artifacts': entries, 'total_bytes': sum(x['bytes'] for x in entries)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf8') as f:
        json.dump(out, f, indent=2)
    for entry in entries:
        print(entry['label'], entry['bytes'], 'bytes')
    print('wrote', args.output)


if __name__ == '__main__':
    main()
