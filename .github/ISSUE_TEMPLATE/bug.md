---
name: Bug Report
about: Report a reproducible defect with a failing test before the fix
title: ""
labels: bug
assignees: ""
---

## Context & Evidence
<!-- Exact file:line locations, logs, or version/commit where the bug occurs. Do not name analyzed/inspiration external tools unless directly benchmarking against them (see docs/ISSUE-RULES.md). -->

## Reproduction
1. Step 1
2. Expected vs. actual behavior

## Scope
<!-- Affected crates, commands, UI components, or platforms. -->

## Acceptance criteria
- [ ] Regression test added and observed failing before the fix
- [ ] Fix implemented and full check suite passing

## Verification
```bash
# Exact test/command proving the regression and fix
```

<!-- Apply the area label (e.g. memory, library, code, launcher, engineering) and agent:<family> (agent:gemini | agent:claude | agent:gpt) when filed by an AI agent. Also attach this issue as a sub-issue under its parent Epic using docs/ISSUE-RULES.md. -->
Parent: #<epic> · Definition of done: `docs/DEFINITION-OF-DONE.md`

<!-- AI agents: append your signature block from .github/AGENT-SIGNATURES.md below. -->
