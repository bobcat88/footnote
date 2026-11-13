"""Write the shipped-binary numbers into README.md and site/index.html from the newest receipts. Read-only on receipts.

    python bench/experiments/fill_numbers.py

README placeholders @@SCIFACT@@ and so on are replaced once; afterwards the table cells are updated in place, so the script can be rerun
after new receipts. Every number comes from a file in bench/receipts, never typed.
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REC = ROOT / "bench" / "receipts"
ENGINE = "shipped-ensemble-mcp"
NAMES = {"scifact": "SCIFACT", "nfcorpus": "NFCORPUS", "alloprof": "ALLOPROF", "syntec": "SYNTEC", "mix-fr-en": "MIX"}


def newest(dataset: str) -> dict:
    files = sorted(REC.glob(f"*-{dataset}-{ENGINE}.json"))
    if not files:
        raise SystemExit(f"no receipt for {dataset}/{ENGINE}")
    r = json.loads(files[-1].read_text(encoding="utf-8"))
    assert not r.get("partial"), files[-1].name
    return r


vals = {d: newest(d) for d in NAMES}
ndcg = {d: f"{r['metrics']['ndcg@10']:.3f}" for d, r in vals.items()}

readme = ROOT / "README.md"
t = readme.read_text(encoding="utf-8")
readme_rows = {"SciFact, science claims (EN)": "scifact", "NFCorpus, nutrition and medicine (EN)": "nfcorpus",
               "Alloprof, school questions (FR)": "alloprof", "Syntec, business documents (FR)": "syntec",
               "French and English in one index, 11,462 documents": "mix-fr-en"}
for label, d in readme_rows.items():
    pat = re.compile(r"(\| " + re.escape(label) + r" \|[^\n]*\| \*\*)(?:[0-9.]+|@@[A-Z]+@@)(\*\* \|)")
    t, count = pat.subn(lambda m: m.group(1) + ndcg[d] + m.group(2), t, count=1)
    if count != 1:
        raise SystemExit(f"missing or ambiguous README row: {label}")
readme.write_text(t, encoding="utf-8")

page = ROOT / "site" / "index.html"
h = page.read_text(encoding="utf-8")
for dataset in ("scifact", "nfcorpus", "alloprof", "syntec"):
    pat = re.compile(r'(<div class="benchmark-line footnote-result" data-dataset="' + dataset + r'">.*?--score:)[0-9.]+(.*?<span class="benchmark-value">)[0-9.]+(</span>)')
    h, count = pat.subn(lambda m: m.group(1) + ndcg[dataset] + m.group(2) + ndcg[dataset] + m.group(3), h, count=1)
    if count != 1:
        raise SystemExit(f"missing chart row: {dataset}")
h, count = re.subn(r'(<strong data-result="mix-fr-en">)[0-9.]+(</strong>)', lambda m: m.group(1) + ndcg["mix-fr-en"] + m.group(2), h, count=1)
if count != 1:
    raise SystemExit("missing mixed-language result")
# Keep each dataset ordered when receipt values change.
def rank_group(match: re.Match) -> str:
    group = match.group(0)
    lines = re.findall(r'<div class="benchmark-line[^>]*>.*?</div>', group)
    if len(lines) != 3:
        raise SystemExit("benchmark group must contain three results")
    ranked = sorted(lines, key=lambda line: float(re.search(r'--score:([0-9.]+)', line).group(1)), reverse=True)
    it = iter(ranked)
    return re.sub(r'<div class="benchmark-line[^>]*>.*?</div>', lambda _: next(it), group)

h, groups = re.subn(r'<div class="benchmark-group">.*?</h3>(?:<div class="benchmark-line[^>]*>.*?</div>){3}</div>', rank_group, h)
if groups != 4:
    raise SystemExit(f"expected four benchmark groups, found {groups}")
page.write_text(h, encoding="utf-8")

for d, r in vals.items():
    print(f"{d:<10} nDCG@10 {ndcg[d]}  p50 {r['latency_ms']['p50']} ms  p95 {r['latency_ms']['p95']} ms  queries {r['dataset']['queries']}  receipt {r['timestamp_utc']}")
