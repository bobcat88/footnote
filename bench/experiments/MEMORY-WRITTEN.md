# Agent-written qualification: prospective controlled bilingual pilot

Frozen 15 November 2026 before question creation or inference. Synthetic source histories and
three human-readable memory variants are authored by the current Codex agent. This is a controlled
mechanism test, not an independently judged real-user benchmark or a leaderboard comparison.

## Writing rule

Write one concise, self-contained fact. Name its subject/project/component rather than using
unresolved references. Include stack, platform/version and triggering condition only when the
source establishes them and they matter to interpretation. Distinguish an observed test environment
from an exclusive applicability restriction. Keep exceptions and causal/remediation details.
Omit unknown details. Do not copy the conversation or generate search questions/keywords.

The agent writes the qualified variant directly; no neighbouring-sentence prefix algorithm or
separate extraction model is used. Some already-clear baseline memories should need no rewrite.

## Frozen ordering and data

1. At target scale, create 300 fictional fact families and translate each into French and
   English: **600 memories per representation**. Use 30 projects with ten fact archetypes each
   (four fixes, three preferences, three decisions). Every memory is grounded in its own source
   history. These fictional fixes are test data, not verified technical advice. Repeated archetypes
   are intentional confusable distractors: this is a templated scale/mechanism fixture, not 300
   independent naturally occurring scenarios. The earlier 36-row draft is not used.
2. Save original, qualified and full-context memory texts, then freeze their file hash **before**
   authoring questions. No memory text changes after questions exist.
3. For each fact, author two answerable paraphrases, one in each language, and two near-miss
   question whose requested detail is absent from the entire source corpus. Gold identifies the
   exact fact, not a session. The answerable questions record a source-supported expected answer;
   no-answer cases state what is missing.
4. Split **projects** by numeric ID: divisible by three is development, others held-out. This
   gives 10 dev and 20 held-out projects (100/200 fact families). Translation pairs, paraphrases,
   and negatives stay together. Both translations count as the same relevant fact and their two
   documents consume real ranking slots. Bootstrap projects, not individual translations.
   Same author for sources, memories, questions and judgments is an explicit bias;
   the ordering prevents direct question-conditioned rewriting, not author/style contamination.

## Run and gates

- Same release binary, models bekko-a8m + gte-small, SQLite/Tantivy and current hybrid fusion.
  All three variants index the same 600 translated memories in isolated temporary databases. All facts share one
  scope. There is no project/tag/gold-derived filter and no backend/model/default changes.
- Search lexical and hybrid, limit five. Report exact-fact recall@3/@5, MRR@5 and precision@5
  separately. Relevance is a frozen source-fact judgment, not an automated entailment model.
- Fit one cutoff per representation on development negatives only: the lowest top-hit fusion
  threshold injecting on at most 20% of those negatives; explicitly handle ties/reject-all.
  Evaluate held-out gated recall and false injection separately from raw ranking.
- Qualification is a raw-ranking improvement only if paired held-out recall@3 gain has a 95%
  bootstrap interval above zero. Resample project families, 10,000 draws, seed 20261115, so bilingual
  questions are not treated as independent facts.
- Automatic hybrid injection additionally needs held-out negative injection <=25%, no more than
  five points above baseline, and positive gated recall@3 gain interval. Passing a synthetic pilot
  alone does not certify production injection. Do not tune the rule after held-out results.
- Report qualified text <=2x baseline and <=75% full context, actual SQLite/Tantivy/vector bytes,
  resident warm p50/p95, first hybrid-call cold time and deterministic repeated rankings.
  These are local small-corpus measurements, not scale/performance certification.
- Use one isolated resident MCP stdio server per variant, hidden on Windows, with private state.
  Warm up each mode before timing, then run the same fixed query order twice serially. Include MCP
  request/response and engine time in the warm measurement; exclude CLI startup/database building. Stop the server
  before measuring final store files and remove all temporary databases.

## Integration boundary

A confirmed raw-ranking improvement may justify adding the tested writing instruction to the
repository's SAVE prompt, with its limits documented. This does not activate hybrid injection,
install a binary, migrate old memories, change metadata/schema, or establish live-agent compliance.
Keep the original results and hashes even if a gate fails. No new LLM service or paid model calls.

The qualified texts are authored as bilingual sentence templates filled with explicit source facts,
not produced by the old sentence-prefix heuristic. One quarter of original memories are already
self-contained and remain unchanged. This controlled construction may favor qualification and is
not evidence about an unattended LLM writer's extraction fidelity or real-world accuracy.
