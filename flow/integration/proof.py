"""Repeatable real-Footnote setup, context preparation and reviewed proof admission."""

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

from .footnote import Footnote, prepare, sha


INITIAL = '''def merge_intervals(intervals):
    result = []
    for start, end in sorted(intervals):
        if result and start <= result[-1][1]:
            result[-1] = (result[-1][0], max(end, result[-1][1]))
        else:
            result.append((start, end))
    return result
'''
CONTRACT = "merge_intervals contract v1. Intervals are half-open [start,end). Overlapping intervals merge. Touching endpoints remain separate by default because each boundary is a distinct billing segment. Keep input unchanged. Endpoints must be integers, not booleans, with start < end. Reject malformed inputs with ValueError. Empty input returns [].\n"
SEED = "Project decision for merge_intervals: preserve touching boundaries by default; adjacent intervals represent separate billing segments. A previous <= comparison incorrectly merged them."


def setup(root, binary):
    root = Path(root).resolve()
    root.mkdir(parents=True, exist_ok=False)
    (root / "bin").mkdir()
    shutil.copy2(binary, root / "bin/footnote.exe")
    for folder in ("workspace", "docs"):
        (root / folder).mkdir()
    (root / "workspace/pyproject.toml").write_text('[project]\nname="flow-interval-proof"\nversion="0.0.1"\nrequires-python=">=3.12"\n')
    (root / "workspace/intervals.py").write_text(INITIAL)
    (root / "docs/interval-contract.txt").write_text(CONTRACT)
    f = Footnote(root / "bin/footnote.exe", root)
    f.call("remember", db=f.state / "memory.db", payload=[{"text": SEED, "kind": "fix", "scope": "flow-interval-proof", "observed_at": "2026-11-17", "code": ["intervals.py", "merge_intervals"]}])
    f.call("index", "--root", str(root / "docs"), "--allow", str(root / "docs"), db=root / "library.db")
    f.call("code-index", str(root / "workspace"))
    (root / "setup-calls.json").write_text(json.dumps(f.calls, indent=2))


def context(root, session):
    root = Path(root).resolve()
    f = Footnote(root / "bin/footnote.exe", root)
    receipt = prepare(f, project="flow-interval-proof", workspace=root / "workspace", symbol="merge_intervals", query="merge_intervals", documents=[root / "docs/interval-contract.txt"])
    destination = root / f"context-session-{session}.json"
    if destination.exists():
        raise ValueError("context receipt already exists; retain it and choose a new proof directory")
    destination.write_text(json.dumps(receipt, ensure_ascii=False, indent=2), encoding="utf-8", newline="\n")
    (root / f"prepare-session-{session}-calls.json").write_text(json.dumps(f.calls, indent=2))
    return receipt


def admit(root, rationale):
    root = Path(root).resolve()
    if not rationale.strip():
        raise ValueError("explicit trusted host admission rationale required")
    destination = root / "admission-session-1.json"
    if destination.exists():
        raise ValueError("admission already recorded; do not replay it")
    work = root / "workspace"
    candidate = json.loads((work / "lesson.json").read_text(encoding="utf-8"))
    if not isinstance(candidate.get("text"), str) or not candidate["text"].strip() or set(candidate.get("evidence", [])) != {"intervals.py", "test_intervals.py"}:
        raise ValueError("candidate must identify exactly the owned verified artifacts")
    evidence = {name: sha(work / name) for name in candidate["evidence"]}
    result = subprocess.run(["rtk", "proxy", sys.executable, "-m", "integration.check_intervals", str(work)], capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise ValueError("external verification failed; memory admission refused")
    if any(sha(work / name) != value for name, value in evidence.items()):
        raise ValueError("artifact changed during verification")
    f = Footnote(root / "bin/footnote.exe", root)
    stored = f.call("remember", db=f.state / "memory.db", payload=[{"text": candidate["text"], "kind": "fix", "scope": "flow-interval-proof", "observed_at": "2026-11-17", "session": "flow-integrated-session-1:" + evidence["intervals.py"][:16], "code": ["intervals.py", "merge_intervals"], "tags": ["validation"]}])
    receipt = {"decision": "admit", "rationale": rationale, "artifacts": evidence, "verification_exit_code": result.returncode, "verification_transcript_sha256": hashlib.sha256(result.stderr.encode()).hexdigest(), "memory_receipt": stored, "scope": "trusted host decision in an isolated proof; not an authenticated production admission service"}
    destination.write_text(json.dumps(receipt, indent=2))
    return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["setup", "context", "admit", "reindex"])
    parser.add_argument("root")
    parser.add_argument("--binary")
    parser.add_argument("--session", type=int, choices=[1, 2], default=1)
    parser.add_argument("--rationale", default="")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    if args.action == "setup":
        if not args.binary: parser.error("setup needs --binary")
        setup(root, args.binary)
    elif args.action == "context": context(root, args.session)
    elif args.action == "admit": admit(root, args.rationale)
    else: Footnote(root / "bin/footnote.exe", root).call("code-index", str(root / "workspace"))
    print(json.dumps({"action": args.action, "completed": True, "root": str(root)}))
