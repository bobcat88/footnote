# Overnight plan, 16 to 17 November 2026 (Footnote One)

Written before the run, for Johan's go. Every item: its own branch and PR, tests, merge (CI is out of credits, so checks run
locally: `cargo fmt --all --check`, clippy on the four CI feature sets, `cargo test`), then install the binary (standing go).
Standing rules apply: no subagents unless Johan says so, no pushed history rewritten, no third-party install hooks, Second Brain
untouched, Arkea-Next-Desktop untouched. If an item is blocked, write why in its PR or this file and move to the next one.

## 1. Code in the free build (small)
- `code` joins the default features; `footnote_code` and `footnote code` answer "install rust-analyzer (`rustup component add
  rust-analyzer`)" when the indexer is missing, instead of a raw error.
- Done when: the default build lists 12 MCP tools, tests pass with and without rust-analyzer on PATH, README lists the commands.

## 2. Architecture map, engine side (medium; CODE-GRAPH-DESIGN.md section 11)
- Keep references to outside packages (the package is in each SCIP symbol), counted per package.
- New `footnote code-map [--root] [--depth N]`: blocks (folders or crates) with symbol counts, links between blocks with
  reference counts, outside packages with counts, classed by a short hand-written table (model provider, network, database).
- Done when: a test on a two-crate demo project gives the expected blocks, link counts and one outside package; on this
  repository the map lists `crates/code`, `crates/search`, `src` and outside packages such as `rusqlite`, `tantivy`, `serde`.

## 3. Launcher, first screen (largest; P1 M3, DESIGN.md section 11)
- `apps/launcher`: Tauri 2 + React + Tailwind, as P1-PLAN says. npm packages installed with `--ignore-scripts` and their scripts
  read first (`@tauri-apps/cli` 2.12.5 has no install script, checked 16 Nov). Fonts self-hosted.
- Overview screen from the locked mock (`.design-studio/launcher/chosen/index.html`, local only, not in the repository): sidebar, three product cards, projects
  table, agents table, activity column, Enterprise group muted, support link; data from `footnote status`, `projects`,
  `activity`, `models list` called as a child process. Mini mode and the Code views come later.
- Done when: `cargo tauri dev` opens the window on Windows with real data, QLoop UI gates G1 to G8 pass on screenshots at
  1920×1080, and the build is documented. If Tauri or WebView2 tooling is missing, stop at a building scaffold and say what is
  missing.

## 4. Spend view, phase 1: local counts (medium; BACKLOG "Spend and savings module")
- `footnote usage [--days 1|7|30]`: tokens per day, per agent and per model from Claude Code and Codex session files
  (input, output, cache read, cache write), read incrementally (new bytes only, a hash guard against rewritten files). Counts
  only: no prompt or answer text is kept.
- Cost from a hand-written price table taken from the providers' official pricing pages (date and URL in the file), so no
  third-party table licence is needed; subscription users see tokens, not money.
- No undocumented account endpoints, no reset button, no savings yet (savings need the measured filters first).
- Done when: a test on fixture transcripts gives exact totals, and the command runs on this machine's real files in under
  a few seconds on a warm run.

## 5. Personal memory encryption, feasibility and measure (medium; EDITIONS.md rule written 16 Nov)
- Build check: rusqlite with `bundled-sqlcipher-vendored-openssl` on Windows; if it does not build cleanly, stop and report.
- Measure on a copy of `memory.db` (never the live file): recall latency p50 and p95, plain against encrypted, keyword index
  kept in memory. Rule written before the measure: encryption ships for personal memory regardless; for project memory and
  Library only if encrypted search keeps at least 90 % of today's speed.
- Timing only if Johan confirms the machine is idle overnight (no games); otherwise build check only, timing waits.
- Done when: a short results note in `bench/experiments/` with the build outcome and, if measured, the numbers and the verdict.
  No product change in this item.

## Added at launch (Johan, 22:00): the night is eight hours, so more goals; what is not done carries over
Johan confirmed the machine stays idle all night (no games), so item 5's timing runs (live load sampled first).

### 6. Launcher, Code screens (medium; DESIGN.md section 11)
- Symbol view: search a symbol, callers and callees as an ego graph, file and line, from `footnote code`.
- Architecture view: blocks and links from `footnote code-map` (item 2); the hairball stays a later, optional "cool view".

### 7. Launcher, Projects, Models and Activity screens (medium)
- The sidebar's other screens from the same data commands; Models shows licence and installed state, Install stays a
  command shown to copy (no download from the UI tonight).

### 8. Launcher, mini mode (medium)
- The D2 lens look as a small always-on-top window (three lenses, last recall), toggled from the full window and back.
  Motion respects reduced-motion.

### 9. Third-party notices (small)
- A generated list of every Rust dependency and its licence (cargo-about or a script over `cargo metadata`, whichever
  passes the security review), checked in, plus a test or script that flags a dependency without a commercial-use licence.

### 10. Docs catch-up (small)
- README, CHANGELOG and BACKLOG updated for everything merged tonight; the morning report table at the end of this file.

## Order and stop
1, 2, 4 and 5 are engine work that does not depend on each other; 3 depends only on the locked design. Order for the night:
1, 2, 3, 4, 5, then 9, 6, 7, 8, 10 (10 always runs last, even if others are left). Report in the morning: one table, item, PR, result, what was skipped and why.

## Not tonight (needs Johan)
Memory review (21 Nov), more Code languages (third-party indexers need a security review first), Alias out of the free repo
(needs the engine as a library), GitHub billing.

## Morning report (17 November 2026)

Every item: its own PR, merged; checks run locally (fmt, clippy on five feature sets, full workspace tests) because CI has no
credits; the installed binary replaced after items 2, 4 and 6 (old copies renamed `footnote.exe.replaced-<time>`).

| # | Item | PR | Result |
|---|---|---|---|
| 1 | Code in the free build | [#53](https://github.com/bobcat88/footnote/pull/53) | Done. Default build has Code; a missing rust-analyzer says `rustup component add rust-analyzer` (tested with an empty PATH). |
| 2 | `footnote code-map` | [#54](https://github.com/bobcat88/footnote/pull/54) | Done. Blocks, links in references, outside packages with kinds; on this repo 8 blocks, `src → crates/core` 67 refs; serde_json, anyhow, rusqlite, tantivy on top. Old graphs need `code-index` again. |
| 3 | Launcher, Overview | [#55](https://github.com/bobcat88/footnote/pull/55) | Done. Tauri 2 + React, real data in the real window, 0 console errors; QLoop G1–G8 pass on screenshots at 1920×1080. Tailwind dropped (plain CSS from the locked mock). |
| 4 | `footnote usage` | [#56](https://github.com/bobcat88/footnote/pull/56) | Done. Claude Code and Codex files, counts only; 965 MB read in 0.86 s cold, 4 ms warm; matched an independent count. 7 days = $625.68 at API list prices. |
| 5 | Memory encryption check | [#57](https://github.com/bobcat88/footnote/pull/57) | **Blocked.** SQLCipher's vendored OpenSSL needs a native Windows Perl; none here. No timing taken. Two options written down (Strawberry Perl, or SQLite3 Multiple Ciphers without OpenSSL). |
| 9 | Third-party notices | [#58](https://github.com/bobcat88/footnote/pull/58) | Done. 622 crates + 7 web packages, all commercial-use licences; `scripts/third_party.py --check` in CI. |
| 6 | Launcher, Code screens | [#59](https://github.com/bobcat88/footnote/pull/59) | Done. Layered architecture map and symbol ego view, checked on this repo; callees no longer list bare `crate`. |
| 7 | Launcher, workspace screens | [#60](https://github.com/bobcat88/footnote/pull/60) | Done, plus a Usage screen on item 4. Projects, Models (install command shown), Activity, Usage. |
| 8 | Launcher, mini mode | [#61](https://github.com/bobcat88/footnote/pull/61) | Done. Lens window, swap both ways checked in the real app; still under reduced motion. |
| 10 | Docs catch-up | this PR | README (usage, code map, desktop window), BACKLOG, this report. CHANGELOG left to `scripts/changelog.py`: git-cliff is not installed here. |

**Needs you:** item 5's choice (install Strawberry Perl, or let me try SQLite3 Multiple Ciphers first, my pick); a support page
URL for the quiet support link; 19 old `footnote.exe.replaced-*` copies in `%LOCALAPPDATA%\Programs\footnote` you may want
deleted (one at a time, your call); restart T3 Code so the agents reconnect to the new engine.

**Not checked:** the independent QLoop critic (needs a subagent), Linux and macOS builds of the launcher, light theme (dark only
by design), screen-reader passes beyond axe.
