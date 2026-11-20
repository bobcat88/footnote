# ADR 0002: local pre-push checks mirror CI

Accepted 19 November 2026. Issue #205.

## Context

Running CI locally before pushing catches formatting, multi-feature clippy warnings, and Windows regressions immediately instead of waiting on remote queues.

## Decision

`scripts/check.sh` (fmt, clippy across feature sets, workspace tests, Launcher JS checks, and site build) is the recommended local check gate. `.githooks/pre-push` runs it before every push; enable it once per clone with `git config core.hooksPath .githooks`. `FOOTNOTE_SKIP_CHECK=1` skips it for a docs-only push.

## Consequences

- A push runs the local validation suite before network upload, catching cross-feature compiler warnings early.
- GitHub Actions runs full Windows and Linux checks on PRs as the canonical verification gate.

## Public release checklist

Before cutting a fresh public snapshot, verify:
- Benchmark records in `bench/` and design notes in `docs/` contain no private machine paths, private project names, or personal data.
- Third-party licences stay recorded in `THIRD-PARTY-NOTICES.md`.
