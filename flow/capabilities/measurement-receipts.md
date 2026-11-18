---
type: reference
tags:
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Measurement Receipts
date_maj: 2026-11-17
---
# Measurement receipts

Up: [[AI-Agents/projects/footnote-framework/CAPABILITIES|Footnote Capability Toolkit]]

#domain/dev/testing #type/reference #project/borg #status/active

One JSON receipt per metric and revision:

```json
{
  "schema_version": 1,
  "workload_id": "input-and-operation-fingerprint",
  "environment_id": "machine-runtime-cache-policy-fingerprint",
  "metric": "elapsed",
  "unit": "ms",
  "lower_is_better": true,
  "revision": "tested-build-revision",
  "correctness_passed": true,
  "samples": [102, 99, 101]
}
```

Workload, environment, metric, unit and direction must match across the two receipts. Revisions may differ. Record correctness before timing; the helper trusts this assertion and the supplied fingerprints rather than independently verifying them. Keep enough raw repetitions for the decision's stakes and investigate variability. These example samples do not establish p95.

The helper is deterministic, Python 3.10+, standard-library only. It reads the named files (maximum 1 MiB each), writes JSON to stdout, and performs no subprocess or network call. No governor, credentials or model is needed.

Without a threshold, exit 0 means a valid measurement comparison. With `--max-regression-pct N`, it checks the caller's project budget against the change in sample medians. Exit 1 means an exceeded budget or a zero baseline that makes a percentage inconclusive. Invalid/mismatched input exits 2. Unknown or failed correctness, non-finite samples and numeric overflow are refused.

It does not calculate statistical significance, collect samples, prove causation, certify correctness, or claim an efficiency gain for a skill. Preserve original receipts with comparison results; do not tune a budget after observing the candidate.
