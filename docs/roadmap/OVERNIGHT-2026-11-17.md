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
- Overview screen from the locked mock (`.design-studio/launcher/chosen/index.html`): sidebar, three product cards, projects
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

## Order and stop
1, 2, 4 and 5 are engine work that does not depend on each other; 3 depends only on the locked design. Order for the night:
1, 2, 3, 4, 5. Report in the morning: one table, item, PR, result, what was skipped and why.

## Not tonight (needs Johan)
Memory review (21 Nov), more Code languages (third-party indexers need a security review first), Alias out of the free repo
(needs the engine as a library), GitHub billing.
