---
skill_hash: db5dd09d7f5b4758
reviewed: 2026-11-21
---
# write-a-skill: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| name 1–64 lowercase-hyphen, = folder; description 1–1024 chars, what + when | [Agent Skills spec](https://agentskills.io/specification) |
| Three-tier loading: metadata ~100 tokens, body < 5000 tokens on activation, resources on demand; SKILL.md < 500 lines; references one level deep | [Agent Skills spec](https://agentskills.io/specification) |
| Only name + description load at startup | [agentskills.io](https://agentskills.io/) |

## Registry survey (2026-11-21)
- skills.sh all-time leaderboard: top installs are skill discovery, video generation, browser automation, design and planning/debugging skills. No Rust, desktop, CI/release, Windows/Linux scripting or diagram skills in the named top ranks.
- Popular database, debugging, code-review, writing and skill-authoring skills were read and distilled into our own text; licences MIT / Apache-2.0; credits in NOTICE.

## House rules (2026-11-21)
- 60-line SKILL.md limit and `- a / b / c` bullet style: chosen to keep activation cost low and the agent on task.
- `_sources.md` tied to a content hash so a skill edit flags its citations for re-review.
