#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rtk --version
if [[ -x .venv/Scripts/python.exe ]]; then PY=.venv/Scripts/python.exe
elif [[ -x .venv/bin/python ]]; then PY=.venv/bin/python
else printf '%s\n' 'Missing optional extraction environment: create .venv and install requirements-extract.lock' >&2; exit 1
fi
if [[ -z ${TMPDIR:-} ]]; then
  TMPDIR="$($PY -c 'from pathlib import Path; import os; p=Path(os.environ.get("LOCALAPPDATA",Path.home()/".cache"))/"hermes/cache/scratch";p.mkdir(parents=True,exist_ok=True);print(p.as_posix())')"
fi
export TMPDIR TMP="$TMPDIR" TEMP="$TMPDIR"
rtk cargo fmt --all --check
rtk cargo clippy --locked --workspace --all-targets -- -D warnings
rtk cargo clippy --locked --workspace --all-targets --no-default-features --features library -- -D warnings
rtk cargo clippy --locked --workspace --all-targets --no-default-features --features memory -- -D warnings
rtk cargo test --locked --workspace --all-targets
rtk cargo build --locked --release
"$PY" tests/extraction_test.py
"$PY" tests/hybrid_metrics_test.py
rtk uv pip check --python "$PY"
"$PY" -c 'import ast; from pathlib import Path; files=list(Path("tools").glob("*.py"))+list(Path("tests").glob("*.py"));[ast.parse(p.read_text(encoding="utf-8"),filename=str(p)) for p in files];print("Python AST checked:",len(files))'
"$PY" scripts/check_bench_index.py
rtk git diff --check
