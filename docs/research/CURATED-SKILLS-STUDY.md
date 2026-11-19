# Curated skills study

Date: 2026-11-19. Epic: #229 (curated skills). Measurement: #232 (with/without test, cost per accepted delivery).

Goal: look at the most-used public skill repos, sort their skills by theme, compare skills that do the same job, and set the rules for our own rewrites. A rewritten skill is listed as **Curated** only once it passes the #232 test.

## Method and safety

- Repos chosen from the top of the claude-skills-leaderboard (GitHub search, 419 tracked repos) plus obra/superpowers and mattpocock/skills from web search. Awesome-lists and single apps were left out.
- Each repo is shallow-cloned on its own into `D:\Projets\_third-party-review\cs_<owner>_<repo>`, outside this repo. Hooks are off, symlinks are off, there are no submodules and LFS files are not downloaded.
- Nothing from these repos was installed or run. The scanners only read files, and they run with `python -I` from the session scratchpad.
- Every Curated skill is our own rewrite. We take ideas, not text, and any script it needs we write ourselves. That keeps third-party code out of agent sessions and stops instructions hidden in someone else's prose from reaching us.

## Repos

Skill counts are unique skill names. Translated copies (ECC ships ja-JP, zh-CN, es, tr, ko-KR and zh-TW) and per-harness mirrors (`.cursor`, `.kiro`, `.agents` and so on) are removed. Tokens are file bytes / 4. "Net" and "proc" count script files that call the network or start processes.

| Repo | Commit | Licence | Skills | Median tokens | Scripts (net / proc) | Use |
|---|---|---|---|---|---|---|
| anthropics/skills | e4bb5d9 | Apache-2.0 per skill (doc-coauthoring has no licence file) | 20 | 2,348 | 79 (4 / 12) | Source for office docs and skill authoring |
| obra/superpowers | 22f0b46 | MIT | 15 | 1,945 | 77 (2 / 5) | Source for the engineering process |
| mattpocock/skills | 5863806 | MIT | 39 | 907 | 6 (0 / 0) | Source; the shortest skills |
| addyosmani/agent-skills | 8c99fa3 | MIT | 26 | 3,487 | 57 (3 / 10) | Source for checklists |
| affaan-m/ECC | 3f14a10 | MIT | 330 | 1,986 | 1,368 (80 / 477) | Reference only: mass-produced, low signal |
| K-Dense-AI/scientific-agent-skills | ec59a29 | MIT | 177 | 3,383 | 776 (108 / 134) | Out of scope unless a project needs science |
| coreyhaines31/marketingskills | 73573e9 | MIT | 53 | 3,518 | 2 | Already triaged: 12 to cherry-pick |
| pbakaus/impeccable | b49177d | Apache-2.0 | 1 (26 harness copies) | 3,176 | 444 (79 / 111) | Installed already; compare, do not rewrite |
| Leonxlnx/taste-skill | f6b7657 | MIT | 1 | 7,742 | 157 (20 / 16) | Installed already; compare |
| emilkowalski/skills | 64eb3de | MIT | 20 | 3,030 | 0 | Source for animation |
| nextlevelbuilder/ui-ux-pro-max-skill | 2de1134 | MIT | 7 | 1,864 | 178 (16 / 48) | Reference: CSV lookup tables driven by Python |
| blader/humanizer | b48c832 | MIT | 1 | 8,098 | 1 | Compare with our `unslop` |
| ayghri/i-have-adhd | 1ecea19 | MIT | 1 | 1,800 | 20 (0 / 8) | Idea: answer first in replies |
| DietrichGebert/ponytail | 96608e0 | MIT | 6 | 817 | 59 (3 / 20) | Installed already |
| cathrynlavery/diagram-design | d74d8c2 | MIT | 1 | 8,220 | 116 (15 / 47) | Reference |
| multica-ai/andrej-karpathy-skills | 2c60614 | none | 1 | 630 | 0 | No licence: read the ideas only |

## Themes

The 703 unique skills, sorted by keyword rules. These counts are rough; they show where the volume is, not exact membership.

| Theme | Skills | Median tokens | Main sources |
|---|---|---|---|
| Marketing and SEO | 134 | 2,398 | ECC 72, marketingskills 39 |
| Science | 116 | 3,220 | K-Dense 100 |
| Design and UI | 95 | 2,985 | ECC 48, Emil 14, Addy 5, Anthropic 5 |
| Writing | 69 | 2,778 | ECC 42, Pocock 9, Addy 5 |
| Testing and debugging | 52 | 1,735 | ECC 30, Superpowers 6, Pocock 4 |
| Review and quality | 51 | 2,888 | ECC 32, Addy 4, ponytail 2 |
| Planning and process | 44 | 2,076 | ECC 25, Pocock 6, Superpowers 4 |
| Agent and meta | 30 | 2,080 | ECC 19, Pocock 4 |
| Backend and data | 23 | 2,220 | ECC 11, K-Dense 9 |
| Office docs | 19 | 2,197 | Anthropic 6 |
| Git, CI, devops | 15 | 1,688 | ECC 10 |
| Security | 5 | 1,860 | ECC 4 |

## Same job, different writing

Body tokens are loaded only when a skill triggers. The description loads in every session, for every installed skill. "Desc" is its length in characters.

| Job | Skill | Tokens | Desc | Pattern |
|---|---|---|---|---|
| TDD | Pocock `tdd` | ~900 | 150 | Short. Agree the test seams with the user first. Detail lives in linked `tests.md` and `mocking.md`. |
| | Superpowers `test-driven-development` | 2,394 | 79 | Iron law ("no code without a failing test"), a good/bad code example, a table of excuses plus a red-flag list (half of it repeats), run the full suite, a done checklist. |
| | Addy `test-driven-development` | 4,246 | 563 | "Discover the stack first": find this repo's test command instead of assuming `npm test`. Long description. |
| | ECC `tdd-workflow` | 5,793 | 294 | 80% coverage rule, a prompt-injection preamble, long. Weakest. |
| Debugging | Superpowers `systematic-debugging` | 2,382 | 91 | Root-cause phases, `find-polluter.sh` (finds the test that leaks state), reference files. |
| | Addy `debugging-and-error-recovery` | 2,792 | 599 | Checklist style. |
| | Pocock `diagnosing-bugs` | 2,166 | — | Has a script. |
| Planning | Pocock `to-spec` | 761 | — | Smallest. |
| | Superpowers `writing-plans` | 2,583 | 84 | Bite-sized tasks with exact files and commands. |
| | Addy `planning-and-task-breakdown` | 2,669 | 237 | 27 checkboxes. |
| Interview | Pocock `grilling` | 519 | — | One question at a time. |
| | Superpowers `brainstorming` | 3,797 | 215 | Ships a local browser companion (3 scripts). |
| | Addy `interview-me` | 3,770 | 485 | |
| Code review | Superpowers `requesting-code-review` | 743 | 107 | Hands the review to a subagent with a template. |
| | Pocock `code-review` | 1,680 | — | |
| | ponytail `ponytail-review` | 1,285 | — | Installed. |
| | Addy `code-review-and-quality` | 5,412 | 374 | 32 checkboxes. |
| Done check | Superpowers `verification-before-completion` | 911 | 225 | "No claim without the command output that proves it". |
| | ECC `verification-loop` | 993 | 306 | |
| Finish a branch | Superpowers `finishing-a-development-branch` | 1,945 | 101 | Lets the user choose merge, PR, keep or discard. |
| | Addy `git-workflow-and-versioning` | 3,600 | 616 | |
| | ECC `git-workflow` | 3,827 | 419 | |
| Handoff | Pocock `handoff` | 235 | — | We already have the Footnote handoff. |
| | Addy `context-engineering` | 3,878 | 198 | |
| Skill authoring | Anthropic `skill-creator` | 8,292 | 319 | Eval loop with 10 scripts (blind A/B comparison, description tuning). |
| | Superpowers `writing-skills` | 4,184 | 97 | Writes skills TDD-style: watch an agent fail without the skill first. |
| Human writing | blader `humanizer` | 8,098 | — | Long list of AI tells. Compare with our `unslop`. |
| | `i-have-adhd` | 1,800 | 244 | Answer first, then detail. |

What we learned:

1. **Shorter wins on cost, and the short skills lose little.** Pocock's TDD is a fifth the size of ECC's and keeps the rules that matter. Long skills mostly repeat themselves: Superpowers' excuse table and red-flag list say the same thing twice, and Addy's checklists repeat its prose.
2. **The description is the real fixed cost.** Addy's descriptions run 400–600 characters, and every installed skill pays that in every session. A trigger line of 80–150 characters ("Use when…") was enough for Superpowers.
3. **Load details on demand.** Pocock and Superpowers link reference files that the agent opens only when needed. The body stays small.
4. **Find this repo's commands; don't assume them.** Addy's "discover the stack" step stops the `npm test` guess in a Cargo repo. Steps like that belong in a script.
5. **Iron rules change behaviour.** One bold rule plus a short red-flag list (Superpowers) is what stops an agent from skipping a step. Keep one of each, not three.
6. **Most scripts are deterministic lookups.** ui-ux-pro-max runs CSV searches through Python, and Superpowers' `find-polluter.sh` bisects the tests. Work like that costs tokens when the model does it and nothing when a script does.
7. **Mass-produced packs are noise.** ECC has 330 skills, translations, prompt-injection boilerplate pasted into skills and 1,368 scripts. It is useful as an index of ideas and nothing more.

## Curated rules

A skill gets the Curated label when it meets all of these rules:

- **Description:** at most 200 characters. It starts with "Use when…" and names the trigger cases.
- **Body:** about 1,500 tokens or less. Longer detail goes into reference files that the body links and the agent reads only when needed.
- **One iron rule and one red-flag list.** No excuse tables.
- **A script wherever a step is deterministic:** finding the test or build command, bisecting a polluting test, pruning a merged branch, checking a page. Scripts are ours, written in Rust or stdlib Python, with no network unless the skill is about the network. Output is short, and a script fails loudly.
- **A "done when" list with evidence:** the command and its output, not a claim.
- **Clean:** no promotion, partner links, telemetry, `@latest` installs or remote fetch-and-run.
- **Traceable:** frontmatter `metadata` records the source repos and commits, and the licence. The skill is hashed in `skills/pack-manifest.json`.
- **Measured:** the #232 with/without run shows a lower cost per accepted delivery than no skill and than the best upstream version. A skill that fails stays out.

## First batch

Ordered by how often our projects need the skill.

| Curated skill | Built from | Script we write | Notes |
|---|---|---|---|
| `tdd` | Pocock (base) + Superpowers (iron rule, watch it fail, full suite) + Addy (discover stack) | `test-cmd`: detects the repo's focused and full test commands | Target ~1,200 tokens |
| `debug` | Superpowers systematic-debugging + Pocock diagnosing-bugs | `find-polluter`: bisect which test leaks state | |
| `done-check` | Superpowers verification-before-completion | none | Pairs with the Lean loop guard (#244) |
| `plan` | Pocock to-spec + Superpowers writing-plans (exact files and commands per task) | none | |
| `interview` | Pocock grilling | none | Brainstorming's browser companion is not worth its size |
| `finish-branch` | Superpowers finishing-a-development-branch + our prune rule | `prune-merged`: checks that the squash-merged head matches, then deletes the branch locally and on the remote | |
| `curate-skill` | Anthropic skill-creator (eval loop) + Superpowers writing-skills (fail first) | wraps the #232 harness and `skillevaluator quality-check` | Builds the rest |
| `office-docs` (pdf, docx, pptx, xlsx) | Anthropic, Apache-2.0 | Their scripts after a full review | Too script-heavy to rewrite; vendor pinned instead |

Not rewritten for now:

- Frontend design: `impeccable`, `taste` and `design-studio` are installed already. They go head to head under #232 instead.
- Animation: Emil's `animate` against our installed `transitions-dev`. Compare before choosing.
- Writing: `humanizer` against our `unslop`. Take any AI tells we lack; do not add a second skill.
- Security: `securitymaxxing` is installed.
- SEO and marketing: covered by the 12 marketingskills picks in [THIRD-PARTY-DUE-DILIGENCE.md](THIRD-PARTY-DUE-DILIGENCE.md).
- Science: out of scope.

## Reproduce

Scanners are in `bench/skills-study/`. They are stdlib Python and only read files. Run them from this repo with `python -I`, passing the clone folders as path arguments:

```
python -I bench/skills-study/scan_skills.py skills.json <clone dirs...>
python -I bench/skills-study/themes.py skills.json uniq.json
python -I bench/skills-study/clusters.py <review root>/
```
