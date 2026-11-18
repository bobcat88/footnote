# Contributing

Thanks for looking. Footnote is small on purpose, and every claim in it has a receipt, so the rules are about evidence.

## Run the checks

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test                        # passes on a fresh checkout; tests needing models, .venv or indexers skip and say why
python scripts/setup_tests.py     # prepares all of it: .venv, the test models, and reports the code indexers on PATH
```

All three must pass. Tests that need a model are in `tests/hybrid*.rs`; they use the models under `artifacts/models/`.

## Changes that touch retrieval

Retrieval quality and speed are measured, not argued. A change to ranking, chunking, models, fusion or storage needs:

1. A row in the lab (`bench/lab/`) or a receipt from the shipped binary (`bench/run.py`), on the public datasets listed in `bench/README.md`.
2. The decision rule written down **before** the run (see `bench/experiments/PLAN.md` for the format).
3. Dev split for tuning, held-out split for confirmation, a paired bootstrap interval (`bench/compare.py`) for any claim of improvement.
4. The cost next to the gain: latency per query, memory per 1,000 chunks, build time.

A change that does not help is still welcome as a result: add its row and say so. `bench/experiments/RESULTS.md` lists 15 ideas that were tried and did not ship.

## Changes that touch security

Keep the model in the README true. New tools must be read-only over MCP; anything that writes needs an explicit CLI command. Add a test that fails without your change. See [SECURITY.md](SECURITY.md) for how to report a vulnerability.

## Pull requests

One topic per pull request, with the evidence in the description. Commits follow `type(scope): summary` (`feat`, `fix`, `perf`, `bench`, `docs`).
