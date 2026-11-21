#!/usr/bin/env python3
"""Local PMO priority queue reader and GitHub state validator.

Reads `docs/roadmap/PRIORITY-QUEUE.md`, syncs with open GitHub issues via `gh`,
and prints the next highest-value issues to implement in order.

Usage:
    python scripts/pmo_queue.py --next 10
    python scripts/pmo_queue.py --tier P2
    python scripts/pmo_queue.py --area engineering
    python scripts/pmo_queue.py --check
    python scripts/pmo_queue.py --offline --next 5
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO = "bobcat88/footnote"
DEFAULT_QUEUE = Path(__file__).resolve().parent.parent / "docs" / "roadmap" / "PRIORITY-QUEUE.md"
TIER_RE = re.compile(r"^##\s+(P[0-4])\b")
ROW_RE = re.compile(
    r"^\|\s*(\d+)\s*\|\s*#(\d+)\s*\|\s*#(\d+)\s*\|\s*([^|]+?)\s*\|\s*(\d+)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*(.+?)\s*\|$"
)


@dataclass
class QueueItem:
    rank: int
    tier: str
    issue: int
    epic: int
    area: str
    impact: int
    effort: str
    depends: list[int]
    summary: str
    state: str = "unknown"


def parse_queue(path: Path) -> list[QueueItem]:
    items: list[QueueItem] = []
    tier = ""
    for line in path.read_text(encoding="utf-8").splitlines():
        m_tier = TIER_RE.match(line.strip())
        if m_tier:
            tier = m_tier.group(1)
            continue
        m_row = ROW_RE.match(line.strip())
        if not m_row or not tier:
            continue
        deps_raw = m_row.group(7).strip()
        deps = [int(x) for x in re.findall(r"#(\d+)", deps_raw)]
        items.append(
            QueueItem(
                rank=int(m_row.group(1)),
                tier=tier,
                issue=int(m_row.group(2)),
                epic=int(m_row.group(3)),
                area=m_row.group(4).strip(),
                impact=int(m_row.group(5)),
                effort=m_row.group(6).strip(),
                depends=deps,
                summary=m_row.group(8).strip(),
            )
        )
    return items


def fetch_open_github_issues(repo: str) -> dict[int, dict]:
    out = subprocess.check_output(
        [
            "gh",
            "issue",
            "list",
            "--repo",
            repo,
            "--state",
            "open",
            "--limit",
            "200",
            "--json",
            "number,title,labels",
        ],
        text=True,
    )
    result: dict[int, dict] = {}
    for row in json.loads(out):
        labels = [lbl["name"] for lbl in row.get("labels", [])]
        if "epic" in labels:
            continue
        result[int(row["number"])] = {
            "title": row["title"],
            "labels": labels,
        }
    return result


def format_table(items: list[QueueItem]) -> str:
    if not items:
        return "No matching issues."
    header = f"{'Rank':>4}  {'Tier':4}  {'Issue':>6}  {'Area':15}  {'Imp':>3}  {'Eff':3}  {'Deps':12}  Summary"
    sep = "-" * len(header)
    lines = [header, sep]
    for it in items:
        deps = ",".join(f"#{d}" for d in it.depends) if it.depends else "-"
        lines.append(
            f"{it.rank:>4}  {it.tier:4}  #{it.issue:<5}  {it.area:15}  {it.impact:>3}  {it.effort:3}  {deps:12}  {it.summary}"
        )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Query and validate the local PMO priority queue.")
    parser.add_argument("--queue", type=Path, default=DEFAULT_QUEUE, help="Path to PRIORITY-QUEUE.md")
    parser.add_argument("--repo", default=REPO, help="GitHub owner/repo")
    parser.add_argument("--offline", action="store_true", help="Read local queue without calling gh")
    parser.add_argument("--next", type=int, default=None, metavar="N", help="Show next N open issues")
    parser.add_argument("--tier", choices=["P0", "P1", "P2", "P3", "P4"], help="Filter by tier")
    parser.add_argument("--area", help="Filter by area label")
    parser.add_argument("--unblocked", action="store_true", help="Only show issues whose dependencies are closed")
    parser.add_argument("--check", action="store_true", help="Validate queue integrity against GitHub open issues")
    parser.add_argument("--format", choices=["table", "json"], default="table", help="Output format")
    args = parser.parse_args()

    items = parse_queue(args.queue)
    if not items:
        print(f"error: no queue items parsed from {args.queue}", file=sys.stderr)
        return 1

    # Duplicate check always runs
    seen: set[int] = set()
    dups: list[int] = []
    for it in items:
        if it.issue in seen:
            dups.append(it.issue)
        seen.add(it.issue)

    open_issues: dict[int, dict] | None = None
    if not args.offline:
        open_issues = fetch_open_github_issues(args.repo)
        for it in items:
            it.state = "open" if it.issue in open_issues else "closed"

    if args.check:
        closed_in_queue = [it for it in items if open_issues is not None and it.issue not in open_issues]
        unranked_open = (
            sorted(num for num in open_issues if num not in seen) if open_issues is not None else []
        )
        print(f"Queue file: {args.queue}")
        print(f"Total ranked items: {len(items)}")
        print(f"Duplicate issue entries: {dups if dups else 'none'}")
        if open_issues is not None:
            print(f"Open non-epic GitHub issues: {len(open_issues)}")
            print(
                f"Closed issues still in queue ({len(closed_in_queue)}): "
                + (", ".join(f"#{it.issue}" for it in closed_in_queue) or "none")
            )
            print(
                f"Open issues not yet ranked ({len(unranked_open)}): "
                + (", ".join(f"#{n}" for n in unranked_open) or "none")
            )
        return 1 if dups else 0

    filtered = items
    if open_issues is not None:
        filtered = [it for it in filtered if it.state == "open"]
    if args.tier:
        filtered = [it for it in filtered if it.tier == args.tier]
    if args.area:
        filtered = [it for it in filtered if it.area.lower() == args.area.lower()]
    if args.unblocked and open_issues is not None:
        filtered = [it for it in filtered if all(d not in open_issues for d in it.depends)]
    if args.next is not None:
        filtered = filtered[: args.next]

    if args.format == "json":
        print(json.dumps([asdict(it) for it in filtered], indent=2))
    else:
        print(format_table(filtered))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
