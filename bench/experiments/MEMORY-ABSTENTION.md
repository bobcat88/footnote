# Memory recall abstention: a reranker decides when no memory answers (goal 9)

Written 17 November 2026, before any data for this question was collected. Continues "Question 2" of
[MEMORY-RECALL-SWITCH.md](MEMORY-RECALL-SWITCH.md); Johan chose the reranker the same day.

## Why

The hook injects the hybrid top 3 on every prompt that matches anything: on no-answer questions that is 100 % injection
([MEMORY-CONTEXT-RESULTS.md](MEMORY-CONTEXT-RESULTS.md)). The fusion score cannot gate it: it is relative to the query, and the
fitted cutoff of 15 November failed on held-out ([MEMORY-CUTOFF.md](MEMORY-CUTOFF.md)).

The second pre-planned candidate, "keyword and vector agree on the first memory", is dropped before any run: keyword search cannot
cross languages, so on a French prompt over English memories it finds nothing and agreement would abstain, which is exactly where
hybrid recall earned its switch (French any@3 0.58 to 0.84). LongMemEval is English only and would not show it.

## Candidate

`cross-encoder/mmarco-mMiniLMv2-L12-H384-v1` at commit `1427fd652930e4ba29e8149678df786c240d8825`: Apache-2.0, 118 M parameters,
multilingual (French included), ONNX graph `onnx/model.onnx` (fp32) and `tokenizer.json`, no custom code. It scores each
(prompt, memory text) pair of the hybrid top 3 with one logit.

Policy: inject only the top-3 memories whose logit is at least `t`; inject nothing when none is. One parameter, `t`.

## Data and splits

LongMemEval-S cleaned, memories extracted as in [memory_cutoff_collect.py](memory_cutoff_collect.py) (the historical extractor),
same split by question family (`_abs` variants stay with their answerable twin), default model pair, installed engine build.
Per question: the answerable question (positive), its `_abs` twin when it exists (no answer), and one cross-database negative with
disjoint answer sessions (no answer), as in the earlier experiments.

## Decision rule (fixed now)

1. **Dev only:** choose `t` as the value that gives the lowest no-answer injection rate while answerable any@3 stays within
   5 points of ungated hybrid any@3 on dev. No other parameter is tuned; the model, its input format and the top 3 are fixed.
2. **Held-out, once, with `t` frozen:** collected only after `t` is written into this file. Passes when both hold (the bar of
   16 November):
   - no-answer injection at most **50 %** (`_abs` and cross negatives together; each also reported alone);
   - answerable any@3 at most **5 points** below ungated hybrid any@3.
   Paired bootstrap 95 % intervals (10,000 resamples, seed 20261117, by question family) are reported, not gated.
3. **Cross-language check, same frozen `t`:** on the 100 questions frozen for the recall switch (Johan's real memories, read-only
   backup copy, 50 French and 50 English, answerable only), gated any@3 at most 5 points below ungated, overall and on the French
   half. Fails the whole decision if it does not hold.
4. **Reported, not gated:** added time per prompt on an idle machine (p50 and p95, three pairs scored), model size on disk and RAM.

If all hold, the reranker gate is built into the engine (Rust, same model, logits checked against this run within 1e-3) and turned on
as an option, with Johan's go on the reported latency. If any fails, injection stays as it is and the result is recorded here.

## Not in scope

Which memory text an agent writes (MEMORY-WRITTEN), answer quality of the agent, int8 or other model variants (a later, separate
measure if this passes), and Library search.

## Result (17 November 2026): fails on dev; held-out not used; injection unchanged

Dev split, 244 question families, 488 rows (231 answerable, 257 no-answer: `_abs` and cross negatives); installed engine build of
`main`, default model pair; reranker pinned as above (hashes checked at load). Runner: [memory_abstention.py](memory_abstention.py);
rows in the git-ignored `bench/work/abstention/` (public LongMemEval text only).

| Dev measure | Ungated hybrid | Gate at the rule's threshold (`t = -10.078`) |
|---|---:|---:|
| any@3, answerable | 0.524 | 0.476 (-4.8 points; paired 95 % interval of the difference [-7.7, -2.2]) |
| No-answer injection | 100 % | **93.4 %** (`_abs` 100 %, cross 93.0 %) |

The bar is at most 50 % injection. Reaching it on dev needs `t = -8.08`, which cuts any@3 to 0.273 (half the recall). The cause is
visible in the scores: the median logit of the memories that answer a question is -8.17, and the median top logit on no-answer
questions is -8.08. On this fixture the reranker does not separate them, although it separates obvious pairs (+2 against -9, and
+5 points across French and English in the smoke check). LongMemEval memories are sentences cut from chat by the historical
extractor; the questions mostly ask for counts or details spread over several sessions, which no single sentence answers word for
word.

Decision, as fixed above: no gate; the hook keeps injecting the hybrid top 3. Following the plan of 16 November ("screen on the
dev split before any held-out use"), the held-out split was not collected, so it stays fresh for a later candidate. The
cross-language check (step 3) was not run, since step 1 already fails.

Not tried, for a later decision: the larger `BAAI/bge-reranker-v2-m3` (Apache-2.0, 568 M parameters), and a fixture of
agent-written memories with written no-answer prompts, closer to what the hook sees than LongMemEval's extracted sentences.
