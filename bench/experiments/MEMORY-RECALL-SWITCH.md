# Recall by meaning on each prompt: split the question in two

Written 16 November 2026, before any new run. Direction approved the same day.

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

**Decision rule (fixed now):** switch the default to hybrid when, on one confirmation run over a real memory database (copy,
read-only) with questions written before the run:

- hybrid any@3 minus keyword any@3, paired bootstrap 95 % interval (10,000 resamples, seed 20261116), lower bound above zero;
- the hook's added time on an idle machine stays under 50 ms at p95 with the daemon warm, and the first prompt after the daemon
  starts never waits more than the existing 2 s fallback (keyword answers if the daemon is not ready);
- daemon memory reported (RAM of the two models), not judged.

Questions: 100, written from the memories' topics by someone other than the memory's author where possible, half French, half
English, each with the memory ids that answer it. Same-author bias is reported as a limit, not hidden.

If it passes: `resident` becomes the default for Memory (option catalogue entry updated with its RAM cost), keyword stays the
fallback. If it fails: keyword stays, and the result is recorded.

## Result (16 November 2026): passes, hybrid is the default

Questions frozen at 12:19:06 UTC (sha256 `af79058e063ffd8fb83f8619473d97bd9594858957b71ff18ee2eb326ef1b27b`) before any run, written by
Claude from the 177 active memories of a `footnote backup` copy of the test database: 100 prompts phrased as a user would type them,
50 English and 50 French, each with the memory ids that answer it, all scoped to the footnote project (global memories included, as
the hook does). Questions and per-question rankings stay in the git-ignored `bench/work/recall-switch/` because some quote
personal administration. Runner: [memory_recall_switch.py](memory_recall_switch.py), installed metrics+llama build of main
(`e07bdf9`), default pair.

| Measure | Keyword | Hybrid |
|---|---:|---:|
| any@3, all 100 | 0.77 | **0.91** |
| any@3, English 50 | 0.96 | 0.98 |
| any@3, French 50 | 0.58 | **0.84** |
| MRR@3 | 0.677 | 0.855 |

- **Recall: pass.** Hybrid minus keyword any@3 **+0.14, 95 % interval [+0.08, +0.21]**. The gain is almost all in French prompts:
  the memories are written in English, and keyword search cannot cross languages.
- **Latency: pass.** Machine idle (CPU 1.4 % average over 10 s before the run). Whole process per call, 200 calls per mode,
  interleaved: keyword p50 11.6 / p95 12.9 ms; hybrid through the warm daemon p50 24.6 / p95 26.2 ms; **+13.3 ms at p95** (limit 50).
  The 2 s fallback to keyword is unchanged.
- **Reported:** the daemon's working set with both models loaded is **448 MB** (private 434 MB); it leaves after `idle_seconds`.
- **Limits:** one author wrote the questions and, through agents, many of the memories (same-author bias); one machine; 177
  memories. The no-answer question below is untouched by this result.

Adopted: `memory.resident` defaults to on (`src/hook.rs`, option catalogue). A company policy can still force it off.

## Question 2: a separate no-answer decision (later)

Candidates, to screen on the synthetic fixtures' dev split before any held-out use:

- a multilingual cross-encoder reranker score on the top 3 (an absolute relevance score, unlike the query-relative fusion score);
- agreement: keyword and vector put the same memory first, with a margin over the second.

Bar for adoption: no-answer injection at most half of today's (≤ 50 %), and any@3 on answerable questions no more than 5 points
below hybrid without a gate. Licence and per-prompt latency reported for each.

## Not in scope

Answer accuracy of the agent (no answering or judging model is run), memory writing quality (covered by MEMORY-WRITTEN), and
company-scale cost (deferred per the 15 Nov design note).
