"""Texts for the candle parity bench (#358): passages and queries cut from Footnote's own Markdown, deterministic.

python bench/experiments/candle-encoder/make_texts.py <out dir>
Writes passages.json (300 paragraphs plus 50 long joined runs that hit the 512-token limit) and queries.json (100 headings).
"""
import json, random, re, sys
from pathlib import Path

root = Path(__file__).resolve().parents[3]
files = sorted(p for d in ("docs", "bench/experiments") for p in (root / d).rglob("*.md"))
paras, heads = [], []
for f in files:
    blocks = re.split(r"\n\s*\n", f.read_text(encoding="utf-8", errors="replace"))
    for b in blocks:
        b = b.strip()
        if b.startswith("#"):
            h = b.lstrip("#").strip().splitlines()[0]
            if 3 <= len(h.split()) <= 12:
                heads.append(h)
        elif len(b) >= 80 and not b.startswith(("|", "```")):
            paras.append(" ".join(b.split()))
rng = random.Random(358)
paras, heads = sorted(set(paras)), sorted(set(heads))
rng.shuffle(paras)
rng.shuffle(heads)
long = [" ".join(paras[i:i + 12]) for i in range(300, 300 + 50 * 12, 12)]
out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
(out / "passages.json").write_text(json.dumps(paras[:300] + long, ensure_ascii=False), encoding="utf-8")
(out / "queries.json").write_text(json.dumps(heads[:100], ensure_ascii=False), encoding="utf-8")
print(len(paras), "paragraphs,", len(heads), "headings in", len(files), "files")
