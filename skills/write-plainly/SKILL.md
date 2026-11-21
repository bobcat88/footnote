---
name: write-plainly
description: Use for any prose a person or agent will read (docs, READMEs, PR and issue text, commit messages, release notes, UI copy, emails) and for agent-facing text (skills, AGENTS.md, CLAUDE.md, prompts). Cuts machine-written tells, filler and vague claims; keeps agent docs short and pointed. Runs prose-lint.
license: MIT
---

# Write plainly

## Run
`node <skills>/write-plainly/scripts/prose-lint/prose-lint.mjs <files or dirs> [--json]` → Markdown and text; skips front matter, code, inline code, URLs, "quoted" mentions. Exit 1 on FAIL; `prose-ok` on a line keeps it.

| Id | Rule | Level |
|---|---|---|
| P1 | chatbot phrase: "I hope this helps" / "Great question" / "Certainly!" / "As an AI" | FAIL |
| P2 | stock AI words: "delve" / "pivotal" / "tapestry" / "testament" / "leverage" / "seamless" / "unlock" | WARN |
| P3 | filler: "in order to" / "due to the fact that" / "it is worth noting" / "serves as" | WARN |
| P4 | framing: "not just X, but Y" / "plays a key role" | WARN |
| P5 | stacked hedge: "could potentially" / "might possibly" | WARN |
| P6 | em dash, or an en dash used as one | WARN |
| P7 | bold list label restated in its own line | WARN |
| P8 | curly quotes | WARN |
| P9 | sentence over 35 words | WARN |

The linter sees words, not meaning. Read the text back.

## Every text
- Lead with the point: the answer / what changed / what to do; context after
- Name the mechanism or the number, not the feeling: "a column rename fails the build", not "type-safe and delightful"
- One idea per sentence / active voice, name the actor / plain words: use, help, many, if
- One term per thing, no synonym cycling
- Cut: puffery / "experts say" / generic conclusions / adverbs propping a weak verb
- Could the sentence appear unchanged in another project's docs? It says nothing; cut it
- Sentence-case headings / no decorative emoji / little bold / lists only for real lists, at their natural length
- Commit and PR: what and why on the first line; how it was verified

## Text for agents (skills, AGENTS.md, CLAUDE.md, prompts)
- Every line costs context on every run: delete what the model does by default and what config, `--help` or the code already says
- Keep what nobody can find by looking: unwritten conventions / reasons / traps
- State the target behaviour; keep prohibitions for hard guardrails, paired with what to do instead
- Each step ends in a check that says when it is done: "every changed model listed"
- A pointer to detail says what it holds and when to read it; detail sits one level deep
- One home per fact; when a fact moves, delete the old copy
- Prefer known terms ("tight loop", "tracer bullet"); define a coined term once
- Skill files: `write-a-skill`

## Review pass
1. `prose-lint` → fix FAILs, judge WARNs
2. Ask "what makes this read as machine-written?" → fix what remains
3. Read it aloud once; where you stumble, split the sentence
