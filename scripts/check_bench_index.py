#!/usr/bin/env python3
"""Fail when bench/INDEX.md drifts from the reports it indexes."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INDEX = ROOT / "bench" / "INDEX.md"
REPORT = re.compile(r"RESULTS|REPORT|STATUS")


def headline(line):
    s = line.strip()
    if s.startswith("|"):
        return " · ".join(c.strip() for c in s.strip("|").split("|"))
    return re.sub(r"^[-*] ", "", s)


def main():
    text = INDEX.read_text(encoding="utf-8")
    errors = []
    linked = set()
    ids = set()
    for row in text.splitlines():
        m = re.match(r"\| ([A-Z]+-\d+) \|", row)
        if not m:
            continue
        if m.group(1) in ids:
            errors.append(f"duplicate id {m.group(1)}")
        ids.add(m.group(1))
        cells = re.split(r"(?<!\\)\|", row)[1:-1]
        cell, status, link = (c.strip() for c in (cells[3], cells[4], cells[5]))
        lm = re.search(r"\]\(([^)#]+)#L(\d+)\)", link)
        if not lm:
            errors.append(f"{m.group(1)}: no line link")
            continue
        path = (INDEX.parent / lm.group(1)).resolve()
        if not path.is_file():
            errors.append(f"{m.group(1)}: missing file {lm.group(1)}")
            continue
        src = path.read_text(encoding="utf-8").split("\n")
        n = int(lm.group(2))
        if not 1 <= n <= len(src):
            errors.append(f"{m.group(1)}: line {n} not in {lm.group(1)} ({len(src)} lines)")
            continue
        want = cell.replace("\\|", "|").replace("]\\(", "](").removesuffix(" …").strip()
        if not headline(src[n - 1]).startswith(want):
            errors.append(f"{m.group(1)}: headline no longer matches {lm.group(1)}:{n}")
        linked.add(path)
    for m in re.finditer(r"\| SUPERSEDED by ([A-Z]+-\d+) \|", text):
        if m.group(1) not in ids:
            errors.append(f"SUPERSEDED by unknown id {m.group(1)}")
    prose = "\n".join(l for l in text.splitlines() if not re.match(r"\| [A-Z]+-\d+ \|", l))
    for m in re.finditer(r"\]\(([^)#:]+)[)#]", prose):
        if not (INDEX.parent / m.group(1)).exists():
            errors.append(f"broken link {m.group(1)}")
    for f in (ROOT / "bench" / "experiments").iterdir():
        if f.suffix == ".md" and REPORT.search(f.name) and f.resolve() not in linked:
            errors.append(f"not in bench/INDEX.md: bench/experiments/{f.name} (add one row)")
    if errors:
        print("bench/INDEX.md check failed:", *errors, sep="\n  ", file=sys.stderr)
        return 1
    print(f"bench/INDEX.md ok: {len(ids)} rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
