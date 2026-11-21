---
name: run-benchmarks
description: Use when comparing two systems, models, prompts or configurations on quality (search relevance, accuracy, recall, judged answers) or deciding whether a change is an improvement: pre-registered decision rule, dev and held-out splits, sealed single-use test sets, matched conditions, paired bootstrap interval. Runs paired-bootstrap. Speed comparisons use make-code-faster's bench-gate.
license: MIT
---

# Run benchmarks

## Run
`node <skills>/run-benchmarks/scripts/paired-bootstrap/paired-bootstrap.mjs <a> <b> [--min 0] [--resamples 10000] [--seed 1] [--level 0.95] [--json]` → per-item scores paired by id; prints a table and the decision. Exit 0 better / 1 worse / 3 inconclusive / 2 invalid (ids differ).
- Inputs: `{"id": score}` / `[{id, score}]` / JSONL / CSV. Retrieval runs: `build-ai-features` ir-score `--out` writes this format
- better = lower bound above `--min` / worse = upper bound below 0 / otherwise inconclusive

## Before any run
- Write the decision rule and commit it: metric / data split / minimum effect / what counts as adopt, reject, inconclusive
- Splits: dev for tuning / held-out for the one confirming run / sealed set (hash committed) for a final public claim
- A sealed set is read once; after that it is spent: mark it, never tune or claim on it again
- Freeze configurations before the held-out run; a config changed after seeing held-out results needs new held-out data
- Same inputs for every system: corpus, queries, returned unit (chunk, document), depth k, hardware, warm or cold

## While running
- Failures count: a crash or timeout scores 0 for that item, never drops out of the denominator
- Record versions: system, model, index, prompt, judge model, dataset hash, seed
- Store raw per-item results; summaries are rebuilt from them
- Paid or random judges: fix the judge model and prompt / check a sample by hand / report agreement

## Reading results
- Paired comparison on the same items, not two separate means
- Many comparisons → say so; at 26 comparisons, about 1 "significant" result is expected by chance at 95%
- "Inconclusive" is a result: report it as such, never as "slightly better"
- A quality win doesn't authorise shipping alone: latency, memory and cost limits still apply
- Unequal setups (keyword-only vs hybrid, different chunking) can't support "better than"; label them as context

## Report
- Flat table: system / score / Δ vs baseline in % / interval / decision; latency and cost in the same table when measured
- Name what was not tested, the systems' settings, and where your own system loses
- Keep a results index: question / protocol / outcome / status (current, superseded, dead end) / link to receipts
