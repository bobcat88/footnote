# Skill ranking eval (#279), 2026-11-21

Question: can Footnote pick the one skill that matches a prompt, and inject nothing when none matches? And at what catalogue size does that break?

Result: hybrid ranking with a 40% topic-word coverage gate injects the right skill on 30% of matching prompts, a wrong one on 4%, and fires on 0 to 7% of plain coding prompts, as long as the catalogue holds 100 skills or fewer; at 238 skills false injections on plain prompts rise to 13%.

## Setup

- Corpus: the 238 skills installed on the maintainer's machine (`~/.claude/skills` and plugin skills), one Library document per skill holding its name and `description`. Kept local, not committed: third-party skill texts.
- Labels: 38 prompts written for the test. 23 name one of 10 narrow target skills (CSS transitions, a deploy checklist, two security reviews, C4 diagrams, browser testing, bot protection, SQL, prose de-slopping, web performance, background removal); 15 are plain coding tasks that should get no skill.
- Sizes: the 10 targets, then 30, 100 and 238 skills (targets plus seeded random distractors).
- Ranking: `footnote search`, lexical (BM25) and hybrid (bekko-a8m + gte-small), top 3.
- Gates on the top hit: `coverage>=c` is Footnote's fetch-on-miss rule (`footnote_library::miss`): the hit must hold at least `c` of the prompt's topic words. `agree`: lexical and hybrid rank the same skill first. `always`: no gate.
- Script: [skill_ranking.py](skill_ranking.py). Windows 11, release build of 2026-11-21.

Small sample (23 positive, 15 negative prompts): treat the numbers as a direction, not as proof.

## Results (hybrid unless noted)

"Right" and "wrong skill" are shares of the 23 prompts that have a target; "fired on none" is the share of the 15 plain prompts that got a skill.

| skills | gate | top-1 | top-3 | right | wrong skill | fired on none |
|---:|---|---:|---:|---:|---:|---:|
| 10 | always | 78% | 96% | 78% | 22% | 100% |
| 10 | coverage>=0.4 | 78% | 96% | 30% | 0% | 0% |
| 30 | coverage>=0.3 | 78% | 96% | 43% | 4% | 33% |
| 30 | coverage>=0.4 | 78% | 96% | 30% | 4% | 7% |
| 30 | coverage>=0.5 | 78% | 96% | 22% | 0% | 0% |
| 30 | agree | 78% | 96% | 57% | 17% | 53% |
| 100 | coverage>=0.4 | 70% | 87% | 30% | 4% | 0% |
| 100 | coverage>=0.5 | 70% | 87% | 22% | 0% | 0% |
| 238 | coverage>=0.4 | 57% | 83% | 30% | 9% | 13% |
| 238 | coverage>=0.5 | 57% | 83% | 22% | 4% | 13% |
| 238 | lexical, always | 48% | 61% | 48% | 52% | 100% |

Full table (lexical and hybrid, every gate): run the script; it prints the markdown table and writes `results.json`.

## Decisions

- **Ranking:** hybrid. It beats lexical on top-1 at every size (78 vs 57% at 30 skills, 57 vs 48% at 238).
- **Gate:** topic-word coverage of the top hit at **0.4**. No ranker score is needed, so the gate is the same for lexical and hybrid. Rank agreement is not a gate: it fires on about half of the plain prompts.
- **Catalogue cap:** **100 skills**. Up to 100 the gate keeps plain prompts clean (0 to 7%); at 238 it does not (13%).
- **Recall is low (30%) on purpose**: a missed skill costs nothing beyond the Footnote-only arm, a wrong one costs tokens and attention (#278). Descriptions written for ranking, with the words users actually type (#282), are the lever to raise it.
