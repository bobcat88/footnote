# Memory tags: a pre-filter before ranking

Built 16 November 2026 on Johan's request ("add the extra filtering"), following his notes of 15 November: tags narrow the
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

- **Data:** Johan's memories as tagged by agents during the week (only memories saved after this change carry tags), and 60
  questions written from those memories before any run, each with its answering ids and the tags a user would name.
- **Measures**, per question, with the named tags as filter against no filter: candidates kept (share of the pool), answer
  survival (answering memory still a candidate), any@3, hook-side latency. Then the same with 30 % of tags removed at random
  (seed 20261121) to simulate incomplete tagging.
- **Hook inference is adopted** (the hook names tags found as words in the prompt) only if, on those questions: survival ≥ 0.98
  with complete tags and ≥ 0.95 with 30 % removed, any@3 not lower than without filter, candidates reduced by at least a
  third, and latency added under 5 ms at p95.
