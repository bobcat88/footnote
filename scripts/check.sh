#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

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

cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked --workspace --all-targets --no-default-features --features library -- -D warnings
cargo clippy --locked --workspace --all-targets --no-default-features --features memory -- -D warnings
cargo clippy --locked --workspace --all-targets --no-default-features --features code -- -D warnings
cargo test --locked --workspace --all-targets

if command -v node >/dev/null 2>&1; then
  node --test "apps/launcher/src/**/*.test.js"
  node site/build.mjs
else
  printf '%s\n' 'node not found: skipped Launcher JS tests and site build' >&2
fi

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    cargo test --locked --manifest-path apps/launcher/src-tauri/Cargo.toml
    ;;
  Linux*)
    if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists webkit2gtk-4.1; then
      cargo test --locked --manifest-path apps/launcher/src-tauri/Cargo.toml
    fi
    ;;
esac

cargo build --locked --release

if [[ -n $PY ]]; then
  if command -v uv >/dev/null 2>&1; then
    uv pip check --python "$PY"
  else
    "$PY" -m pip check
  fi
  "$PY" -c 'import ast; from pathlib import Path; files=list(Path("tools").glob("*.py"))+list(Path("scripts").glob("*.py"));[ast.parse(p.read_text(encoding="utf-8"),filename=str(p)) for p in files];print("Python AST checked:",len(files))'
  "$PY" scripts/check_bench_index.py
else
  printf '%s\n' 'Optional .venv missing: skipped Python environment check' >&2
  if [[ -n $PY_SYS ]]; then
    "$PY_SYS" -c 'import ast; from pathlib import Path; files=list(Path("tools").glob("*.py"))+list(Path("scripts").glob("*.py"));[ast.parse(p.read_text(encoding="utf-8"),filename=str(p)) for p in files];print("Python AST checked:",len(files))'
    "$PY_SYS" scripts/check_bench_index.py
  fi
fi

git diff --check
