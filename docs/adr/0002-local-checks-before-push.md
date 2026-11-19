# ADR 0002: checks run locally before a push while Actions is blocked

Accepted 19 November 2026 (Johan: "we go local for now"). Issue #205.

## Context

On 19 November 2026 every job of `ci` and `Release` fails after 3 seconds with "recent account payments have failed or your
spending limit needs to be increased". The repository is private, so GitHub also refuses branch protection and required
checks on this plan. PRs #202 to #204 and #290 merged with no job started.

## Decision

Until Actions runs again, `scripts/check.sh` (fmt, clippy on three feature sets, tests, release build, Python checks, diff
check) is the gate. `.githooks/pre-push` runs it before every push; enable it once per clone with
`git config core.hooksPath .githooks`. `FOOTNOTE_SKIP_CHECK=1` skips it for a docs-only push.

## Consequences

- A push takes as long as the check (a few minutes). Windows is covered; Linux is not, so Linux-only breaks (such as #285's
  Unix socket) still surface only when someone builds there.
- Nothing forces a clone to enable the hook, so this is discipline, not enforcement.
- Johan's other option stays open: going public (Actions is free on public repositories), or paying so Actions runs. Then
  this ADR is superseded and the hook becomes optional.

## Before the repository could go public

Not decided; Johan's condition (19 November 2026) is that the local design and iteration work must not go out, benchmarks
eventually yes. Flipping visibility would publish the whole git history (about 107 MB), including everything removed since, so
the route is a new public repository built from a reviewed snapshot, not a visibility switch. Open questions for that review:

- Tracked today: `bench/` 723 files (receipts, sweeps, experiments), `docs/research/` 137, `flow/` 78, `evidence/` 6, plus the
  experiment folders under `tools/`. Benchmarks are wanted, so each needs a look for private paths, prompts and customer names.
- Design and marketing material already lives in `D:\Projets\Footnote-Marketing`, not here (one tracked file left in `design/`).
- Secrets and personal data scan of the history, and the third-party licences already recorded in `THIRD-PARTY-NOTICES.md`.
