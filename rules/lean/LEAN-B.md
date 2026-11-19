# Footnote Lean B: write less

Rules for an agent that writes code and replies. Status: **experimental, not on by default**. Nothing here is a measured
saving until `bench/experiments/LEAN-B-BENCH.md` has run; see #239. Output is about 15% of a session's bill
(docs/research/TOKEN-ECONOMY-PLAN.md section 6), so this is a bounded lever.

## 1. Code ladder
Take the first rung that fully solves the task:
1. Skip it if the need does not justify it. Name what you skipped in one line.
2. Reuse what the codebase already has, the way the surrounding code does.
3. Use the language or platform standard library.
4. Use an already installed dependency. Add none for a few lines.
5. One line a reader gets at a glance.
6. Otherwise the minimum code that works.

A known limit gets a one-line comment: `shortcut: <the limit>, <when to upgrade>`.
Non-trivial new logic (a branch, loop, parser, money or security) ships with one small test.

## 2. Never cut
Shorter is never a reason to drop: input validation at trust boundaries, error handling that prevents data loss, security,
accessibility, the callers/tests/fixtures a change breaks, and anything the user asked for. Code you move keeps its checks.

## 3. Edits, not rewrites
- Edit the lines that change; do not reprint a file the user can open.
- No recap of tool output, no restating the question, no summary of a diff the user can read.
- End with what you skipped or did not check, and any risk.

## 4. Prose levels (agent chat replies only)
| Level | Replies |
|---|---|
| lite | Drop filler, hedging and pleasantries. Full sentences. |
| full | Short sentences, fragments allowed, no recaps. Default when on. |
| ultra | Answer first, one to three lines, abbreviations allowed. |

**Off, always, at any level:** documentation, commit messages, PR and issue bodies, release notes, comments left for other
people, and any text a user or customer reads. Write those in normal, complete prose. Safety warnings and destructive
confirmations are never compressed.

## 5. Routing
After a strong model has structured the work (a plan, a list of issues, an edit recipe), hand the mechanical part (bulk
edits, issue creation, formatting) to a cheap model. Do not route design, debugging or review. Per-task routing stays a
suggestion until the activation table from the ladder bench exists (plan section E).
