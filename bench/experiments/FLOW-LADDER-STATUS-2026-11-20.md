# Flow complexity ladder: status, 20 November 2026

**Work in progress. The run is not finished.** 119 of 132 planned cells were collected. The runner stopped on 19 November at 13:06 (last log line: cell 119) and was not restarted; no process is running. Nothing here comes from a new run. Every number is computed from the raw per-cell records in `flow/build/ladder-v2/results.json` (git-ignored; study `flow-complexity-ladder-claude-v1`, run 2, prices read 18 November 2026).

## What was run

- 3 models (Haiku 5.5, Sonnet 5.5, Opus 5.5), all at **medium reasoning only**.
- 4 task levels, L1 (5 min budget) to L4 (12 min), each with an independent checker that decides acceptance.
- 5 arms: AI alone, AI + skills, Flow, Footnote only, Footnote + skills. Planned repetitions per model and level: 3 for AI alone, AI + skills and Flow; 1 for Footnote only and Footnote + skills. That gives 3 x 4 x 11 = 132 cells.
- Collected: Haiku 44 of 44, Sonnet 43 of 44, Opus 32 of 44. **Missing: all of Opus repetition 3 (12 cells) and one Sonnet repetition 3 cell.** Opus therefore has 2 runs, not 3, per level for AI alone, AI + skills and Flow.

## Results by model and arm

Accepted = the independent checker passed and the cell was not contaminated (see below). Cost, time and tokens per accepted delivery divide the total of **all** runs in that group, accepted or not, by the number accepted, so failed runs count against their arm. Cost is the list API price (an upper bound for negotiated rates). Time is wall seconds including Claude Code start-up. Tokens are all tokens, including cached input.


### Haiku 5.5, medium reasoning

| Arm | Accepted n/N | Cost per accepted | Time per accepted | Tokens per accepted |
|---|---:|---:|---:|---:|
| AI alone | 6/12 | $0.037 | 153 s | 827 k |
| AI + skills | 7/12 | $0.034 | 142 s | 730 k |
| Flow | 6/12 | $0.042 | 159 s | 894 k |
| Footnote only | 4/4 | $0.020 | 80 s | 457 k |
| Footnote + skills | 3/4 | $0.029 | 115 s | 671 k |

### Sonnet 5.5, medium reasoning

| Arm | Accepted n/N | Cost per accepted | Time per accepted | Tokens per accepted |
|---|---:|---:|---:|---:|
| AI alone | 8/12 | $0.257 | 79 s | 322 k |
| AI + skills | 10/11 | $0.185 | 60 s | 227 k |
| Flow | 10/12 | $0.260 | 69 s | 338 k |
| Footnote only | 4/4 | $0.155 | 50 s | 178 k |
| Footnote + skills | 3/4 | $0.266 | 72 s | 265 k |

### Opus 5.5, medium reasoning

| Arm | Accepted n/N | Cost per accepted | Time per accepted | Tokens per accepted |
|---|---:|---:|---:|---:|
| AI alone | 5/8 | $0.727 | 151 s | 359 k |
| AI + skills | 5/8 | $0.754 | 168 s | 433 k |
| Flow | 2/8 | $1.701 | 294 s | 1,007 k |
| Footnote only | 4/4 | $0.465 | 102 s | 287 k |
| Footnote + skills | 1/4 | $1.806 | 392 s | 1,052 k |

## Accepted by level (n accepted / n run)

| Model | Arm | L1 | L2 | L3 | L4 |
|---|---|---:|---:|---:|---:|
| Haiku | AI alone | 0/3 | 2/3 | 2/3 | 2/3 |
| Haiku | AI + skills | 1/3 | 2/3 | 3/3 | 1/3 |
| Haiku | Flow | 0/3 | 2/3 | 2/3 | 2/3 |
| Haiku | Footnote only | 1/1 | 1/1 | 1/1 | 1/1 |
| Haiku | Footnote + skills | 0/1 | 1/1 | 1/1 | 1/1 |
| Sonnet | AI alone | 3/3 | 3/3 | 2/3 | 0/3 |
| Sonnet | AI + skills | 3/3 | 3/3 | 3/3 | 1/2 |
| Sonnet | Flow | 3/3 | 3/3 | 1/3 | 3/3 |
| Sonnet | Footnote only | 1/1 | 1/1 | 1/1 | 1/1 |
| Sonnet | Footnote + skills | 1/1 | 1/1 | 0/1 | 1/1 |
| Opus | AI alone | 2/2 | 0/2 | 2/2 | 1/2 |
| Opus | AI + skills | 2/2 | 1/2 | 0/2 | 2/2 |
| Opus | Flow | 1/2 | 1/2 | 0/2 | 0/2 |
| Opus | Footnote only | 1/1 | 1/1 | 1/1 | 1/1 |
| Opus | Footnote + skills | 0/1 | 0/1 | 0/1 | 1/1 |

## What the data shows

- **Footnote only was accepted in 12 of 12 cells** (one run per model and level), with the lowest cost, time and tokens per accepted delivery of any arm for each of the three models. AI alone was accepted in 19 of 32. With one run per cell this is a small sample: it supports "Footnote only performed well here", not a size of effect.
- **Flow shows no saving in this data.** Accepted 18 of 32 against 19 of 32 for AI alone; cost per accepted delivery was higher on Haiku and Opus and level on Sonnet. On Opus, Flow was accepted in 2 of 8 cells, but 3 of its 8 cells were contaminated (below), and 2 of those 3 had passed the checker.
- **AI + skills** was accepted in 22 of 31 and was cheaper per accepted delivery than AI alone on Haiku and Sonnet, slightly dearer on Opus.
- **Footnote + skills** was accepted in 7 of 12, below Footnote only. 2 of its 4 Opus cells were contaminated, both of which had passed the checker.
- Difficulty does not rise smoothly with level: Opus was accepted in 3 of 8 at L2 and L3 but 5 of 8 at L4, and Haiku failed most L1 cells (2 of 11).

## Contaminated cells

14 of 119 cells are marked contaminated: the worker's tool calls reached the benchmark project or a parent folder (for example a drive root). The runner marks them not accepted whatever the checker says. 10 of the 14 had passed the independent checker, so the accepted counts above are conservative for the arms affected. Contaminated cells per model and arm, with how many passed the checker: Opus AI alone 2 (1), Opus Flow 3 (2), Opus Footnote + skills 2 (2), Opus AI + skills 1 (0), Sonnet AI alone 2 (2), Sonnet Flow 2 (2), Sonnet AI + skills 1 (1), Haiku AI + skills 1 (0). No Footnote-only cell was contaminated.

## What is not measured

- **Higher reasoning levels.** Only medium was run. Whether Flow helps at high reasoning is not checked; the conclusion above holds for medium only.
- **Flow against Footnote only.** Flow and Footnote only are different arms with different run counts (3 against 1 per cell); the data does not isolate what Flow adds on top of Footnote.
- **Sample size.** 3 runs per cell for AI alone, AI + skills and Flow (2 for Opus), 1 for the two Footnote arms. No confidence intervals; differences of one or two accepted cells are within noise.
- **Missing cells:** Opus repetition 3 and one Sonnet cell, so the ladder was not completed.
- **Footnote memory is empty in every cell.** The Footnote arms measure the tools and harness overhead, not a filled memory.
- **Prices are list prices;** no negotiated rate or subscription effect.
- Known frame limits from the run manifest: L1 and L2 task texts mention "other arms" and "hidden checks"; Flow handoff files carry a Codex engine label although the worker is Claude Code.

