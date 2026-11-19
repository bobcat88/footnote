# ADR 0002: checks run locally before a push while Actions is blocked

Proposed 19 November 2026, for Johan's decision (issue #205).

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
- Johan's other option stays open: pay or cap spending so Actions runs again, Linux only on PRs and Windows on release. Then
  this ADR is superseded and the hook becomes optional.
