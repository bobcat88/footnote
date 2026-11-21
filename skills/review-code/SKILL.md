---
name: review-code
description: Use when reviewing a diff, branch or pull request, asking another agent for a review, or acting on review feedback. Fixed base ref, spec and code checked as separate axes, severity that decides merge, scanner findings limited to changed lines, verify before replying. Runs sarif-diff.
license: MIT
---

# Review code

## Run
`node <skills>/review-code/scripts/sarif-diff/sarif-diff.mjs <file.sarif...> [--base origin/main] [--all] [--json]` → keeps findings on lines added since the merge base (`--all`: every finding). error FAIL / warning WARN / note INFO; exit 1 on FAIL. Input: `semgrep scan --sarif -o s.sarif` / CodeQL / any SARIF 2.1 tool.

## Scope
- Fixed base: `git rev-parse <base>` resolves / `git diff <base>...HEAD` (three dots) is not empty / no base given → ask
- Spec: issue named in commits / PR body / plan file; none → say "no spec" and review the code axis only
- Standards: CONTRIBUTING / AGENTS.md or CLAUDE.md / lint config; what tooling already enforces is not a finding
- Read each changed function whole, plus its callers; a hunk alone hides the bug

## Spec axis
- Requirement missing / partly done / looks done but wrong / scope creep
- Quote the spec line behind each finding

## Code axis
- Correctness: wrong condition / off by one / empty or null input / swallowed error / resource left open / race on shared state / time zone, locale, encoding
- Trust boundaries: input validated / injection: SQL, shell, path, HTML / authorisation on every new route or command / secrets out of code and logs
- Data: migration reversible / destructive operation guarded / stored format still readable
- Tests: new branches covered / the test fails without the change (`debug-and-test` red-green) / no `.only`, skips or sleeps
- Smells, as judgement, not violations: unclear name / duplicated logic / long parameter list / speculative generality / one change touching many files
- Simpler: code to delete / an existing helper it duplicates / an abstraction with one caller

## Report
- Two sections: Spec / Code; never merge or rerank across them, a change can pass one and fail the other
- Each finding: `file:line` / what breaks, with the input that breaks it / fix / severity
- Critical = data loss, security, broken main path → blocks merge / Important = wrong edge case, missing test → fix before merge / Minor = naming, style → note
- Last line: count per section + worst finding in each
- Unsure → say so; no padding with Minor findings

## Asking another agent
- Fresh context: base and head SHA / spec or plan / what changed and why; never the chat history
- When: before merging to main / after each task of a multi-step plan / after a hard bug fix
- Check its claims against the actual diff

## Acting on feedback
- Verify each point in the code before changing anything / reproduce claimed bugs
- Critical now / Important before moving on / Minor logged
- Wrong feedback → reply with evidence (test, trace, spec line), not agreement
- Reply per thread: what changed + commit
