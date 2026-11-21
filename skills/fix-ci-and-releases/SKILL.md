---
name: fix-ci-and-releases
description: Use when a CI run fails or is slow, when writing or reviewing GitHub Actions workflows, or when cutting a release: failing logs, flaky tests, caching, action pinning, token permissions, script injection, pull_request_target risks, release artifacts, checksums, signing and provenance. Runs workflow-check.
license: MIT
---

# Fix CI and releases

## Run
`node <skills>/fix-ci-and-releases/scripts/workflow-check/workflow-check.mjs [.github/workflows] [--json]` → exit 1 on FAIL. Tests: `node --test scripts/workflow-check/tests`.

| Id | Rule | Level |
|---|---|---|
| W1 | action not on a full commit SHA (third-party FAIL / `actions/*`, `github/*` WARN) / `docker://` without digest | FAIL / WARN |
| W2 | no top-level `permissions:` (WARN) / `write-all` (FAIL) | WARN / FAIL |
| W3 | `${{ }}` inside `run:`: titles, bodies, branch names, commit messages FAIL / other event or input values WARN → pass via `env:`, quote `"$VAR"` | FAIL / WARN |
| W4 | `pull_request_target` / `workflow_run`: checks out contributor code FAIL, else WARN | FAIL / WARN |
| W5 | `continue-on-error: true` | WARN |

Pin as `uses: owner/action@<sha> # v1.2.3`; Dependabot or Renovate bumps SHAs.

## Failing run
1. First error in the log, not the last / record run URL + tested commit / `gh run view <id> --log-failed`
2. Reproduce locally with the same command + toolchain; a local pass doesn't prove CI (runner image, OS, caches differ)
3. Linker or loader error after a dependency bump = ABI mismatch: native lib needs newer glibc than the runner / wrong Windows CRT → fix runner or CRT (`ship-rust-cli`), not the symptom
4. Flaky: rerun once to confirm → fix or quarantine with an issue; never `continue-on-error` or retries on a required check

## Faster CI
- Measure queue / setup / restore / build / test / upload separately first
- Cache key = OS + toolchain + lockfile hash; a miss must still build; prove with a periodic clean build
- Split independent jobs when the wall-time saving beats the extra minutes / path filters only where skipped checks truly can't be affected
- Faster by skipping required verification = failure

## Release
- From a tag / `--locked` / pinned runners (`ubuntu-24.04`, not `-latest`)
- Checksums per artifact / provenance (`actions/attest-build-provenance`) or signing where supported / signing secrets scoped to the release job
- Download the published artifact and run it per target before announcing
- Version = changelog = tag / a failed release is deleted or marked, never left half-published

## Report
Run URLs + commits / cause / fix / the check that proves it / timings before + after for speed work.
