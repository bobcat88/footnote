# Changelog

Notable changes to Footnote, newest first. Versions follow semantic versioning. From 16 November 2026 the Unreleased section is
generated from Conventional Commits with git-cliff (`cliff.toml`); earlier entries were written by hand.

## Unreleased

### Added

- **library:** Documentation crawler, system TLS roots, overnight benchmarks
- **search:** Structure-aware chunks, heading path, extraction fix (#19)
- **search:** Lean search output, one passage per page for docs (#20)
- **models:** Installable French model (Jina v5 nano), llama in releases (#22)
- **memory:** Recall by meaning on each prompt, on by default (#23)
- **alias:** French identifier rules with check digits, alias-scan (#25)
- **search:** Hybrid by default when the corpus models are installed (#27)
- **memory:** Tags and scope as pre-filters before ranking (#28)
- **memory:** Tag engine, tested and left off (any@3 0.91 to 0.87) (#29)
- **alias:** Name model and keyed pseudonyms inside the binary (#31)
- **models:** Install Alias's detector, every file pinned (#32)
- **code:** Stage 1, a code graph from rust-analyzer's SCIP index (#33)
- **code:** Footnote_code MCP tool for agents (#44)
- **projects:** Every project an agent opens, listed with its state (#45)
- **status:** Footnote Code in the product overview (#46)
- **code:** Include Code in the free default build (#53)
- **code:** Architecture map from the compiler index (#54)
- **launcher:** Tauri window with the Overview screen (#55)
- **usage:** Token counts and list prices from agent session files (#56)
- **launcher:** Code screens, architecture map and symbol view (#59)
- **launcher:** Projects, Models, Activity and Usage screens (#60)
- **launcher:** Mini mode, the lens window (#61)
- **memory:** Opt-in encrypted personal store (experimental) (#65)
- Per-product switches for Library and Code (#68)
- **usage:** Measured savings from RTK, kept apart from cost (#70)
- **launcher:** Product switches on the Overview cards (#72)
- Footnote find, one search across the switched-on products (#73)
- **launcher:** Search screen over Memory, Library and Code (#74)
- **launcher:** Install a model from the Models screen (#78)
- **launcher:** Live in the tray; closing a window hides it (#79)
- **search:** Reuse vectors by text across files and versions (#80)
- **memory:** Memory-review lists candidates; memory-restore undoes (#81)
- **library:** Footnote corpora lists, adds and removes corpora (#143)
- **memory:** Activity shows which memories reached which prompt (#161)
- **usage:** Measure Footnote's own lean search results (#165)
- **usage:** Count DeepSeek Harness tokens; name what is not measured (#168)
- **encryption:** Memory runs on an encrypted personal store (#177)
- **encryption:** Keep decrypted memory out of Windows crash reports (#178)
- **code:** TypeScript/JavaScript and Python projects through SCIP (#180)

### Fixed

- **library:** Crawler saves each page once (.md, HTML and trailing-slash forms)
- **search:** Plain Windows paths in results, backlog up to date (#30)
- **models:** State every default model's licence (#43)
- **daemon:** Keep an agent's tools when the shared daemon stops (#67)
- **find:** Keep test corpora out of the user's corpus list (#75)
- Keep test runs out of the real state folder; cap footnote jobs (#77)
- Write each log line in one append so parallel writers never mix (#166)
- **audit:** A starting server waits for an append instead of refusing (#169)
- **code:** Run scip-python on Windows without editing its package (#181)

### Performance

- **search:** One lock per model; keyword search outside its lock (#171)

### Documentation

- **library:** Documentation storage and crawler study; pre-registered crawl bench
- **research:** Alias desk study, remote access, launcher catalogue (#21)
- **federation:** Rights from company SSO, inside the agent session (#34)
- **roadmap:** Free and commercial editions, one engine (#35)
- **roadmap:** Free edition, private data and five linked devices (#36)
- Today's claims checked against their sources (#37)
- **roadmap:** Footnote One and Footnote Enterprise, the split (#38)
- **roadmap:** Editions review, encryption rule, licence options (#39)
- **roadmap:** Footnote One stays Apache-2.0 (#40)
- **p1:** Launcher lists Enterprise modules, muted, no nagging (#41)
- **p1:** One quiet support link in the launcher (#42)
- **backlog:** How T3 Code gets usage, cost and limits (#47)
- **design:** Lock the launcher, D1 full window with D2 mini mode (#48)
- **backlog:** Spend and savings module, design rules (#49)
- **code:** Architecture map, how open projects build it (#50)
- **roadmap:** Overnight plan for 17 November (#51)
- **roadmap:** Add five overnight goals at launch (#52)
- **bench:** Memory encryption build check, blocked on Perl (#57)
- Overnight catch-up and morning report (#62)
- **roadmap:** Fix the install folder path in the morning report (#63)
- **roadmap:** Restore the backslash path in the morning report (#64)
- **roadmap:** Plan the encrypted personal corpus (#76)
- Epics, user stories and the definition of done (#82)
- **roadmap:** Migrate issue tracking to GitHub Issues (#164)
- **encryption:** Derived-data audit of the personal store (#174)

### Tests and benchmarks

- **docs:** Fresh 40-question confirmation set and protocol, written before running; crawler keeps same-host redirect scope only
- **docs:** Confirmation verdict (worse), sealed held-out set 3 and optimisation protocol; crawler cites the HTML page for Markdown copies
- **bench:** Alias evaluation set v1 and rules, before any detector (#24)
- **bench:** Alias rules plus OpenAI Privacy Filter pass on held-out (#26)
- **bench:** Reranker abstention gate fails on dev (goal 9) (#69)
- **bench:** Bge-reranker candidate stopped before scoring (#71)
- Keep usage tests out of the user's real agent sessions (#170)
- **bench:** Shared daemon +54 to +58 % throughput, same rankings (#175)
- **hook:** Hook.log never quotes prompt or memory text (#176)

## 0.1.0-alpha.1 - 2026-11-15

Private pre-release (GitHub Releases, Windows and Linux x64, built with dist).

### Added
- `footnote status`: what is installed, on, registered and running, and what memory did this week (the launcher's overview).
- `footnote options` and `footnote config get/set`: the options catalogue, memory on/off, and a company policy file
  (`%ProgramData%\footnote\policy.json`, `/etc/footnote/policy.json`) that settings cannot override.
- `footnote uninstall --memory`.
- Model manager: `footnote models list/install/use/remove/recommend`, with hardware detection (GPU vendor, VRAM, CPU, RAM);
  downloaded models stay out of any git repository.
- Memory scope is the repository's remote name (folder name as fallback); `footnote memory-rescope`.
- Live progress of long steps (`footnote jobs`, `<state>/jobs/<id>.json`), `footnote memory-list`, `footnote activity --days`.
- Release workflow (dist): GitHub Releases for Windows and Linux x64.

### Changed
- The repository is a Cargo workspace (ADR 0001): `footnote-core`, `footnote-search`, `footnote-library` and `footnote-memory`
  under `crates/`, the `footnote` executable at the root. Library and Memory are features of the executable, on by default.
- The memory hook runs in Rust (`footnote hook`); the Python hook is gone. Per-prompt recall about 24 ms in-process.
- Embeddings are computed in-process with ONNX Runtime; no Python on the search or memory path.
- Product site and QLoop passes moved under `design/`.

### Added
- Opt-in resident daemon for recall by meaning on each prompt (`"resident": true`, off by default until its relevance cutoff passes).
- Memory visibility: agents say when a memory shapes an answer and give a one-line receipt after each save.
- Opt-in metrics build (`--features metrics`) and `footnote metrics`.
- CI on Windows and Linux: format, clippy for every product combination, tests.

### Fixed
- Audit chain fork when two MCP sessions wrote at once.
- Long accented prompts failed memory recall (512-byte query limit).
