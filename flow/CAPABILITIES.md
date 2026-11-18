---
type: reference
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Footnote Capability Toolkit
date_maj: 2026-11-17
---
# Footnote Capability Toolkit

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Footnote Framework]] · Review: [[AI-Agents/projects/footnote-framework/REVIEW|Review]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## Standalone use

The eleven competence skills and six job procedures live in `AI-Agents/skills`. Source curation is recorded in SOURCE-REVIEW.md; behavioral benefit remains experimental. They neither create agents nor require governor state. The web pack also includes the revised website-launch-checklist dependency, bringing the full export to eighteen cards. Nothing has been installed.

Thematic packaging: systems (Rust, SQLite, CI), desktop (Tauri, React, CI), intelligence (RAG plus the implementation-language capability), data (Python and conditional backtesting), automation (scraping and conditional Apps Script/Python), Linux workstation (Nix/Hyprland), and web (website launch, delivery job and conditional React). These are distribution groupings, never instructions to load every member.

UI-facing packs include a normalized QLoop protocol and declare its external prerequisites in the manifest. The website job's checklist/reference is bundled and source-reviewed. External UI tooling and specialist prerequisites are not silently installed or certified as part of the pack. A missing prerequisite leaves its acceptance checks pending.

## Build a pack without installation

From this project directory:

```powershell
$env:FLOW_SKILL_SOURCES = "<notes vault>/AI-Agents"   # the authored cards stay in the vault
rtk proxy python capabilities/build_packs.py systems build/systems-v0.2.0
rtk proxy python capabilities/build_packs.py all build/toolkit-v0.2.0
rtk proxy python -m unittest discover -s capabilities -p 'test_build_packs.py' -v
```

Choose a new output path on each run; existing outputs are never overwritten. The builder uses the separately pinned PyYAML dependency in capabilities/requirements-toolkit.txt. Exported measurement scripts need only Python's standard library. Source cards retain vault fields; exported cards contain standard skill metadata and provenance hashes. References are copied and vault-only navigation is normalized. Relative dependencies are checked before writing output.

In-project output is restricted to build, and skill installation/discovery roots are refused. There is no install, approval-signing, runtime registration or agent-launch command. The manifest hashes every exported artifact and lists external prerequisites. The full export v0.2.0 contains eighteen cards and thirty-one hashed artifacts, plus its manifest.

## First deterministic helper

`capabilities/scripts/compare_receipts.py` is independent of orchestration and uses Python's standard library only. It reads two explicitly supplied JSON receipts, verifies comparable metadata and caller-asserted correctness, and reports medians and relative regression. No network access, subprocesses, installation or output-file writes.

Example invocation from this project directory:

```powershell
rtk proxy python capabilities/scripts/compare_receipts.py before.json after.json
rtk proxy python capabilities/scripts/compare_receipts.py before.json after.json --max-regression-pct 5
rtk proxy python -m unittest discover -s capabilities/scripts -p 'test_*.py' -v
```

The 5 percent budget is an example chosen by the caller, not a recommended universal threshold. Exit 0 without a threshold means a valid measurement comparison, not performance acceptance. With a threshold, exit 1 indicates an over-budget or inconclusive zero-baseline comparison. Invalid input exits 2.

Receipt shape:

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

Use separate receipts per metric. Workload, environment, metric, unit and direction must match. Build revisions may differ. Samples must be finite, nonnegative numeric values. Missing/failed correctness, invalid schemas, mismatched metadata and oversized files are rejected. Extreme numeric overflow is rejected rather than emitted as an invalid JSON number.

The helper does not independently verify supplied fingerprints or correctness, collect process memory, compute p95, establish statistical significance, or measure skill benefit. It is a receipt comparator, not a benchmark runner or governor.

## Current evidence

- All eighteen v0.2.0 cards passed the bundled format validator and all thirty-one artifact hashes matched. The shipped comparator ran successfully with isolated Python in a temporary directory outside the project.
- Export validation proves declared relative references are bundled; external UI prerequisites remain explicit. It does not certify real UI/tool installation or all task behavior.
- SkillSpector inspected the script-bearing v0.2.0 export without LLM analysis. Five same LP3 tool-scope leads remain; no failed executions or uninspected files were reported. Manual scope assessment is recorded in SOURCE-REVIEW.md.
- Original three long drafts and the previous website card are preserved under `evidence/original-drafts`. They are not active instructions.
- The helper tests cover direction, budget semantics, comparability, correctness refusal, invalid numbers, zero baseline, overflow, unchanged inputs, CLI exit codes and bounded input reads.

## Sources

- [Project acceptance contract](PROJECT.md).
- [Independent content review](REVIEW.md).
- [Current export validation](evidence/export-validation-v0.2.0.json).
- [Current scanner report](evidence/skillspector-export-v0.2.0.json).
- [Source and code review](SOURCE-REVIEW.md).
