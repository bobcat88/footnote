"""Lean search output check: same rankings, fewer characters.

    bench/.labenv/Scripts/python.exe -I bench/experiments/lean_output_check.py <before.exe> <lean.exe> [db label]

Runs every development question (docs_dev.py sets) against the databases of one docs_dev label with both binaries, in
hybrid (the shipped pair) and lexical mode, `--limit 5`, and reports whether the ranked chunk ids are identical and the
output size in characters (compact JSON) and in o200k_base tokens when tiktoken is installed.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ARGS, sys.argv = sys.argv, sys.argv[:1]
sys.path.insert(0, str(Path(__file__).parent))
import docs_dev as d  # noqa: E402
import yaml  # noqa: E402

g = d.g


def search(exe: str, db: Path, query: str, mode: list[str]) -> dict:
    out = subprocess.run([exe, "--db", str(db), "search", query, "--limit", "5", *mode],
                         capture_output=True, text=True, encoding="utf-8", check=True).stdout
    return json.loads(out)


def main() -> None:
    before, lean = ARGS[1], ARGS[2]
    label = ARGS[3] if len(ARGS) > 3 else "B2-structure"
    try:
        import tiktoken
        size = lambda s: len(tiktoken.get_encoding("o200k_base").encode(s))  # noqa: E731
        unit = "tokens"
    except ImportError:
        size, unit = len, "characters"
    modes = {"hybrid": ["--mode", "hybrid", *[a for m in g.MODELS for a in ("--model", m)]], "lexical": []}
    report = {}
    for mode, args in modes.items():
        same, n, sizes = 0, 0, {"before": 0, "lean": 0}
        for name, (_, dataset) in d.SETS.items():
            for e in yaml.safe_load(dataset.read_text(encoding="utf-8"))["entries"]:
                q = e["vars"]
                db = d.OUT / label / name / f"{q['library']}.db"
                a, b = search(before, db, q["query"], args), search(lean, db, q["query"], args)
                same += [r["id"] for r in a["results"]] == [r["id"] for r in b["results"]]
                n += 1
                for side, v in (("before", a), ("lean", b)):
                    sizes[side] += size(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
        report[mode] = {"questions": n, "identical_rankings": same, unit: sizes,
                        "ratio": round(sizes["lean"] / sizes["before"], 3)}
        print(mode, json.dumps(report[mode]), flush=True)
    (d.OUT / "lean-output.json").write_text(json.dumps(report, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
