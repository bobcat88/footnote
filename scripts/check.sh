#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if command -v rtk >/dev/null 2>&1; then RUN=(rtk); else RUN=(); fi

pick_python() {
  for cand in "$@"; do
    if command -v "$cand" >/dev/null 2>&1 && "$cand" -c "import sys" >/dev/null 2>&1; then
      printf '%s\n' "$cand"
      return 0
    fi
  done
  return 1
}

PY=""
if [[ -x .venv/Scripts/python.exe ]]; then PY=.venv/Scripts/python.exe
elif [[ -x .venv/bin/python ]]; then PY=.venv/bin/python
fi
PY_SYS="$(pick_python python3 python || true)"

if [[ -z ${TMPDIR:-} ]]; then
  PY_TMP="${PY:-$PY_SYS}"
  if [[ -n $PY_TMP ]]; then
    TMPDIR="$("$PY_TMP" -c 'from pathlib import Path; import os; p=Path(os.environ.get("LOCALAPPDATA",Path.home()/".cache"))/"hermes/cache/scratch";p.mkdir(parents=True,exist_ok=True);print(p.as_posix())')"
  else
    TMPDIR=/tmp
  fi
fi
export TMPDIR TMP="$TMPDIR" TEMP="$TMPDIR"

"${RUN[@]}" cargo fmt --all --check
"${RUN[@]}" cargo clippy --locked --workspace --all-targets -- -D warnings
"${RUN[@]}" cargo clippy --locked --workspace --all-targets --no-default-features --features library -- -D warnings
"${RUN[@]}" cargo clippy --locked --workspace --all-targets --no-default-features --features memory -- -D warnings
"${RUN[@]}" cargo clippy --locked --workspace --all-targets --no-default-features --features code -- -D warnings
"${RUN[@]}" cargo test --locked --workspace --all-targets

if command -v node >/dev/null 2>&1; then
  node --test "apps/launcher/src/**/*.test.js"
  node site/build.mjs
else
  printf '%s\n' 'node not found: skipped Launcher JS tests and site build' >&2
fi

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    "${RUN[@]}" cargo test --locked --manifest-path apps/launcher/src-tauri/Cargo.toml
    ;;
  Linux*)
    if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists webkit2gtk-4.1; then
      "${RUN[@]}" cargo test --locked --manifest-path apps/launcher/src-tauri/Cargo.toml
    fi
    ;;
esac

"${RUN[@]}" cargo build --locked --release

if [[ -n $PY ]]; then
  "$PY" tests/extraction_test.py
  "$PY" tests/hybrid_metrics_test.py
  if command -v uv >/dev/null 2>&1; then
    "${RUN[@]}" uv pip check --python "$PY"
  else
    "$PY" -m pip check
  fi
  "$PY" -c 'import ast; from pathlib import Path; files=list(Path("tools").glob("*.py"))+list(Path("tests").glob("*.py"));[ast.parse(p.read_text(encoding="utf-8"),filename=str(p)) for p in files];print("Python AST checked:",len(files))'
  "$PY" scripts/check_bench_index.py
else
  printf '%s\n' 'Optional .venv missing: skipped Python extraction and metrics checks' >&2
  if [[ -n $PY_SYS ]]; then
    "$PY_SYS" -c 'import ast; from pathlib import Path; files=list(Path("tools").glob("*.py"))+list(Path("tests").glob("*.py"));[ast.parse(p.read_text(encoding="utf-8"),filename=str(p)) for p in files];print("Python AST checked:",len(files))'
    "$PY_SYS" scripts/check_bench_index.py
  fi
fi

"${RUN[@]}" git diff --check
