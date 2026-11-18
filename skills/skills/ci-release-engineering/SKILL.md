---
name: ci-release-engineering
description: Improve CI duration, cache effectiveness and release reliability. Use
  for build pipelines, artifact provenance and release checks; publishing still requires
  task authorization.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 2235a994af665601dede124d37617087928ef83963ff945ed2a9597ea3f93051
  source-card: AI-Agents/skills/ci-release-engineering/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# CI release engineering



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Make the pipeline measurable

- Separate queue time, runner setup, dependency restore, build, tests and upload. Compare equivalent runner images and inputs.
- Parallelize independent jobs when wall-time savings justify total runner time. Do not hide a failing required check with continue-on-error.
- Cache reproducible dependencies using keys that capture platform and relevant lock/config inputs. A cache miss must still produce a correct build.
- Treat caches and downloadable release artifacts as different mechanisms. Test a clean build; do not treat a warm-cache-only pass as reproducibility.
- Where supported, use reviewed workflow dependency lockfiles that verify action identities and transitive dependencies; otherwise pin third-party actions to reviewed full commit SHAs. Minimize token permissions and keep untrusted pull-request code away from privileged execution. Avoid interpolating untrusted text directly into shell code. Check the repository's current GitHub support before prescribing a lockfile format.
- Verify artifact identity, version alignment, signature/provenance where required, and installation of the artifact actually produced by CI.

## Measurements and checks

Record end-to-end wall time, runner minutes, job durations, cache hit/miss, artifact size, flaky reruns and failed releases. Predeclare acceptable duration and reliability for the repository. Faster CI achieved by skipping required verification fails acceptance.

Use the repository's normal build/test commands through RTK and inspect workflow definitions. Local tests do not prove hosted CI passed. Record remote run URLs and the tested commit when live runs are authorized.

## Candidate helper for round 2

Summarize exported CI job timings and artifact metadata without credentials. A separate static check may flag unpinned actions and broad permissions for review; it cannot prove workflow security.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://docs.github.com/en/actions/reference/security/secure-use
- https://docs.github.com/en/actions/concepts/workflows-and-actions/dependency-caching

