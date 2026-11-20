#!/usr/bin/env python3
"""Summarize GitHub issue and PR ownership, completion, and hygiene metrics per AI agent and model."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict

DEFAULT_REPO = "bobcat88/footnote"

SIGNATURE_RE = re.compile(
    r"\*\*([A-Za-z0-9 ._-]+?)\s*(?:·|-)\s*([^*]+?)\*\*\s*$",
    re.MULTILINE,
)
PARENT_RE = re.compile(r"(?:Parent|Epic)\s*:\s*#\d+", re.IGNORECASE)
AGENT_PREFIX = "agent:"
FAMILIES = ("gemini", "claude", "gpt")


def fetch_items(repo: str, limit: int) -> list[dict]:
    items: list[dict] = []
    page = 1
    while len(items) < limit:
        per_page = min(100, limit - len(items))
        endpoint = (
            f"repos/{repo}/issues?state=all&sort=updated&direction=desc"
            f"&per_page={per_page}&page={page}"
        )
        proc = subprocess.run(
            ["gh", "api", endpoint],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )
        if proc.returncode != 0:
            err = proc.stderr.strip() or proc.stdout.strip() or f"gh api exit {proc.returncode}"
            raise RuntimeError(f"failed to fetch {endpoint}: {err}")
        batch = json.loads(proc.stdout)
        if not batch:
            break
        items.extend(batch)
        if len(batch) < per_page:
            break
        page += 1
    return items[:limit]


def parse_signature(body: str) -> tuple[str, str] | None:
    if not body:
        return None
    matches = SIGNATURE_RE.findall(body)
    for agent, model in reversed(matches):
        agent_clean = agent.strip()
        model_clean = model.strip()
        if "<" in model_clean and ">" in model_clean:
            continue
        return agent_clean, model_clean
    return None


def infer_family(labels: list[str], sig: tuple[str, str] | None) -> str:
    for lbl in labels:
        if lbl.startswith(AGENT_PREFIX):
            return lbl[len(AGENT_PREFIX) :]
    if sig is not None:
        combined = f"{sig[0]} {sig[1]}".lower()
        for fam in FAMILIES:
            if fam in combined:
                return fam
        return "other-ai"
    return "unattributed"


def empty_bucket() -> dict[str, int]:
    return {
        "issues_total": 0,
        "issues_open": 0,
        "issues_completed": 0,
        "issues_not_planned": 0,
        "prs_total": 0,
        "prs_open": 0,
        "prs_closed_or_merged": 0,
        "missing_parent": 0,
        "missing_signature": 0,
    }


def analyze(items: list[dict]) -> tuple[dict[str, dict[str, int]], dict[str, dict[str, int]]]:
    by_family: dict[str, dict[str, int]] = defaultdict(empty_bucket)
    by_model: dict[str, dict[str, int]] = defaultdict(empty_bucket)

    for item in items:
        labels = [lbl.get("name", "") for lbl in item.get("labels", [])]
        body = item.get("body") or ""
        sig = parse_signature(body)
        family = infer_family(labels, sig)
        has_agent_label = any(lbl.startswith(AGENT_PREFIX) for lbl in labels)

        if sig is not None:
            model_key = f"{sig[0]} · {sig[1]}"
        elif has_agent_label:
            model_key = f"{family} (missing signature)"
        else:
            model_key = "human / unattributed"

        is_pr = "pull_request" in item
        is_epic = "epic" in labels
        state = item.get("state", "open")
        reason = item.get("state_reason")

        for bucket in (by_family[family], by_model[model_key]):
            if is_pr:
                bucket["prs_total"] += 1
                if state == "open":
                    bucket["prs_open"] += 1
                else:
                    bucket["prs_closed_or_merged"] += 1
            else:
                bucket["issues_total"] += 1
                if state == "open":
                    bucket["issues_open"] += 1
                elif reason == "not_planned":
                    bucket["issues_not_planned"] += 1
                else:
                    bucket["issues_completed"] += 1
                if not is_epic and not PARENT_RE.search(body):
                    bucket["missing_parent"] += 1

            if (has_agent_label or sig is not None) and sig is None:
                bucket["missing_signature"] += 1

    return dict(sorted(by_family.items())), dict(sorted(by_model.items()))


def render_table(title: str, key_col: str, stats: dict[str, dict[str, int]]) -> list[str]:
    lines = [
        f"## {title}",
        "",
        f"| {key_col} | Issues (Open / Done / Not planned) | PRs (Open / Closed or Merged) | Missing Parent | Missing Sig |",
        "|---|---:|---:|---:|---:|",
    ]
    for name, b in stats.items():
        issues = (
            f"{b['issues_total']} "
            f"({b['issues_open']} / {b['issues_completed']} / {b['issues_not_planned']})"
        )
        prs = f"{b['prs_total']} ({b['prs_open']} / {b['prs_closed_or_merged']})"
        lines.append(
            f"| {name} | {issues} | {prs} | {b['missing_parent']} | {b['missing_signature']} |"
        )
    lines.append("")
    return lines


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Report AI agent & model ownership metrics from GitHub issues and PRs."
    )
    parser.add_argument(
        "--repo",
        default=DEFAULT_REPO,
        help=f"GitHub owner/repo (default: {DEFAULT_REPO})",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=100,
        help="Maximum recent issues+PRs to sample (default: 100)",
    )
    parser.add_argument(
        "--format",
        choices=("markdown", "json"),
        default="markdown",
        help="Output format (default: markdown)",
    )
    args = parser.parse_args()
    if args.limit <= 0:
        parser.error("--limit must be a positive integer")

    try:
        items = fetch_items(args.repo, args.limit)
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    by_family, by_model = analyze(items)
    payload = {
        "repo": args.repo,
        "limit": args.limit,
        "sampled": len(items),
        "by_family": by_family,
        "by_model": by_model,
    }

    if args.format == "json":
        print(json.dumps(payload, indent=2, ensure_ascii=False))
        return 0

    lines = [
        f"# AI Agent Ownership Metrics (`{args.repo}`, last {len(items)} updated items)",
        "",
    ]
    lines.extend(render_table("By agent family", "Family", by_family))
    lines.extend(render_table("By signature model", "Agent · Model", by_model))
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
