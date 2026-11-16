"""Commit message rule (Conventional Commits): `type(scope)!: subject`, the first line at most 72 characters.

    python scripts/check-commits.py <base>          every non-merge commit in <base>..HEAD
    python scripts/check-commits.py --message FILE  one message (a commit-msg hook can call this)

Types: feat fix perf refactor docs test build ci chore style revert (git-cliff reads them for CHANGELOG.md and the next
version). Commits authored before the rule (16 November 2026, 07:00 CET) are not checked: published history is not rewritten.
"""
import re
import subprocess
import sys

TYPES = "feat|fix|perf|refactor|docs|test|build|ci|chore|style|revert"
RULE = re.compile(rf"^({TYPES})(\([a-z0-9._/-]+\))?!?: \S")
SINCE = 1794808800  # 2026-11-16T07:00:00+01:00


def problem(subject: str) -> str | None:
    if not RULE.match(subject):
        return f"not `type(scope): subject` with type in {TYPES.replace('|', ', ')}"
    if len(subject) > 72:
        return f"{len(subject)} characters (at most 72; details go in the body)"
    return None


def main() -> int:
    if sys.argv[1:2] == ["--message"]:
        subject = open(sys.argv[2], encoding="utf-8").read().splitlines()[0]
        bad = problem(subject)
        if bad:
            print(f"commit message: {bad}\n  {subject}")
        return 1 if bad else 0
    base = sys.argv[1]
    log = subprocess.run(["git", "log", "--no-merges", "--format=%h%x09%at%x09%s", f"{base}..HEAD"],
                         capture_output=True, text=True, check=True).stdout
    failures = 0
    for line in log.splitlines():
        sha, when, subject = line.split("\t", 2)
        if int(when) < SINCE:
            continue
        bad = problem(subject)
        if bad:
            failures += 1
            print(f"{sha}: {bad}\n  {subject}")
    print(f"{failures} commit message(s) to fix" if failures else "commit messages follow the rule")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
