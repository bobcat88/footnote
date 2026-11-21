---
skill_hash: 53abb2ce1be701cc
reviewed: 2026-11-21
---
# run-benchmarks: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| Bootstrap confidence intervals | [Efron and Tibshirani, An Introduction to the Bootstrap](https://doi.org/10.1201/9780429246593) |
| Paired significance tests for IR | [Smucker, Allan, Carterette: A comparison of statistical significance tests for IR (CIKM 2007)](https://doi.org/10.1145/1321440.1321528) |
| Multiple comparisons inflate false positives | [NIST/SEMATECH e-Handbook: multiple comparisons](https://www.itl.nist.gov/div898/handbook/prc/section4/prc47.htm) |
| Pre-registration of analysis plans | [Center for Open Science: preregistration](https://www.cos.io/initiatives/prereg) |

## Field evidence
- This repository's bench practice: dev and held-out splits, sealed single-use sets with committed hashes, paired bootstrap with 10,000 resamples and a fixed seed, results index with status (`bench/INDEX.md`).
