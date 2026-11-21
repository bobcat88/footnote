---
skill_hash: 0ba5a36eb0342e9c
reviewed: 2026-11-21
---
# analyze-data: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| SUMMARIZE profile | [DuckDB SUMMARIZE](https://duckdb.org/docs/stable/guides/meta/summarize) |
| Polars lazy evaluation | [Polars lazy API](https://docs.pola.rs/user-guide/lazy/using/) |
| Chronological splits | [scikit-learn TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) |
| Backtest overfitting from repeated selection | [Bailey et al., Probability of Backtest Overfitting](https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf) |
| Sharpe ratio annualisation under serial correlation | [Lo, The Statistics of Sharpe Ratios (2002)](https://doi.org/10.2469/faj.v58.n4.2453) |
| Fill modelling, corporate actions | [QuantConnect: trade fills](https://www.quantconnect.com/docs/v2/writing-algorithms/reality-modeling/trade-fills/key-concepts) |

## Field evidence
- Internal analysis and trading projects: survivorship (delisted names), same-close fills and untracked variant counts were the recurring errors.
