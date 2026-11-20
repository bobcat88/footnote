# Self-contained memories at 600-document bilingual scale

Measured 15 November 2026. Protocol: [MEMORY-WRITTEN.md](MEMORY-WRITTEN.md).

## Scope and limitations first

This is a controlled synthetic fixture, **not a real-user extraction evaluation, general search
accuracy estimate or comparison with ICM**. The current agent authored the histories, qualified
sentence templates, translations, questions and relevance judgments. Thirty fictional projects
each contain ten fact archetypes. Repeated patterns intentionally make weak memories confusable;
explicit project names make the qualified fixture easy. This construction favors qualification.
Perfect recall here must not be presented as expected production performance.

At the target scale, each representation stores **300 French and 300 English memories**.
These represent 300 underlying facts, not 600 independent facts. Translation pairs consume real
result slots but both count as the same relevant fact. All documents share one memory scope;
there is no project/tag/gold-derived filter. One quarter of original texts were already qualified
and remain unchanged.

Source histories and memory versions were frozen at **15:26:54 UTC**, before questions were
created and frozen at **15:28:28 UTC**. The original freeze hashes and raw files are retained.
This ordering prevents direct question-conditioned rewriting, but not same-author or template bias.

Ten projects (100 fact families) are development; twenty projects (200 families) are held-out.
All translations and questions of one project stay together. Held-out metrics use 400 answerable
and 400 no-answer questions, evenly split between French and English. Corpus documents from both
splits are searchable, as appropriate for retrieval evaluation; only cutoff fitting uses dev labels.

## Raw retrieval, before any injection cutoff

| Held-out result | Original short memory | Agent-qualified memory | Full source context |
|---|---:|---:|---:|
| Hybrid recall@3 | 34.00% | 100.00% | 99.75% |
| Hybrid recall@5 | 40.25% | 100.00% | 100.00% |
| Hybrid MRR@5 | 0.308 | 1.000 | 0.997 |
| Hybrid document precision@5 | 10.50% | 27.80% | 26.30% |
| Keyword recall@3 | 25.00% | 100.00% | 88.00% |
| Keyword recall@5 | 25.25% | 100.00% | 88.50% |

Qualified-minus-original hybrid recall@3 gain: **+66.0 points**, project-clustered bootstrap 95%
interval **[+62.75, +69.25]**, 10,000 samples, seed 20261115. This interval quantifies variation
over these twenty synthetic projects; it does not cover uncertainty about real-world tasks,
unseen writing styles, extraction errors or different fact archetypes.

Hybrid recall@3 by query language:

| Language | Original | Qualified | Full context |
|---|---:|---:|---:|
| English, 200 answerable | 30.0% | 100.0% | 100.0% |
| French, 200 answerable | 38.0% | 100.0% | 99.5% |

The lexical improvement also reaches the fixture ceiling. The evidence therefore supports
explicitly naming a memory's subject, not a claim that the embedding model itself improved.
Precision@5 is document precision: each query has two relevant translated documents among 600,
and five result slots are returned. High recall does not mean every returned document is useful.

## Automatic injection still fails

Unlike earlier diagnostic runs, **each representation received its own cutoff fitted on dev
negatives only**, before held-out metrics were calculated. The rule was unchanged: lowest top-hit
fusion threshold injecting on at most 20% of dev no-answer queries.

| Held-out hybrid after its dev-fitted cutoff | Original | Qualified | Full context |
|---|---:|---:|---:|
| Cutoff | 4.3701225183 | 4.5380160341 | 4.4563711710 |
| Recall@3 after cutoff | 9.50% | 4.00% | 8.25% |
| Injections on no-answer questions | 71/400 = 17.75% | 98/400 = 24.50% | 90/400 = 22.50% |

Qualified gated-recall gain is **-5.50 points**, clustered interval **[-8.75, -2.25]**. No-answer
injection also rises 6.75 points over the original, above the permitted five-point increase.
The automatic-injection gates fail. Improving memory text did not make a query-relative fusion
score a reliable answerability signal; this cannot be blamed solely on reusing the old cutoff.

The no-answer policy is deliberately strict: any injection counts when the requested affirmative
detail is absent. Some source memories explicitly say that a detail is unrecorded and could help
an answering agent abstain. This test measures that injection policy, **not actual hallucinations
or downstream answer accuracy**. No answering or judging LLM was run.

## Cost and runtime

Same release binary, bekko-a8m + gte-small models, SQLite/Tantivy and hybrid fusion. Each variant
was indexed in an isolated database and served through a private resident MCP stdio process.
Warm timings include JSON-RPC request/response, retrieval and query encoding, but exclude CLI
startup and database construction. Two identical-order passes ran serially; all 14,400 recorded
mode/query/pass measurements had identical repeated document rankings, with zero score drift.
CPU load was observed at 14% immediately before the run; this is not a continuously verified
idle-machine or large-scale benchmark. Timings below use first-pass held-out positive and negative
requests after warm-up, not just successful searches.

| Cost, all 600 memories | Original | Qualified | Full context |
|---|---:|---:|---:|
| Text characters | 53,451 | 73,646 | 143,357 |
| Relative text | 1.000x | 1.378x | 2.682x |
| SQLite bytes incl. sidecars | 1,425,408 | 1,474,560 | 1,646,592 |
| Tantivy bytes | 298,896 | 312,153 | 376,130 |
| Total store/index bytes | 1,724,304 | 1,786,713 | 2,022,722 |
| Relative store/index | 1.000x | 1.036x | 1.173x |
| Vector payload within SQLite | 465,600 | 465,600 | 465,600 |
| Warm hybrid p50 / p95 | 11.04 / 12.77 ms | 11.12 / 12.76 ms | 11.22 / 13.14 ms |
| First hybrid request, model cold | 1,303 ms | 1,259 ms | 1,287 ms |

Qualified text costs 37.8% more characters than the original and about half as many as full source
context; the measured store/index increases 3.6%. Shared model files are excluded. The first-call
figures are single observations, not cold-start distributions. No peak RAM, encoding build time,
long-term memory growth, extraction fidelity or live-agent compliance claim is made.

## Integration and remaining work

Added the tested writing guidance to the repository's `tools/memory_hook.py` SAVE prompt: name the
subject, include only relevant source-supported stack/platform/condition qualifiers, keep exceptions,
and avoid inferring exclusive applicability from a test environment. This clarifies the existing
self-contained-memory instruction. It is a writing-policy change, not an automated rewriting
service, schema/tag system or proof that every future agent will follow the instruction.

No installed hook or binary, personal memory database, old memories or live recall setting was
modified. Automatic hybrid injection remains unactivated. Next evidence should come from actual
agent-written memories with independent source-grounded relevance review and a separate support /
no-answer decision, rather than another cutoff on the same fusion score.

## Artifacts and reproduction

- Source-only memory generator: [memory_written_fixture.py](memory_written_fixture.py).
- Questions authored later: [memory_written_questions.py](memory_written_questions.py).
- Real MCP runner and analyser: [memory_written_run.py](memory_written_run.py).
- Reproduction helper: [prepare_memory_written.py](prepare_memory_written.py).
- Tests: [test_memory_written.py](test_memory_written.py).
- Frozen source files: `bench/work/memory-written-v2.sources.json` and `.sources.freeze.json`.
- Frozen questions: `bench/work/memory-written-v2.questions.json` and `.questions.freeze.json`.
- Run directory: `bench/work/memory-written-v2-run/`; includes binary/model/input hashes,
  per-variant raw JSONL, metadata, dev-only cutoffs and `analysis.json`.

On a fresh checkout with the existing models and release binary available:

```powershell
rtk proxy python bench/experiments/prepare_memory_written.py
rtk proxy python bench/experiments/memory_written_run.py run --output bench/work/memory-written-reproduction
rtk proxy python bench/experiments/memory_written_run.py analyse bench/work/memory-written-reproduction
```

The helper refuses existing frozen files. Recreating these published templates reproduces the
fixture; it does not create new independent held-out evidence. Preserve the original results.
Twenty-three memory-benchmark tests and five hook tests pass; `git diff --check` passes. The hook
tests emit the existing detached-helper ResourceWarning; the referenced helper had exited when
checked afterwards. No unrelated product-source edits were reverted or formatted.
