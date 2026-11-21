---
skill_hash: 6cb5021365e336c6
reviewed: 2026-11-21
---
# debug-and-test: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| `git bisect run` exit codes (0 good, 125 skip, 1–127 bad) | [git-bisect](https://git-scm.com/docs/git-bisect#_bisect_run) |
| Test doubles: mock only at boundaries, verify behaviour | [Fowler: Mocks Aren't Stubs](https://martinfowler.com/articles/mocksArentStubs.html) |
| Flaky tests: causes and quarantine | [Google Testing Blog: flaky tests](https://testing.googleblog.com/2016/05/flaky-tests-at-google-and-how-we.html) |
| Red, green, refactor | [Fowler: Test Driven Development](https://martinfowler.com/bliki/TestDrivenDevelopment.html) |

## Distilled from
High-install public debugging, test-first and verification skills, surveyed 2026-11-21 (MIT); rules rewritten in our words, credited in NOTICE. Kept: reproducing loop before theory, falsifiable ranked hypotheses, tagged debug logs, 3-failed-fixes stop, red-green proof of a regression test, evidence table for completion claims.
