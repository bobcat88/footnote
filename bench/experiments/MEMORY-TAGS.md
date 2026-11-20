# Memory tags: a pre-filter before ranking

Built 16 November 2026 on request ("add the extra filtering"), following notes from 15 November: tags narrow the
candidate pool before keyword and meaning ranking; they are not expected to guarantee an answer; they are judged on candidate
reduction, survival of the relevant memory (including when tags are incomplete), ranking and latency, separately from any
"is there an answer" decision.

## What exists now

- **Scope before ranking.** Recall used to take the 20 best matches across every project and then drop other projects'
  memories, so a relevant memory of the current project could be pushed out by other projects. The project and global scope
  is now a pre-filter (`records::Filters::only`), applied before ranking in both keyword and hybrid search. The 100-question
  recall check gives the same scores as before (any@3 hybrid 0.91, keyword 0.77), as expected: all its answers are in scope.
- **Tags.** At most 10 per memory, lowercase words or hyphenated phrases (`rust`, `windows`, `daemon`, `bugfix`, `resolved`).
  Set with `footnote_remember` (`tags`), changed with `footnote_tag` / `footnote memory-tag <id> --add x --remove y`, shown by
  recall and `memory-list`. The save request asks agents for 1 to 5 tags when the conversation supports them.
- **Filtering by tag.** `footnote_recall` (memory databases) and `memory-recall --tag` rank only memories that share a tag
  **or have no tag at all**: an untagged memory is unknown, not excluded, so incomplete tagging cannot hide it.
- The per-prompt hook passes no tags: it cannot know them from a prompt. Whether it should infer them is the question below.
- Storage: a `memory_tags` table created on the first write, not through a schema version, so read-only opens of existing
  databases keep working after the upgrade.

## Evaluation, after a week of real tags (with the 21 November memory review)

Rule written now, before any tagged memory exists:

- **Data:** Real memories as tagged by agents during the week (only memories saved after this change carry tags), and 60
  questions written from those memories before any run, each with its answering ids and the tags a user would name.
- **Measures**, per question, with the named tags as filter against no filter: candidates kept (share of the pool), answer
  survival (answering memory still a candidate), any@3, hook-side latency. Then the same with 30 % of tags removed at random
  (seed 20261121) to simulate incomplete tagging.
- **Hook inference is adopted** (the hook names tags found as words in the prompt) only if, on those questions: survival ≥ 0.98
  with complete tags and ≥ 0.95 with 30 % removed, any@3 not lower than without filter, candidates reduced by at least a
  third, and latency added under 5 ms at p95.

## Early run, 16 November 2026: the tag engine fails, it stays off

The engine was requested the same day, so it was built (`memory::prompt_tags`, option `memory.infer_tags`, default **off**;
`memory-recall --infer-tags`) and checked at once instead of after the week. Deviations from the plan above, written before the
run: Claude tagged the 164 in-scope memories of a copy of the test database (386 tag pairs, 117 distinct tags, from each memory's
text) instead of agents during a week, and the questions are the 100 of [MEMORY-RECALL-SWITCH.md](MEMORY-RECALL-SWITCH.md)
(written earlier, seen by the same author: bias toward passing). "Candidates reduced" is measured on prompts that name a tag;
the others are unchanged by design. Runner: [memory_tag_engine.py](memory_tag_engine.py); hybrid recall, top 3; CPU 7 % before
the run.

| Measure | Rule | All tags | 30 % of tags removed |
|---|---|---:|---:|
| Prompts naming a tag | | 66 / 100 (EN 42, FR 24) | 62 / 100 |
| Answer survives the filter | ≥ 0.98 / ≥ 0.95 | **0.94** | **0.90** |
| any@3, without → with | not lower | 0.91 → **0.87** | 0.91 → **0.82** |
| Candidates kept, prompts naming a tag | ≤ 2/3 | 4 % | 12 % |
| Added latency, p95 (keyword path) | < 5 ms | −0.1 ms | |

**Fails** on survival and any@3, with complete tags and more so with incomplete ones. It narrows hard and costs nothing, but a
single incidental tag word in the prompt ("logo", "site", "search", "install", "scifact") becomes a hard filter that drops the
answer, and it helped on one question only. Tags stay useful as an explicit filter an agent chooses (`footnote_recall` `tags`);
automatic inference stays off. Not retried on these questions (they are now spent for this question). An idea for a later,
separately registered test: tags as a ranking boost rather than a filter.
