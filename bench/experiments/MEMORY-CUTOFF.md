# Memory recall by meaning on each prompt: relevance cutoff

Written 15 November 2026, before any measurement.

## Why

The memory hook can now recall by meaning on each prompt, through a resident daemon started on demand (`"resident": true` in
`memory.json`, off by default). Search by meaning always returns its nearest neighbours, even when no memory is relevant (live check,
14 Nov: a French question about table formatting returned two design preferences). Injected noise costs tokens and can mislead the
agent, so hybrid recall goes on only with a cutoff below which nothing is injected.

## Data

LongMemEval-S, the same memory databases as `MEMORY-BASELINE.md` run 1 (memories stamped with their session). Positives: questions
whose answer sessions produced at least one memory; negatives: the abstention questions (ids ending `_abs`, no answer in the history)
plus, for every question, the same query against another question's database (nothing relevant by construction). Questions are split
by id hash: half to choose the cutoff (dev), half to judge it (held-out).

## Rule

- **Cutoff, chosen on dev:** the lowest top-hit hybrid score such that at most 20 % of negatives inject anything.
- **Pass, on held-out:** hybrid with the cutoff beats keyword recall (today's hook) on any@3 for positives, paired bootstrap 95 %
  interval above zero, while injecting on at most 25 % of negatives (keyword's own rate is reported next to it).
- **Then:** `"resident": true` in the user's `memory.json`, and the cutoff becomes the default of `memory-recall --resident`. If it fails,
  resident stays off and keyword recall stays.
- Reported, not judged: hook latency with the daemon warm and cold, daemon RAM.

## Result: no activation (15 November 2026)

The historical keyword-extraction method was recovered from Git and rebuilt through `remember` and
`rebuild-embeddings` on 500 LongMemEval-S questions. The current release build collected the actual
`fusion_score` for each hit. The earlier `memory-recall` implementation looked for `score`, so its
scores were null; this was corrected before the full run. Question IDs were hashed into the two
splits, grouping `_abs` variants with their original question. A cross-database negative was paired
within the split only when the target's answer-session IDs were absent from the source database.

| Held-out measure | Result |
|---|---:|
| Positives | 239 |
| Negatives, including cross-database pairs | 273 |
| Keyword any@3 | 31.80% |
| Hybrid with frozen cutoff any@3 | 26.78% |
| Difference, paired bootstrap 95% interval | -9.21 to -1.26 percentage points |
| Negative injection rate | 21.61% |

Cutoff selected from 257 dev negatives: **2.341051842888992**. The held-out negative rate meets the
25% ceiling, but hybrid does not beat keyword; the registered gate **fails**. Keep `resident` off
and do not deploy this cutoff. Bootstrap used 10,000 resamples, seed 20261115.

Evidence: `bench/work/memory-cutoff-validated-v1.jsonl` and its `.manifest.json` contain the raw
score records and SHA-256 identities. Recompute with
`rtk proxy python bench/experiments/memory_cutoff_analysis.py bench/work/memory-cutoff-validated-v1.jsonl`.
Each question's temporary database was discarded; the JSONL records session-level relevance flags,
not memory text. Earlier smoke files and the interrupted runs are not part of this result.

Limitations: these are heuristically extracted historical memories, not agent-written memories;
session membership is a proxy for relevance, not verification that a memory answers the question.
Disjoint answer-session IDs do not prove that a cross-database question is semantically unrelated.
The mixed negative set includes repeated paired questions and is not an independent personal-memory
test. The keyword negative injection rate, warm/cold hook latency and daemon RAM were not collected;
the test should not be represented as a complete production evaluation. PersonaMem was not run.
The fusion score is query-relative and is not a calibrated probability.
