# Issue-writing rules & AI ownership

Every feature, fix, audit finding, or maintenance task in this repository is tracked by a GitHub issue before or alongside its pull request. Consistent structure and attribution allow maintainers to link related work to Epics and measure issue quality across humans and AI coding agents.

## 1. Required sections

Use `.github/ISSUE_TEMPLATE/story.md` or `.github/ISSUE_TEMPLATE/bug.md`. Every issue must include:

1. **`## Context & Evidence`** — Exact `file:line` references, measured numbers, or reproducible command output motivating the change.
2. **`## Problem`** (or **`## Reproduction`** for bugs) — Clear statement of what is broken, missing, or suboptimal and why it matters.
3. **`## Scope`** — Bounded list of crates, modules, docs, or scripts in scope (and explicit non-goals when helpful).
4. **`## Acceptance criteria`** — Unambiguous `- [ ]` checkboxes defining what "done" means.
5. **`## Verification`** — Exact commands (tests, benchmarks, linters, or manual checks) that prove the acceptance criteria pass.
6. **Footer links** — `Parent: #<epic>` (for Epic tracking) and `Definition of done: \`docs/DEFINITION-OF-DONE.md\``.
7. **Signature block** (for AI agents) — Mandatory `agent + model + logo` signature from [`.github/AGENT-SIGNATURES.md`](../.github/AGENT-SIGNATURES.md).

## 2. Label taxonomy

Apply at least one type label, one area label, and (when filed by an AI agent) one agent-family label:

- **Type:** `story`, `bug`
- **Area:** `memory`, `library`, `code`, `launcher`, `installer`, `encryption`, `personal-corpus`, `alias`, `usage`, `enterprise`, `engineering`, `bench`, `security`, `docs`
- **Agent family:** `agent:gemini`, `agent:claude`, `agent:gpt`

## 3. Epics & native GitHub Sub-Issues

Attach every new issue to its parent Epic (`docs/roadmap/EPICS.md`):

| Epic | GitHub Issue | Primary Area Label |
|---|---|---|
| E1 Memory | `#83` | `memory` |
| E2 Library | `#84` | `library` |
| E3 Code | `#85` | `code` |
| E4 Launcher | `#86` | `launcher` |
| E5 Installer and updates | `#87` | `installer` |
| E6 Personal memory encryption | `#88` | `encryption` |
| E7 Personal corpus | `#89` | `personal-corpus` |
| E8 Alias | `#90` | `alias` |
| E9 Usage and savings | `#91` | `usage` |
| E10 Company deployment | `#92` | `enterprise` |
| E11 Engineering | `#93` | `engineering` |
| E12 Benchmarks | `#94` | `bench` |

### Two-step native Sub-Issues attachment

GitHub's native Sub-Issues REST endpoint requires the child issue's **numeric database `id`**, not its visible issue number.

1. Look up the child issue's database `id`:
   ```bash
   gh api repos/bobcat88/footnote/issues/<child_number> --jq .id
   ```
2. Attach the child as a sub-issue under its parent Epic (use uppercase `-F` so `sub_issue_id` is sent as a JSON integer):
   ```bash
   gh api -X POST repos/bobcat88/footnote/issues/<epic_number>/sub_issues -F sub_issue_id=<child_database_id>
   ```

## 4. Ownership & AI quality analysis

AI coding agents do not share a single failure profile. Labeling issues with `agent:<family>` and signing every issue, comment, and PR with the exact model (`**<Agent> · <Model>**`) creates an auditable trail of:

- Issue clarity (missing `Parent:` links or missing signatures)
- Completion vs. `not_planned` / invalid closures by agent and model
- Regression or rework patterns surfaced during implementation and review

Run the metrics report at any time with:

```bash
python scripts/agent_metrics.py --limit 100
python scripts/agent_metrics.py --limit 100 --format json
```
