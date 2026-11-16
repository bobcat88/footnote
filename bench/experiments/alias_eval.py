"""Score a Footnote Alias detector on bench/datasets/alias-v1 with the measures fixed in ALIAS-EVAL.md.

    bench/.labenv/Scripts/python.exe -I bench/experiments/alias_eval.py <footnote.exe built with --features alias> dev|heldout [rules|pf|rules+pf|alias|footnote]

Detectors: the Rust rules (`footnote alias-scan`), OpenAI Privacy Filter (alias_pf.py), both (union of their spans), or alias (both, without the model's account guesses), or footnote
(the same, all inside the Rust binary: `alias-scan --model`). Found = every letter and digit of a gold value inside predicted spans,
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
DETECTOR = sys.argv[3] if len(sys.argv) > 3 else "rules"
DATA = Path(__file__).resolve().parents[1] / f"datasets/alias-v1/{HALF}.jsonl"
DIRECT = {"EMAIL", "PHONE", "IBAN", "CARD", "NIR", "SIREN", "SIRET", "IP"}


def overlaps(a: dict, b: dict) -> bool:
    return a["start"] < b["end"] and b["start"] < a["end"]


def main() -> None:
    rows = [json.loads(line) for line in DATA.read_text(encoding="utf-8").splitlines()]
    texts = [r["text"] for r in rows]
    preds: list[list[dict]] = [[] for _ in rows]
    timings = {}
    if DETECTOR == "footnote":
        pf_dir = Path(__file__).resolve().parents[2] / "artifacts/alias-models/openai-privacy-filter"
        subprocess.run([EXE, "alias-scan", "--model", str(pf_dir)], input=json.dumps(texts[:1]), capture_output=True,
                       text=True, encoding="utf-8", check=True)  # warm-up: file cache
        t = time.perf_counter()
        out = subprocess.run([EXE, "alias-scan", "--model", str(pf_dir)], input=json.dumps(texts), capture_output=True,
                             text=True, encoding="utf-8", check=True).stdout
        timings["footnote"] = (time.perf_counter() - t) * 1000 / len(rows)
        preds = json.loads(out)["results"]
    if "rules" in DETECTOR or DETECTOR == "alias":
        t = time.perf_counter()
        out = subprocess.run([EXE, "alias-scan"], input=json.dumps(texts), capture_output=True, text=True, encoding="utf-8",
                             check=True).stdout
        timings["rules"] = (time.perf_counter() - t) * 1000 / len(rows)
        preds = [p + q for p, q in zip(preds, json.loads(out)["results"])]
    if "pf" in DETECTOR or DETECTOR == "alias":
        sys.path.insert(0, str(Path(__file__).parent))
        from alias_pf import PrivacyFilter
        pf = PrivacyFilter()
        pf.detect(texts[0])  # warm-up
        t = time.perf_counter()
        # "alias": the rules own account-like identifiers (check digits), so the model's account guesses are dropped.
        keep = (lambda h: h["label"] != "ACCOUNT") if DETECTOR == "alias" else (lambda h: True)
        preds = [p + [h for h in pf.detect(x) if keep(h)] for p, x in zip(preds, texts)]
        timings["pf"] = (time.perf_counter() - t) * 1000 / len(rows)
    found, total = defaultdict(int), defaultdict(int)
    by_lang = defaultdict(lambda: [0, 0])
    nl = defaultdict(list)
    useful = n_pred = trap_hit = n_trap = 0
    misses, name_misses = [], []
    for r, ps in zip(rows, preds):
        covered = {i for p in ps for i in range(p["start"], p["end"])}
        for s in r["spans"]:
            need = {i for i in range(s["start"], s["end"]) if r["text"][i].isalnum()}
            ok = need <= covered
            total[s["label"]] += 1
            found[s["label"]] += ok
            if s["label"] == "PERSON":
                nl[r["lang"]].append(ok)
            if s["label"] in DIRECT:
                by_lang[r["lang"]][0] += ok
                by_lang[r["lang"]][1] += 1
            if not ok and s["label"] == "PERSON":
                name_misses.append((r["id"], r["text"][s["start"]:s["end"]]))
            if s["label"] in DIRECT:
                if not ok:
                    misses.append((r["id"], s["label"], r["text"][s["start"]:s["end"]]))
        n_pred += len(ps)
        useful += sum(any(overlaps(p, s) for s in r["spans"]) for p in ps)
        n_trap += len(r["traps"])
        trap_hit += sum(any(overlaps(p, t) for p in ps) for t in r["traps"])
    names_by_lang = {lang: round(sum(nl[lang]) / max(1, len(nl[lang])), 3) for lang in nl}
    group = lambda labels: sum(found[x] for x in labels) / max(1, sum(total[x] for x in labels))  # noqa: E731
    result = {
        "half": HALF, "detector": DETECTOR, "texts": len(rows),
        "recall": {k: round(found[k] / total[k], 3) for k in sorted(total)},
        "direct_identifiers_recall": round(group(DIRECT), 3),
        "direct_by_language": {k: round(v[0] / v[1], 3) for k, v in by_lang.items()},
        "names_recall": round(group({"PERSON"}), 3),
        "precision": round(useful / max(1, n_pred), 3),
        "look_alikes_hit": f"{trap_hit}/{n_trap}",
        "ms_per_text": {k: round(v, 2) for k, v in timings.items()},
        "by_language_names": names_by_lang,
        "direct_misses": misses[:10],
        "name_misses": name_misses[:12],
    }
    print(json.dumps(result, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
