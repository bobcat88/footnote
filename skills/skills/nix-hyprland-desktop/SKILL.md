---
name: nix-hyprland-desktop
description: Make reproducible NixOS or Hyprland configuration changes with measured
  rebuild/session impact and a recovery path. Use on the actual Linux host; Windows
  cannot establish session acceptance.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: b39b18cac4b2ff00eb62a92fb0171af4044a82099c0ee4c9897e375ded0fbd2c
  source-card: AI-Agents/skills/nix-hyprland-desktop/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Nix Hyprland desktop



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Match the installed system

- Identify host, Nix/NixOS version, Hyprland version, configuration entry points, lock revision and current generation.
- Use documentation for that version. Hyprland configuration syntax evolves; do not paste the latest syntax into an older installation or migrate formats as a side effect.
- Preserve the current working generation and a recovery path. Build first, test activation when appropriate, and switch persistently only within authorization.
- Pin dependency inputs for reproducibility; record both cache state and input revisions. Do not delete old generations or change boot settings to make a benchmark look cleaner.
- Keep secrets outside publicly readable configuration/store paths. Inspect generated service and environment changes for unintended exposure.
- Change one measured source of startup, rendering or resource cost at a time. Preserve accessibility, keyboard navigation and usable recovery from a compositor failure.

## Measurements and checks

Separate evaluation/build time, downloads, activation and session startup. Compare cold/warm cache conditions explicitly. Measure idle CPU/memory after a declared settling interval and observe frame/interaction problems on the actual target display.

Check configuration evaluation/build, service failures, login, keyboard, monitors, suspend/resume and rollback as applicable. A build on Windows or a static file parse is not a working Linux desktop test. If the host is unavailable, report that acceptance is pending.

## Candidate helper for round 2

Collect read-only version, build-duration and service-status receipts on the authorized host. Keep privileged activation and cleanup out of the measurement helper.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://nixos.org/manual/nixos/stable/
- https://wiki.hypr.land/configuring/core/environment-variables/
- https://wiki.hypr.land/0.50.0/configuring/variables/

