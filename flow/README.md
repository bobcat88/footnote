# Flow

Flow is the governed delivery loop around Footnote: a frozen project plan, dependent specialist attempts, deterministic
verification, optional review and memory admission (`orchestration/`), the bridge to the Footnote engine
(`integration/`), the capability-pack builder (`capabilities/`) and the benchmark harness (`evaluation/`).
Python 3.12, standard library only (the pack builder also uses PyYAML, `capabilities/requirements-toolkit.txt`).

Moved from the notes vault (`AI-Agents/projects/footnote-framework`) on 2026-11-18 (#230). The vault folder stays as
the historical copy. The documents here were written there: their `[[wiki links]]` and many `build/...` and
`evidence/...` citations point at that copy.

## Layout

| Path | What |
|---|---|
| `../skills/` | The curated skills pack, unchanged: `pack-manifest.json` (hash of every file), `skills/<name>/` one folder per skill, `_shared/`, `_provenance/` |
| `results/` | Recorded results the docs cite (Sonnet baselines, matrix launch manifest and review, the L2 pilot result) |
| `evidence/` | Only the inputs the code reads: the code-change fixture, the L1 known-valid workspace, the L1 pilot results |
| `build/` | Every run output (demos, pilots, matrix cells). Ignored by git |

The pilots copy the Footnote engine into each sandbox: `target/release/footnote.exe` by default, or `FOOTNOTE_BIN`.

## Checks

From this folder:

```powershell
rtk proxy python -B -m unittest orchestration.test_governor orchestration.test_verifiers integration.test_footnote evaluation.test_measurement_adapter evaluation.test_native_receipts evaluation.test_receipts
rtk proxy python -B -m unittest discover -s capabilities -p "test_*.py"
rtk proxy python -B -m unittest discover -s capabilities/scripts -p "test_*.py"
```

`evaluation.prepare_matrix` also compares L1/L2 with the recorded pilot folders; copy them from the vault's
`build/luna-medium-development-v1` and `build/luna-medium-dependency-v1` into `build/` first.
