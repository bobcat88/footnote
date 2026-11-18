---
name: backtesting-rigor
description: Review or evaluate trading backtests for temporal leakage, execution
  costs, selection bias and reproducibility. Use for strategy experiments; results
  do not authorize live trading.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 60bb6fc3a357f11c8e40b5399071f465b302c1c175991c769011066f7d7ccbc8
  source-card: AI-Agents/skills/backtesting-rigor/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Backtesting rigor



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Establish temporal and financial correctness

- Freeze data vintage, universe membership, corporate-action treatment, timezone, trading calendar and available-at timestamps. Delisted assets and historical membership matter.
- Separate signal time, order submission and fill time. A close-derived signal cannot assume execution at that same close without an executable model.
- Include fees, spread, slippage and applicable financing, with sensitivity scenarios. Label simulated fills and missing market data.
- Keep chronological holdouts. Fit transformations and select parameters using training data only. TimeSeriesSplit can preserve ordering and leave a gap, but a generic gap does not automatically purge overlapping financial labels.
- Record every strategy/parameter trial, including abandoned variants. Repeated selection against the same holdout weakens it. The backtest-overfitting literature addresses this selection problem; no fixed Sharpe cutoff proves an edge.
- Where household investment conclusions are drawn, apply the project’s French tax and marital-framework requirements separately from raw strategy returns.

## Measurements and checks

Report net returns with sampling period and annualization assumptions, drawdown, volatility, turnover, exposure, trade count and a named benchmark on identical sessions. Sharpe must state risk-free assumption and frequency; dependence makes naive square-root annualization questionable.

Measure runtime and memory independently of investment performance. Accelerated/vectorized/JIT results must match a reference calculation within declared numeric tolerance, including cash flows and fees. Keep research claims distinct from investment recommendations.

## Candidate helper for round 2

Validate timestamps, cash/position conservation, reference parity and declared comparison sessions on synthetic fixtures. It cannot establish profitability or eliminate all data biases.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html
- https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf
- https://alo.mit.edu/publications/page/18/ (author's abstract for The Statistics of Sharpe Ratios, 2002; serial-correlation/annualization caution only, not a full mathematical audit)
- https://www.quantconnect.com/docs/v2/writing-algorithms/reality-modeling/trade-fills/key-concepts
- https://www.quantconnect.com/docs/v2/writing-algorithms/securities/asset-classes/us-equity/corporate-actions
- https://github.com/hudson-and-thames/mlfinlab/blob/master/mlfinlab/cross_validation/cross_validation.py (purging overlapping label intervals; no code imported)

