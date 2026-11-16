"""Score a Footnote Alias detector on bench/datasets/alias-v1 with the measures fixed in ALIAS-EVAL.md.

    bench/.labenv/Scripts/python.exe -I bench/experiments/alias_eval.py <footnote.exe built with --features alias> dev|heldout

Detector today: the Rust rules (`footnote alias-scan`). Found = every letter and digit of a gold value inside predicted spans,
any label. Precision = predicted spans overlapping a gold value / predicted spans. Look-alikes hit = look-alikes overlapped.
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from collections import defaultdict
from pathlib import Path

EXE, HALF = sys.argv[1], sys.argv[2]
DATA = Path(__file__).resolve().parents[1] / f"datasets/alias-v1/{HALF}.jsonl"
DIRECT = {"EMAIL", "PHONE", "IBAN", "CARD", "NIR", "SIREN", "SIRET", "IP"}


def overlaps(a: dict, b: dict) -> bool:
    return a["start"] < b["end"] and b["start"] < a["end"]


def main() -> None:
    rows = [json.loads(line) for line in DATA.read_text(encoding="utf-8").splitlines()]
    t = time.perf_counter()
    out = subprocess.run([EXE, "alias-scan"], input=json.dumps([r["text"] for r in rows]), capture_output=True,
                         text=True, encoding="utf-8", check=True).stdout
    ms_per_text = (time.perf_counter() - t) * 1000 / len(rows)
    preds = json.loads(out)["results"]
    found, total = defaultdict(int), defaultdict(int)
    by_lang = defaultdict(lambda: [0, 0])
    useful = n_pred = trap_hit = n_trap = 0
    misses = []
    for r, ps in zip(rows, preds):
        covered = {i for p in ps for i in range(p["start"], p["end"])}
        for s in r["spans"]:
            need = {i for i in range(s["start"], s["end"]) if r["text"][i].isalnum()}
            ok = need <= covered
            total[s["label"]] += 1
            found[s["label"]] += ok
            if s["label"] in DIRECT:
                by_lang[r["lang"]][0] += ok
                by_lang[r["lang"]][1] += 1
                if not ok:
                    misses.append((r["id"], s["label"], r["text"][s["start"]:s["end"]]))
        n_pred += len(ps)
        useful += sum(any(overlaps(p, s) for s in r["spans"]) for p in ps)
        n_trap += len(r["traps"])
        trap_hit += sum(any(overlaps(p, t) for p in ps) for t in r["traps"])
    group = lambda labels: sum(found[x] for x in labels) / max(1, sum(total[x] for x in labels))  # noqa: E731
    result = {
        "half": HALF, "texts": len(rows),
        "recall": {k: round(found[k] / total[k], 3) for k in sorted(total)},
        "direct_identifiers_recall": round(group(DIRECT), 3),
        "direct_by_language": {k: round(v[0] / v[1], 3) for k, v in by_lang.items()},
        "names_recall": round(group({"PERSON"}), 3),
        "precision": round(useful / max(1, n_pred), 3),
        "look_alikes_hit": f"{trap_hit}/{n_trap}",
        "ms_per_text_whole_process": round(ms_per_text, 2),
        "direct_misses": misses[:20],
    }
    print(json.dumps(result, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
