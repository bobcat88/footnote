# Footnote Memory baseline: plan and pre-registered reading

Written 14 November 2026, before any result. Footnote Library has receipts on five corpora; Footnote Memory has none. This measures the real memory path: conversations are turned into memories by extraction, stored with `remember`, and found with `memory-recall`.

## Data and metric

LongMemEval-S (500 questions, 38 to 62 past sessions each, 18,810 unique sessions, about 193 million characters). For each question: a fresh memory database, every past session extracted into memories stamped with its session id and date, then the question recalled. A memory counts for the session it came from.

Metric, as in the existing document-path receipt: share of questions whose top 5 recalled memories come from at least one (any@5) and from all (all@5) of the sessions holding the answer, plus any@10 and all@10. Reference: the document path, where whole sessions are indexed, scored any@5 0.978 and all@5 0.908 (`LONGMEM-ICM.md`).

## Runs

1. **Keyword extraction, all 500 questions** (the fallback when no LLM is configured, since removed; local, no cost). Recall twice: keyword only (what the per-prompt hook uses today) and hybrid with both models.
2. ~~LLM extraction pilot, 30 questions, through the user's Claude CLI with Haiku.~~ Cancelled 14 November 2026: one-layer design chosen, where the agent writes its own memories, so there is no separate extractor to measure (`docs/AGENT-MEMORY-DESIGN.md`, 7b). The harness was removed with the extractor.
3. **PersonaMem:** later; it needs an LLM to answer the multiple-choice questions as well.

## What each result triggers (decided now)

- Keyword extraction is expected to be far below the document path; it is a floor, reported, not judged.
- LLM pilot: if all@5 is more than 0.10 below the document path (below 0.808), extraction is losing the information questions need, and the extraction prompt becomes the next work item before anything else on memory.
- LLM pilot: if hybrid recall beats keyword recall on all@5 by 0.05 or more, hybrid recall on each prompt moves up in priority.
- With 30 questions the intervals will be wide; the pilot sizes the effect, a larger run confirms it.

## Results

### Run 1: keyword extraction, 500 questions (14 November 2026)

Raw: `bench/work/memory-baseline-none.json`. Median 30 memories per question; every question got at least one.

| Recall | any@5 | all@5 | any@10 | all@10 |
|---|---|---|---|---|
| keyword only (per-prompt hook today) | 0.384 | 0.110 | 0.480 | 0.160 |
| hybrid, both models | 0.504 | 0.176 | 0.576 | 0.232 |
| document path, whole sessions indexed (reference) | 0.978 | 0.908 | | |

By question type, hybrid any@5: temporal reasoning 0.632, multi-session 0.639, knowledge update 0.462, preference 0.400, assistant 0.286, single-session user 0.271.

Reading, as planned a floor, not judged:
- The keyword rules keep only sentences that look like preferences or fixes, so most facts a question needs ("I graduated in Business Administration") never become memories. A user without an LLM configured gets about half the questions' sessions in the top 5, against 0.98 when sessions are kept whole.
- Hybrid recall beats keyword recall by 0.12 any@5 and 0.07 all@5 on the same memories.
- Decision (14 November 2026): memory is for agents and by agents, so it does not work without an LLM, and no fallback is kept. The keyword extractor is removed; when the LLM is missing or fails, the conversation stays pending and is retried. This run stays as the measured reason.
