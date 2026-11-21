---
name: analyze-data
description: Use when answering a question from data (CSV, Parquet, spreadsheets, SQL exports, logs, market data) or reviewing someone's analysis or backtest: question first, provenance, joins and duplicates checked, numbers reconciled, uncertainty shown, no look-ahead in time-series work. DuckDB and Polars one-liners.
license: MIT
---

# Analyze data

## Run
- Profile: `duckdb -c "SUMMARIZE SELECT * FROM 'data.parquet'"` → types / nulls / min-max / distinct counts per column
- Key unique: `duckdb -c "SELECT k, count(*) n FROM 'f.csv' GROUP BY k HAVING n > 1 LIMIT 20"`
- Join check: row count before and after every join; a join that grows rows needs a reason
- Large files: Polars lazy (`pl.scan_parquet(...)...collect()`) or DuckDB on the file; never load-all then filter

## Before computing
- Write the question / the population / the unit of one row / the output and how precise it must be
- Record source, export date or version, filters applied; keep raw files read-only
- Check units, currency, time zone, encoding, decimal separator; dates parsed, not strings
- Missing values: count per column, decide drop / fill / flag, say which
- Use the project's stack; a small question doesn't justify a new library

## While computing
- One pipeline from raw to result, rerunnable: script or notebook run top to bottom, no hand edits in between
- Assert as you go: row counts / no duplicate keys / totals match the source / no negative where impossible
- Reconcile one number by hand or against an independent source before trusting the rest
- Aggregates: say mean or median and why / show counts beside percentages / tiny groups flagged

## Time series and backtests
- Fix the data version, the universe as it was at each date (delisted names stay in), corporate actions, calendar, time zone
- Signal time < order time < fill time; a signal from today's close can't fill at today's close
- Costs in: fees / spread / slippage / financing, plus a sensitivity run at 2× costs
- Chronological split; fit scaling and parameters on the training window only; purge overlapping labels at the boundary
- Log every variant tried, including dropped ones; a holdout reused for selection stops being a holdout
- Report net return / max drawdown / volatility / turnover / trade count / benchmark on the same dates; Sharpe with its risk-free rate and frequency
- Vectorised or accelerated code matches a slow reference implementation to a stated tolerance
- Taxes and personal circumstances are a separate step from strategy returns; never invent tax rules

## Deliver
- Answer first, in one sentence, with the number and its uncertainty or range
- Then: method / data version / checks run / limits / what would change the answer
- Charts: one message each / labelled axes and units / start bars at zero
- Hand over the script and the input hashes so someone else gets the same numbers
- Speed of the pipeline and quality of the result are separate claims (`make-code-faster`, `run-benchmarks`)
