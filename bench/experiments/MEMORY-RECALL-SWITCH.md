# Recall by meaning on each prompt (backlog bk-gsh): split the question in two

Written 16 November 2026, before any new run. Johan approved the direction the same day ("ok with everything").

## Why the earlier bar was the wrong one

Hybrid recall stayed off because it failed a gate: inject on at most 20 % of questions that have no answer
([MEMORY-WRITTEN-RESULTS.md](MEMORY-WRITTEN-RESULTS.md): 17.75 to 24.50 % after a dev-fitted cutoff, and the cutoff then cost most of
the recall). But the recall live today has no gate at all: the hook injects the top 3 keyword hits whenever any word matches.
Measured on the context fixture, keyword injection on no-answer questions is **257/257**
([MEMORY-CONTEXT-RESULTS.md](MEMORY-CONTEXT-RESULTS.md)).

So two different questions were tied together:

1. **Switch:** does hybrid find the right memory more often than keyword, under the same no-gate policy that is live? Noise is
   unchanged by construction (both inject on every match), so this is a recall question only.
2. **Noise:** can a decision other than the fusion score tell "no memory answers this" (to stop injecting irrelevant memories)? This
   is open for both modes and is not a reason to keep the weaker one.

## Question 1: switch keyword → hybrid (no gate)

Evidence already on record, all held-out or separate fixtures, none tuned for this decision:

| Fixture | Keyword | Hybrid | Source |
|---|---:|---:|---|
| LongMemEval, 500 questions, any@5 | 0.384 | 0.504 | [MEMORY-BASELINE.md](MEMORY-BASELINE.md) |
| Synthetic memories as written, held-out, recall@3 | 25.0 % | 34.0 % | MEMORY-WRITTEN-RESULTS.md |
| Same, memories naming their subject, recall@3 | 100 % | 100 % | MEMORY-WRITTEN-RESULTS.md (ceiling) |
| Context fixture, dev, any@3 | 34.2 % | 52.4 % | MEMORY-CONTEXT-RESULTS.md |

**Decision rule (fixed now):** switch the default to hybrid when, on one confirmation run over Johan's real memory database (copy,
read-only) with questions written before the run:

- hybrid any@3 minus keyword any@3, paired bootstrap 95 % interval (10,000 resamples, seed 20261116), lower bound above zero;
- the hook's added time on an idle machine stays under 50 ms at p95 with the daemon warm, and the first prompt after the daemon
  starts never waits more than the existing 2 s fallback (keyword answers if the daemon is not ready);
- daemon memory reported (RAM of the two models), not judged.

Questions: 100, written from the memories' topics by someone other than the memory's author where possible, half French, half
English, each with the memory ids that answer it. Same-author bias is reported as a limit, not hidden.

If it passes: `resident` becomes the default for Memory (option catalogue entry updated with its RAM cost), keyword stays the
fallback. If it fails: keyword stays, and the result is recorded.

## Question 2: a separate no-answer decision (later)

Candidates, to screen on the synthetic fixtures' dev split before any held-out use:

- a multilingual cross-encoder reranker score on the top 3 (an absolute relevance score, unlike the query-relative fusion score);
- agreement: keyword and vector put the same memory first, with a margin over the second.

Bar for adoption: no-answer injection at most half of today's (≤ 50 %), and any@3 on answerable questions no more than 5 points
below hybrid without a gate. Licence and per-prompt latency reported for each.

## Not in scope

Answer accuracy of the agent (no answering or judging model is run), memory writing quality (covered by MEMORY-WRITTEN), and
company-scale cost (later, per Johan's 15 Nov note).
