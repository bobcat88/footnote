---
name: write-a-skill
description: Use when creating, rewriting, merging or reviewing an agent skill (a SKILL.md folder): naming, the description that makes it trigger, terse body format, references and scripts, the _sources.md research file tied to the skill hash, trigger tests, and the skill-check lint.
license: MIT
---

# Write a skill

A skill focuses an agent on one job. It is not a manual: what the agent must do / run / check, nothing else.

## Run
`node <skills>/write-a-skill/scripts/skill-check/skill-check.mjs [skills-dir|skill-dir…] [--stamp] [--deny terms.txt] [--json]` → exit 1 on FAIL. Tests: `node --test scripts/skill-check/tests`.

| Id | Check | Level |
|---|---|---|
| S1 | front matter: `name` = folder, lowercase-hyphen ≤ 64 / `description` ≤ 1024 / `license` | FAIL / WARN |
| S2 | SKILL.md > 60 lines (house) / > 500 (spec) | WARN / FAIL |
| S3 | every relative link resolves | FAIL |
| S4 | `_sources.md` exists and its `skill_hash` matches the skill | WARN |
| S5 | Sources section left in SKILL.md | WARN |
| S6 | a `--deny` term (private names, paths, clients) in any file | FAIL |

`--stamp` after re-checking the claims: writes `skill_hash` + `reviewed` date into each `_sources.md`.

## Layout
```
<name>/SKILL.md        front matter + terse body (loaded on activation)
<name>/references/*.md detail loaded on demand, one level deep, one topic per file
<name>/scripts/<tool>/ short-named script + tests/ (+ fixtures/)
<name>/_sources.md     citations, research, field evidence; maintainers only, never synced to agents
```
`_`-prefixed files and folders are excluded from the hash and from agent sync.

## Name + description
- Name = verb + object, readable at a glance (`launch-website`, `manage-databases`), not a brand or a codename
- Description = what it covers + when to use it, with the words a user would type; only these ~100 tokens are always loaded, so they decide triggering
- Name the scripts it runs in the description ("Runs launch-check …")

## Body format
- Purpose line / `Run` block with commands / rule table (id / check / level) / rules as `- a / b / c` bullets / `Report` line
- Every rule checkable or actionable; no history, no motivation essays, no "it is important to"
- Facts with numbers come from a primary source recorded in `_sources.md`; uncited numbers out
- Anything a script can decide → a script with tests, not prose. Zero-dependency Node or Python first; Rust when speed matters
- Detail an agent needs only sometimes → `references/`; link it from the line that needs it

## Merging + borrowing
- Similar skills → one skill with sections; overlapping rules stated once, cross-link the rest by skill name
- Third-party skills: read without running, confirm upstream + licence, rewrite in our words, credit in NOTICE; no third-party skill names in public skill files
- Private names, paths, clients → `--deny` list kept outside the repo

## Test
- 3 prompts that should trigger it + 2 near-misses that shouldn't; check the description alone picks correctly
- Run one real task with the skill loaded; fix what the agent misread
- `skill-check` clean → `--stamp`
