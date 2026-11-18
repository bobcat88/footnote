---
type: protocol
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Footnote Toolkit Evaluation
date_maj: 2026-11-17
---
# Footnote Toolkit Evaluation

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Footnote Framework]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## Pilot protocol

Three real-project review tasks use frozen source documents, not live systems or artificial fault injection. They evaluate review/decision quality, not implementation success. Each arm uses a fresh agent, the same task and input, and inherited provider/model settings. Baseline receives no curated toolkit; treatment loads named relevant skills. Global harness rules may still apply to both arms. Skills were developed with related project context, so this is a development pilot, not a pristine held-out generalization test.

Tasks and frozen files:

1. Footnote/ICM comparison: decide what comparisons the documented suite can support and what must precede a product superiority claim (`rag-comparison.txt`). Treatment: llm-rag-quality.
2. simple-code-gui release review: assess release readiness from the supplied Tauri configuration and state what cannot be established (`tauri-config.json`). Treatment: tauri-desktop-app.
3. RingMaster benchmark review: identify which numerical conclusions can justify engineering changes and what measurements are missing (`kernel-benchmarks.txt`). Treatment: rust-performance and sqlite-sql-performance.

Maximum response 250 words per task; no code changes, installations, external calls or running benchmarks. Review only the frozen inputs and, in treatment, designated skills. Do not use the project review/evaluation documents as answer keys.

## Scoring rules frozen before runs

Each task has five evidence-backed criteria, scored 0 (missed/incorrect), 1 (partial), 2 (clear and correct). Also record unsupported claims, unsafe recommendations, and unnecessary work. Parent scoring is not blind; disclose this limitation. An independent blind pass would be stronger.

| Task | Criteria |
|---|---|
| RAG | Distinguishes lab from shipped systems; recognizes unequal ingestion/modes/result units; separates component latency from end-to-end; requires frozen/held-out and paired comparable evidence; treats file inventories as incomplete footprint rather than RAM/total-size proof |
| Desktop | Recognizes null CSP risk without claiming an exploit; recognizes development updater key; does not treat devUrl presence as proof a release contacts localhost; recognizes Linux-only bundle target and missing target requirements; requests built-artifact/install/update/capability evidence rather than certifying from config |
| Kernel | Recognizes warm/single-run evidence limitations; compares durability equivalents; includes WAL in footprint; rejects self-recall as relevance/keyword ceiling evidence; treats projected scale/startup estimates as hypotheses requiring actual workload/turn measurements |

## Efficiency accounting

Record model settings, input hashes, start/end times where available, response lengths, tool use and total token usage only when exposed by the harness. Character counts are not token counts. Unknown usage is not zero; no cost or token-efficiency winner may be declared without it. Small quality pilots cannot establish a universal gain or reliable latency percentile.

## Current state

Six runs completed with fresh agents. The baseline already covered most issues well. Parent scoring found equal coverage in the RAG and desktop tasks, and fuller WAL/checkpoint/durability coverage in the kernel skill arm. Both desktop arms omitted an explicit development-URL distinction; neither made the false release-contact claim. This is a small, nonblind development observation, not proof of a general benefit.

Per-run token usage and task start/end timestamps were subsequently recovered from six explicitly identified local child-session logs, each linked by parent ID and agent path. [Runtime usage receipts](evidence/pilot-runtime-usage.json) retain cumulative input/cache/output accounting, elapsed time, model/effort, tool-call count and original final answers. Cached input is a subset of input; reasoning is not added again to output. The API cost remains unknown: no invoice or subscription-headroom conversion is inferred.

This changes the evidence from unknown usage to a measurable single-run pilot. It does not establish a general efficiency winner: one kernel baseline used an additional inference/tool cycle, the arms were not randomized, and substantial shared harness context remains even with fresh children. Compare per task rather than attributing an aggregate change solely to skills. The runs are review tasks, not implemented project changes, and do not establish usefulness across all 17 skills. Keep behavioral claims experimental pending broader evidence.

Transcribed answers and criterion scores are in [pilot-results.json](evidence/pilot-results.json); formatting and inline source references were normalized. Original final answers were recovered into the runtime receipt, so subsequent scoring can use the exact outputs rather than the transcription. The standalone comparator's unit tests are separate evidence.

## Sources

The next study is defined in [[AI-Agents/projects/footnote-framework/FULL-EVALUATION|Flow three-arm evaluation]]. It compares baseline, selected skills and Flow, including startup, per-agent/per-run tokens, accepted-delivery latency, quality, rework and recovery. No three-arm confirmation runs are complete. The earlier pilot remains unchanged.

- [Input fingerprints](evidence/pilot-input-hashes.json).
- Frozen copies under `evidence/pilot-inputs`, from BorgKnowledge, simple-code-gui and RingMaster.
