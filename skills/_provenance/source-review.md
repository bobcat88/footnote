# Footnote Capability Source Review



## Scope and interpretation

A separate reviewer read the eleven competence cards, six job cards, revised website dependency and its launch reference. This ledger distinguishes externally checkable factual clusters from local methods and delivery policies. Local choices about repeated measurements, failure tests, data boundaries and Ship/Best are explicitly policies, not externally proven universal thresholds. No numerical speedup, model-quality floor, conversion uplift or general skill benefit is asserted.

The independent pass checked the clusters in the middle column. Parent follow-up retrieved sources for the remaining specific gaps and corrected wording after that pass; these additions have not received another independent pass. Full mathematical audits of finance papers and actual product/site certification are outside this content review. Behavioral status remains experimental regardless of source coverage.

| Capability | Independent primary-source checks | Parent gap closure |
|---|---|---|
| Rust | Rust Performance Book: equivalent realistic workloads, workload-dependent metrics, timing variability, nightly built-in benchmarking, platform profilers and release debug symbols | Cargo Book command reference retrieved for bench options; no new benchmark implementation or nightly requirement introduced |
| Tauri | Official IPC, capabilities and updater guidance: message passing, window/webview permissions and update verification | Windows installer documentation retrieved for webview distribution modes; process-tree measurements remain a local method |
| React | Official Profiler overhead/build behavior; Google CWV thresholds, percentile and lab/field distinction | No remaining material numeric claim gap identified; optimization choices are conditional methods |
| Python | Python profiling and tracemalloc; Polars lazy scan/query-plan; multiprocessing startup | Polars multiprocessing and Python free-threading guidance added; no automatic parallelism benefit is promised |
| RAG | ir-measures recall/graded nDCG definitions; Ragas faithfulness against supplied context | Splits, equal comparison units, judge review, injection cases and telemetry controls are local evaluation/security policies |
| SQLite | Official EQP output instability, WAL single-writer/concurrency/network constraints, FTS5 lower-is-better bm25 convention | Relevant official sections directly rechecked; disposable-copy/parity policies are local |
| CI | GitHub lockfiles/fallback SHA pinning, token/input boundaries and cache/artifact distinction | Updated guidance to prefer applicable workflow dependency lockfiles; no unsupported lockfile format prescribed |
| Backtesting | scikit-learn chronological splitting/gap assumptions; Bailey et al. selection/holdout limitations | Added QuantConnect corporate-action/fill models, mlfinlab overlapping-label source and Lo's own published abstract on serial correlation/annualization. Lo PDF URLs failed; abstract supports only the stated caution, not a detailed formula audit |
| Scraping | Scrapy adaptive throttle and RFC 9309's non-authorization semantics | RFC 9110 Retry-After section retrieved; bounds, provenance and idempotency remain local policies |
| Nix/Hyprland | NixOS build/test/switch/recovery and readable-store risks; current Hyprland environment syntax | Historical Hyprland 0.50 variable documentation added alongside current Lua environment examples; no migration procedure is prescribed |
| Apps Script | Google batching/caching/quotas plus Lock reference | Replaced unsupported expired-lock wording with acquisition timeouts/execution termination; added lock and web-app identity documentation and flush-before-release guidance |
| Website dependency | Google CWV/LCP/long tasks; Chrome bfcache; Service Public notices; CNIL cookies; DGCCRF scope; MDN CSP; W3C contrast/target exceptions; Google sitemap limits | Added official CNIL GDPR Article 28 and Google structured-data policies. Removed prior population statistics, penalties and universal score/weight gates instead of carrying unverified claims into the pack |

All six job cards were checked as composition: selective skill loading, honest missing dependencies, no implicit agent/install behavior, authorization boundaries and pending unrun checks. They do not establish technical correctness of a particular deliverable.

## Code and export review

Independent source inspection found no additional defect in the receipt comparator within its reviewed contract. The reviewer verified all 30 manifest hashes in export v0.1.1. It found an exporter destination-policy gap: non-build project paths and agent skill-discovery roots were not refused. Parent added resolved-path refusal and negative tests for those cases. Sixteen tests currently pass across builder and comparator; fresh evidence is required after further changes.

The static scan of v0.1.1 returned five identical LP3 tool-scope leads on cards shipping the helper. The helper reads explicitly supplied, bounded JSON files and writes a comparison to stdout; it has no network or subprocess behavior. No allowed-tools preapproval was added. Runtime permission enforcement remains the host's responsibility; neither a card nor a scanner grade proves sandboxing. External UI prerequisites are declared, not automatically installed or scanned as part of this pack.

## Provenance and limitations

Sources and retrieval date appear in each card. Independent retrieval used public primary pages, occasionally recent Firecrawl cache entries. A CNIL www-host request timed out and its official non-www equivalent succeeded; the parent also retrieved the official Article 28 text from CNIL when EUR-Lex returned a bot challenge. The helper and exporter make no model or network calls and receive no credentials. No private corpus was used for these source checks.

The pilot's token and time fields were subsequently recovered from its own explicitly identified local child-session logs; API cost remains unknown. Source review and unit tests do not show that all capabilities improve project outcomes. The three-task development review pilot is the current behavioral evidence; broader implementation tasks remain future validation work.

## Sources

The comparator was subsequently tightened to reject duplicate JSON keys, including nested objects. Ten helper tests include the new regression. The change was accepted through a registered real Python suite in an isolated copy before canonical promotion. This parser correction changes no research claim or performance threshold; see CORE-REVIEW.md and evidence/code-change-acceptance.json.

Primary URLs are linked in each capability card and the website reference. Additional sources used to close independent-review gaps:

- [Cargo bench](https://doc.rust-lang.org/cargo/commands/cargo-bench.html).
- [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/).
- [Polars multiprocessing](https://docs.pola.rs/user-guide/misc/multiprocessing/) and [Python free threading](https://docs.python.org/3/howto/free-threading-python.html).
- [Lo's published abstract](https://alo.mit.edu/publications/page/18/), [QuantConnect fills](https://www.quantconnect.com/docs/v2/writing-algorithms/reality-modeling/trade-fills/key-concepts), [corporate actions](https://www.quantconnect.com/docs/v2/writing-algorithms/securities/asset-classes/us-equity/corporate-actions), [mlfinlab purging source](https://github.com/hudson-and-thames/mlfinlab/blob/master/mlfinlab/cross_validation/cross_validation.py).
- [Retry-After](https://www.rfc-editor.org/rfc/rfc9110.html#section-10.2.3), [Hyprland 0.50 variables](https://wiki.hypr.land/0.50.0/configuring/variables/), [Apps Script Lock](https://developers.google.com/apps-script/reference/lock/lock), [web-app identity](https://developers.google.com/apps-script/guides/web).
- [GDPR Article 28](https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre4), [structured-data policies](https://developers.google.com/search/docs/appearance/structured-data/sd-policies).
