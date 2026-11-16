"""Candidate G check (DOCS-OPTIMIZE-2026-11-16.md): crawl the seven development libraries with the binary before G and with
G, back to back, and compare each page's extracted length.

    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_extract_check.py <before.exe> <G.exe>

Output in bench/work/dev/extract-check/{before,G}/<lib>; the list of changed pages in changes.json.
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ARGS, sys.argv = sys.argv, sys.argv[:1]
sys.path.insert(0, str(Path(__file__).parent))
import docs_crawl_speed as s  # noqa: E402

OUT = s.g.REPO / "bench/work/dev/extract-check"
FOOTER = "Python Software Foundation"


def pages(folder: Path) -> dict[str, str]:
    urls = json.loads((folder / "urls.json").read_text(encoding="utf-8"))
    return {u: (folder / f).read_text(encoding="utf-8") for f, u in urls.items()}


def main() -> None:
    s.OUT = OUT
    for lib, (url, n, exclude) in s.LIBS.items():
        for side, exe in (("before", ARGS[1]), ("G", ARGS[2])):
            if not (OUT / side / lib / "urls.json").exists():
                r = s.crawl(exe, side, lib, url, n, exclude)
                print(time.strftime("%H:%M:%S"), side, lib, json.dumps(r), flush=True)
    report = {}
    for lib in s.LIBS:
        a, b = pages(OUT / "before" / lib), pages(OUT / "G" / lib)
        common = sorted(a.keys() & b.keys())
        changed = [{"url": u, "before": len(a[u]), "G": len(b[u])} for u in common
                   if abs(len(b[u]) - len(a[u])) > 0.05 * max(len(a[u]), 1)]
        footer_only = lambda p: [u for u in common if len(p[u]) < 900 and FOOTER in p[u]]
        report[lib] = {"pages": len(common), "changed": len(changed), "share": len(changed) / max(len(common), 1),
                       "footer_only_before": len(footer_only(a)), "footer_only_G": len(footer_only(b)), "list": changed}
        print(f"{lib:12} pages {len(common):4} changed>5% {len(changed):3} ({report[lib]['share']:.1%}) "
              f"footer-only {report[lib]['footer_only_before']} -> {report[lib]['footer_only_G']}", flush=True)
    (OUT / "changes.json").write_text(json.dumps(report, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
