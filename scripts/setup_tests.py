"""Prepare everything the full test run needs (#206). A fresh checkout passes `cargo test` without it: the tests that need
these skip and say why. Run from the repository root: python scripts/setup_tests.py

1. `.venv` with the pinned extraction packages (requirements-extract.lock), for the PDF and Office helper tests.
2. The embedding models the search tests use (minilm, e5-small, gte-small), hash-checked by tools/provision_embeddings.py.
3. Reports which code indexers are on PATH; installing them is left to you, since each is its own tool.
"""
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def run(*cmd):
    print("+", " ".join(str(c) for c in cmd))
    subprocess.run([str(c) for c in cmd], cwd=ROOT, check=True)


venv = ROOT / ".venv"
python = venv / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
if not python.exists():
    run(sys.executable, "-m", "venv", venv)
run(python, "-m", "pip", "install", "-r", ROOT / "requirements-extract.lock")
run(python, ROOT / "tools/provision_embeddings.py", "minilm", "e5-small", "gte-small")

for tool, hint in [
    ("rust-analyzer", "rustup component add rust-analyzer"),
    ("scip-typescript", "npm install -g @sourcegraph/scip-typescript"),
    ("scip-python", "npm install -g @sourcegraph/scip-python"),
    ("node", "install Node.js (the Python indexer's Windows loader test)"),
]:
    print(f"{tool:16} {'found' if shutil.which(tool) else 'missing: ' + hint}")
print("Ready. The model-backed hook test is ignored by default: cargo test --test hook -- --ignored")
