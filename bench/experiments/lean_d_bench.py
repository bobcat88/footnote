#!/usr/bin/env python3
"""Head-to-head benchmark for #243 (Lean D: `footnote run` vs `rtk` 1.2.0).

Decision rule (frozen before results):
- Ship a command family only if marker recall >= 99.0% AND mean byte saving >= 20.0%
  across its corpus, with 0 bloat cases (output > raw) and 0 credential leaks to disk.
- Compare head-to-head against `rtk` on the exact same real commands in this repository.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def strip_ansi(s: str) -> str:
    return ANSI_RE.sub("", s)


def run_proc(cmd: list[str], cwd: Path, env: dict[str, str], stdin_text: str | None = None) -> tuple[str, int, float]:
    t0 = time.perf_counter()
    proc = subprocess.run(
        cmd,
        cwd=cwd,
        env=env,
        input=stdin_text,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    dt_ms = (time.perf_counter() - t0) * 1000.0
    combined = proc.stdout if not proc.stderr else (
        f"{proc.stderr}\n{proc.stdout}" if proc.stdout else proc.stderr
    )
    return combined, proc.returncode, dt_ms


def marker_recall(output: str, markers: list[str]) -> tuple[int, int]:
    if not markers:
        return 0, 0
    clean = strip_ansi(output)
    hit = sum(1 for m in markers if m in clean)
    return hit, len(markers)


def make_fixtures() -> list[dict]:
    """Deterministic multi-family corpus combining realistic failure/lint/status traces."""
    # 1. Cargo test failure fixture (realistic large workspace output + failure block)
    ct_lines = [f"   Compiling dep-crate-{i:02d} v0.3.{i}" for i in range(60)]
    ct_lines.append("    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.42s")
    ct_lines.append("     Running unittests src/lib.rs (target/debug/deps/demo-1234.exe)")
    ct_lines.append("")
    ct_lines.append("running 45 tests")
    for i in range(44):
        ct_lines.append(f"test engine::case_{i:02d} ... ok")
    ct_lines.extend([
        "test engine::strict_cutoff_equivalence ... FAILED",
        "",
        "failures:",
        "",
        "---- engine::strict_cutoff_equivalence stdout ----",
        "thread 'engine::strict_cutoff_equivalence' panicked at crates/search/src/semantic.rs:512:9:",
        "assertion `left == right` failed: shortlist diverged from exact scan",
        "  left: [101, 204, 309]",
        " right: [101, 309, 204]",
        "",
        "failures:",
        "    engine::strict_cutoff_equivalence",
        "",
        "test result: FAILED. 44 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s",
    ])
    ct_raw = "\n".join(ct_lines) + "\n"

    # 2. Cargo clippy / build fixture with error + warning spans
    cb_lines = [f"   Compiling workspace-dep-{i:02d} v1.2.{i}" for i in range(70)]
    cb_lines.extend([
        "    Checking footnote-core v0.1.0",
        "error[E0308]: mismatched types",
        "  --> crates/core/src/state.rs:88:12",
        "   |",
        "88 |     return Ok(42);",
        "   |               ^^ expected `PathBuf`, found integer",
        "   |",
        "   = note: expected enum `Result<PathBuf, anyhow::Error>`",
        "warning: unused variable: `tmp`",
        "  --> crates/core/src/state.rs:92:9",
        "   |",
        "92 |     let tmp = 1;",
        "   |         ^^^ help: if this is intentional, prefix it with an underscore: `_tmp`",
        "error: could not compile `footnote-core` (lib) due to 1 previous error; 1 warning emitted",
    ])
    cb_raw = "\n".join(cb_lines) + "\n"

    # 3. Pytest failure fixture
    py_lines = [
        "============================= test session starts =============================",
        "platform win32 -- Python 3.12.7, pytest-8.3.4, pluggy-1.5.0",
        "rootdir: /repo",
        "configfile: pyproject.toml",
        "plugins: cov-5.0.0, anyio-4.4.0",
        "collecting ... collected 38 items",
        "",
    ]
    for i in range(37):
        py_lines.append(f"tests/test_pipeline.py::test_stage_{i:02d} PASSED [{int((i+1)*100/38):3d}%]")
    py_lines.extend([
        "tests/test_pipeline.py::test_secret_guard FAILED [100%]",
        "",
        "================================== FAILURES ===================================",
        "______________________________ test_secret_guard ______________________________",
        "",
        "    def test_secret_guard():",
        "        res = store_output('token=abcd1234efgh')",
        ">       assert res is None",
        "E       AssertionError: assert 'h-deadbeef' is None",
        "",
        "tests/test_pipeline.py:114: AssertionError",
        "=========================== short test summary info ===========================",
        "FAILED tests/test_pipeline.py::test_secret_guard - AssertionError: assert 'h-deadbeef' is None",
        "======================== 1 failed, 37 passed in 1.18s =========================",
    ])
    py_raw = "\n".join(py_lines) + "\n"

    # 4. Git status verbose fixture
    gs_raw = "\n".join([
        "On branch feat/243-lean-d-footnote-run",
        "Your branch is up to date with 'origin/feat/243-lean-d-footnote-run'.",
        "",
        "Changes to be committed:",
        '  (use "git restore --staged <file>..." to unstage)',
        "\tnew file:   src/run.rs",
        "\tmodified:   src/app.rs",
        "\tmodified:   src/main.rs",
        "",
        "Changes not staged for commit:",
        '  (use "git add <file>..." to update what will be committed)',
        '  (use "git restore <file>..." to discard changes in working directory)',
        "\tmodified:   src/usage.rs",
        "\tmodified:   src/hook.rs",
        "\tmodified:   apps/launcher/src/Screens.jsx",
        "",
        "Unmerged paths:",
        '  (use "git restore --staged <file>..." to unstage)',
        '  (use "git add <file>..." to mark resolution)',
        "\tboth modified:   Cargo.toml",
        "",
        "Untracked files:",
        '  (use "git add <file>..." to include in what will be committed)',
        "\ttests/run.rs",
        "\tbench/experiments/lean_d_bench.py",
        "",
    ]) + "\n"

    # 5. Linter fixture (ruff + tsc multi-file)
    ruff_lines = []
    for fidx in range(4):
        path = f"packages/analytics/services/worker_{fidx}.py"
        for lidx in range(1, 9):
            ruff_lines.append(f"{path}:{lidx * 10}:5: F401 `os.path` imported but unused")
            ruff_lines.append("help: Remove unused import: `os.path`")
            ruff_lines.append("[*] 1 fixable with the `--fix` option.")
    ruff_lines.append("Found 32 errors.")
    ruff_raw = "\n".join(ruff_lines) + "\n"

    tsc_lines = []
    for fidx in range(4):
        path = f"apps/launcher/src/components/Panel{fidx}.tsx"
        for lidx in range(1, 7):
            tsc_lines.append(
                f"{path}({lidx * 12},9): error TS2322: Type 'string | null' is not assignable to type 'string'."
            )
    tsc_lines.append("Found 24 errors in 4 files.")
    tsc_raw = "\n".join(tsc_lines) + "\n"

    return [
        {
            "id": "cargo-test-fail-fixture",
            "family": "cargo-test",
            "kind": "stdin",
            "raw": ct_raw,
            "markers": [
                "engine::strict_cutoff_equivalence ... FAILED",
                "crates/search/src/semantic.rs:512:9",
                "left: [101, 204, 309]",
                "right: [101, 309, 204]",
                "44 passed; 1 failed",
            ],
        },
        {
            "id": "cargo-build-diag-fixture",
            "family": "cargo-build",
            "kind": "stdin",
            "raw": cb_raw,
            "markers": [
                "error[E0308]: mismatched types",
                "crates/core/src/state.rs:88:12",
                "expected `PathBuf`, found integer",
                "warning: unused variable: `tmp`",
                "crates/core/src/state.rs:92:9",
                "could not compile `footnote-core`",
            ],
        },
        {
            "id": "pytest-fail-fixture",
            "family": "tests",
            "kind": "stdin",
            "raw": py_raw,
            "markers": [
                "test_secret_guard FAILED",
                "AssertionError: assert 'h-deadbeef' is None",
                "tests/test_pipeline.py:114: AssertionError",
                "1 failed, 37 passed",
            ],
        },
        {
            "id": "git-status-fixture",
            "family": "git",
            "subcommand": "status",
            "kind": "stdin",
            "raw": gs_raw,
            "markers": [
                "On branch feat/243-lean-d-footnote-run",
                "src/run.rs",
                "src/app.rs",
                "src/usage.rs",
                "Cargo.toml",
                "tests/run.rs",
            ],
        },
        {
            "id": "ruff-lint-fixture",
            "family": "linter",
            "kind": "stdin",
            "raw": ruff_raw,
            "markers": [
                "packages/analytics/services/worker_0.py",
                "packages/analytics/services/worker_3.py",
                "10:5: F401",
                "80:5: F401",
                "Found 32 errors.",
            ],
        },
        {
            "id": "tsc-lint-fixture",
            "family": "linter",
            "kind": "stdin",
            "raw": tsc_raw,
            "markers": [
                "apps/launcher/src/components/Panel0.tsx",
                "apps/launcher/src/components/Panel3.tsx",
                "12:9 error TS2322",
                "72:9 error TS2322",
                "Found 24 errors in 4 files.",
            ],
        },
    ]


def real_commands() -> list[dict]:
    """Live commands executed identically under raw, `footnote run`, and `rtk`."""
    return [
        {
            "id": "live-cargo-test-core",
            "family": "cargo-test",
            "cmd": ["cargo", "test", "--locked", "-p", "footnote-core"],
            "rtk_cmd": ["rtk", "cargo", "test", "--locked", "-p", "footnote-core"],
            "markers": ["passed", "0 failed"],
        },
        {
            "id": "live-cargo-test-code",
            "family": "cargo-test",
            "cmd": ["cargo", "test", "--locked", "-p", "footnote-code"],
            "rtk_cmd": ["rtk", "cargo", "test", "--locked", "-p", "footnote-code"],
            "markers": ["passed", "0 failed"],
        },
        {
            "id": "live-node-test-launcher",
            "family": "tests",
            "cmd": ["node", "--test", "apps/launcher/src/layout.test.js"],
            "rtk_cmd": None,
            "markers": ["tests 4", "pass 4", "fail 0"],
        },
        {
            "id": "live-git-log-20",
            "family": "git",
            "cmd": ["git", "log", "-n", "20"],
            "rtk_cmd": ["rtk", "git", "log", "-n", "20"],
            "markers": [],  # dynamically populated from commit short SHAs in raw output
        },
        {
            "id": "live-git-diff-head3",
            "family": "git",
            "cmd": ["git", "diff", "HEAD~3..HEAD"],
            "rtk_cmd": ["rtk", "git", "diff", "HEAD~3..HEAD"],
            "markers": [],  # dynamically populated from @@ headers & diff --git filenames
        },
        {
            "id": "live-rg-pub-fn",
            "family": "search",
            "cmd": ["rg", "-n", "pub fn ", "crates/search/src"],
            "rtk_cmd": ["rtk", "rg", "-n", "pub fn ", "crates/search/src"],
            "markers": [],  # dynamically populated from file:line symbols in raw output
        },
        {
            "id": "live-rg-fn-core",
            "family": "search",
            "cmd": ["rg", "-n", "fn ", "crates/core/src"],
            "rtk_cmd": ["rtk", "rg", "-n", "fn ", "crates/core/src"],
            "markers": [],  # dynamically populated from line numbers + fn names
        },
    ]


def dynamic_markers(case_id: str, raw: str) -> list[str]:
    lines = raw.splitlines()
    if case_id == "live-git-log-20":
        shas = [l.split()[1][:7] for l in lines if l.startswith("commit ")][:10]
        return shas
    if case_id == "live-git-diff-head3":
        headers = [l for l in lines if l.startswith("@@ ") or l.startswith("+++ b/")][:12]
        return headers
    if case_id.startswith("live-rg-"):
        # Require every matched function identifier + line number to survive
        marks = []
        for l in lines[:25]:
            m = re.match(r"^([^:]+):(\d+):(.*)$", l)
            if m:
                ln, body = m.group(2), m.group(3).strip()
                ident = re.search(r"\bfn\s+([A-Za-z0-9_]+)", body)
                if ident:
                    marks.append(f"{ln}:")
                    marks.append(ident.group(1))
        return list(dict.fromkeys(marks))[:20]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(description="Benchmark footnote run vs RTK 1.2.0")
    parser.add_argument(
        "--footnote",
        default=str(ROOT / "target/debug/footnote.exe" if os.name == "nt" else ROOT / "target/debug/footnote"),
        help="Path to footnote binary",
    )
    parser.add_argument("--json", action="store_true", help="Print JSON instead of Markdown")
    args = parser.parse_args()

    fn_bin = Path(args.footnote)
    if not fn_bin.is_file():
        raise SystemExit(f"footnote binary not found at {fn_bin}")
    has_rtk = shutil.which("rtk") is not None

    rows = []
    with tempfile.TemporaryDirectory() as tmp:
        env = os.environ.copy()
        env["FOOTNOTE_STATE_DIR"] = tmp

        # 1. Fixture cases (deterministic stdin)
        for fx in make_fixtures():
            raw = fx["raw"]
            cmd = [str(fn_bin), "run", "--stdin", "--family", fx["family"]]
            if fx.get("subcommand"):
                # Pass subcommand token after -- only when not stdin? No, handle git subcommand via fake argv isn't used in stdin;
                # run a tiny python echo command so real command classification + subcommand detection runs end-to-end!
                pass
            if fx["id"] == "git-status-fixture":
                # Execute via real git-like stdin isn't subcommand-aware; pass `--family git` with `git status` detection tested via unit/cli
                # Actually pass python printer via stdin only for non-subcommand families; for git status test via `footnote run --family git` with raw status
                # Wait: filter_git checks subcommand! Let's invoke `footnote run` on a real git status inside a temp repo!
                continue
            fn_out, fn_rc, fn_ms = run_proc(cmd, ROOT, env, stdin_text=raw)
            hit, total = marker_recall(fn_out, fx["markers"])
            rows.append({
                "id": fx["id"],
                "family": fx["family"],
                "raw_bytes": len(raw),
                "fn_bytes": len(fn_out),
                "fn_save_pct": (1.0 - len(fn_out) / len(raw)) * 100.0 if raw else 0.0,
                "fn_recall": (hit / total * 100.0) if total else 100.0,
                "fn_markers": f"{hit}/{total}",
                "fn_ms": fn_ms,
                "fn_bloated": len(fn_out) > len(raw),
                "rtk_bytes": None,
                "rtk_save_pct": None,
                "rtk_recall": None,
                "rtk_markers": "-",
                "rtk_ms": None,
            })

        # Git status live inside an isolated temporary git repository (exact head-to-head vs RTK!)
        with tempfile.TemporaryDirectory() as git_tmp:
            gt = Path(git_tmp)
            subprocess.run(["git", "init", "-q"], cwd=gt, check=True)
            subprocess.run(["git", "config", "user.email", "bench@example.com"], cwd=gt, check=True)
            subprocess.run(["git", "config", "user.name", "Bench"], cwd=gt, check=True)
            for idx in range(6):
                (gt / f"file_{idx}.rs").write_text(f"pub fn f{idx}() -> i32 {{ {idx} }}\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=gt, check=True)
            subprocess.run(["git", "commit", "-q", "-m", "init"], cwd=gt, check=True)
            (gt / "file_0.rs").write_text("pub fn f0() -> i32 { 99 }\n", encoding="utf-8")
            (gt / "file_1.rs").write_text("pub fn f1() -> i32 { 88 }\n", encoding="utf-8")
            (gt / "staged_new.rs").write_text("pub fn s() {}\n", encoding="utf-8")
            subprocess.run(["git", "add", "staged_new.rs"], cwd=gt, check=True)
            (gt / "untracked_a.rs").write_text("pub fn u() {}\n", encoding="utf-8")
            (gt / "untracked_b.rs").write_text("pub fn v() {}\n", encoding="utf-8")

            raw, _, _ = run_proc(["git", "status"], gt, env)
            markers = ["file_0.rs", "file_1.rs", "staged_new.rs", "untracked_a.rs", "untracked_b.rs"]
            fn_out, _, fn_ms = run_proc([str(fn_bin), "run", "--", "git", "status"], gt, env)
            fn_hit, fn_tot = marker_recall(fn_out, markers)
            rtk_bytes = rtk_save = rtk_rec = rtk_ms = None
            rtk_m = "-"
            if has_rtk:
                rtk_out, _, rtk_ms = run_proc(["rtk", "git", "status"], gt, env)
                rh, rt = marker_recall(rtk_out, markers)
                rtk_bytes = len(rtk_out)
                rtk_save = (1.0 - rtk_bytes / len(raw)) * 100.0 if raw else 0.0
                rtk_rec = rh / rt * 100.0 if rt else 100.0
                rtk_m = f"{rh}/{rt}"
            rows.append({
                "id": "live-git-status",
                "family": "git",
                "raw_bytes": len(raw),
                "fn_bytes": len(fn_out),
                "fn_save_pct": (1.0 - len(fn_out) / len(raw)) * 100.0 if raw else 0.0,
                "fn_recall": fn_hit / fn_tot * 100.0,
                "fn_markers": f"{fn_hit}/{fn_tot}",
                "fn_ms": fn_ms,
                "fn_bloated": len(fn_out) > len(raw),
                "rtk_bytes": rtk_bytes,
                "rtk_save_pct": rtk_save,
                "rtk_recall": rtk_rec,
                "rtk_markers": rtk_m,
                "rtk_ms": rtk_ms,
            })

        # 2. Live commands in repo
        for case in real_commands():
            raw, raw_rc, _ = run_proc(case["cmd"], ROOT, env)
            markers = case["markers"] or dynamic_markers(case["id"], raw)
            fn_out, fn_rc, fn_ms = run_proc([str(fn_bin), "run", "--", *case["cmd"]], ROOT, env)
            assert fn_rc == raw_rc, f"{case['id']} exit code mismatch: {fn_rc} vs {raw_rc}"
            fn_hit, fn_tot = marker_recall(fn_out, markers)
            rtk_bytes = rtk_save = rtk_rec = rtk_ms = None
            rtk_m = "-"
            if has_rtk and case["rtk_cmd"]:
                rtk_out, _, rtk_ms = run_proc(case["rtk_cmd"], ROOT, env)
                rh, rt = marker_recall(rtk_out, markers)
                rtk_bytes = len(rtk_out)
                rtk_save = (1.0 - rtk_bytes / len(raw)) * 100.0 if raw else 0.0
                rtk_rec = rh / rt * 100.0 if rt else 100.0
                rtk_m = f"{rh}/{rt}"
            rows.append({
                "id": case["id"],
                "family": case["family"],
                "raw_bytes": len(raw),
                "fn_bytes": len(fn_out),
                "fn_save_pct": (1.0 - len(fn_out) / len(raw)) * 100.0 if raw else 0.0,
                "fn_recall": (fn_hit / fn_tot * 100.0) if fn_tot else 100.0,
                "fn_markers": f"{fn_hit}/{fn_tot}",
                "fn_ms": fn_ms,
                "fn_bloated": len(fn_out) > len(raw),
                "rtk_bytes": rtk_bytes,
                "rtk_save_pct": rtk_save,
                "rtk_recall": rtk_rec,
                "rtk_markers": rtk_m,
                "rtk_ms": rtk_ms,
            })

    if args.json:
        print(json.dumps(rows, indent=2))
        return 0

    print("| Case | Family | Raw B | Footnote B | FN Save | FN Recall | RTK B | RTK Save | RTK Recall |")
    print("|---|---|---:|---:|---:|---:|---:|---:|---:|")
    for r in rows:
        rb = "-" if r["rtk_bytes"] is None else str(r["rtk_bytes"])
        rs = "-" if r["rtk_save_pct"] is None else f"{r['rtk_save_pct']:.1f}%"
        rr = "-" if r["rtk_recall"] is None else f"{r['rtk_recall']:.1f}% ({r['rtk_markers']})"
        print(
            f"| {r['id']} | {r['family']} | {r['raw_bytes']} | {r['fn_bytes']} | "
            f"{r['fn_save_pct']:.1f}% | {r['fn_recall']:.1f}% ({r['fn_markers']}) | {rb} | {rs} | {rr} |"
        )

    # Family roll-up
    print("\n## Per-family gate summary (>= 99.0% recall, >= 20.0% byte saving, 0 bloat)\n")
    print("| Family | Cases | Raw B | Footnote B | Saving | Marker Recall | Bloated | Gate |")
    print("|---|---:|---:|---:|---:|---:|---:|---|")
    fams = sorted({r["family"] for r in rows})
    for fam in fams:
        sub = [r for r in rows if r["family"] == fam]
        tot_raw = sum(r["raw_bytes"] for r in sub)
        tot_fn = sum(r["fn_bytes"] for r in sub)
        save = (1.0 - tot_fn / tot_raw) * 100.0 if tot_raw else 0.0
        hits = sum(int(r["fn_markers"].split("/")[0]) for r in sub)
        m_tot = sum(int(r["fn_markers"].split("/")[1]) for r in sub)
        rec = (hits / m_tot * 100.0) if m_tot else 100.0
        bloated = sum(1 for r in sub if r["fn_bloated"])
        gate = "PASS" if rec >= 99.0 and save >= 20.0 and bloated == 0 else "FAIL"
        print(f"| {fam} | {len(sub)} | {tot_raw} | {tot_fn} | {save:.1f}% | {rec:.1f}% ({hits}/{m_tot}) | {bloated} | **{gate}** |")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
