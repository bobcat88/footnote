# Shared daemon throughput, before and after one lock per model (story #152, PR #171)

Measured 17 November 2026 on Johan's PC, which he confirmed idle (CPU 6 % average over 10 s before the run; no other cargo work
during it). Same bench as 16 November ([docs_mcp_load.py](docs_mcp_load.py): private copy of the React docs database, 3 warm-ups
and 30 measured calls per client), shared daemon only (`FOOTNOTE_LOAD_SCENARIOS=shared`), run back to back:

- **Before:** commit `d337ce0` (just before #171), built in its own worktree and target folder.
- **After:** `main` with #171. Both with `--features metrics,llama,alias`, the installed build's features.

| Clients | Before (req/s) | After (req/s) | Δ | p95 before | p95 after | Δ p95 |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 75 | 78 | +4.5 % | 14.9 ms | 13.9 ms | −6.7 % |
| 5 | 259 | 319 | +23.1 % | 23.9 ms | 17.5 ms | −26.8 % |
| 10 | 284 | 437 | +53.9 % | 39.1 ms | 25.1 ms | −35.8 % |
| 20 | 282 | 436 | +54.3 % | 75.3 ms | 49.3 ms | −34.5 % |
| 40 | 282 | 447 | +58.4 % | 151.8 ms | 93.6 ms | −38.3 % |

No errors in either run. "Before" reproduces the 16 November plateau (about 285 requests/s), so the runs are comparable.

**Rankings:** all 59 queries of the bench dataset, searched through both binaries on the same database copy (hybrid, 10 results),
returned identical ids and scores (59 of 59).

**What changed:** the encoder pool's lock is held only to find or load an encoder; each model has its own lock, and the keyword
search runs outside the Tantivy cache lock. No extra ONNX sessions, so no extra RAM. The new plateau (about 440 requests/s) is the
next limit: one session per model runs one inference at a time. A pool of sessions per model would raise it at about +200 MB per
extra bekko session; measure before adopting.

Raw rows: `bench/work/mcp-load-ab-20261117-{before,after}.log` (git-ignored).
