# Token economy layer: research and plan

18 November 2026. Johan asked for a layer that saves input and output tokens, especially on code work, in the spirit of RTK
and Ponytail: study how the existing tools save tokens, keep their best ideas, and build Footnote's own in Rust. Per the
borrowing rule (16 Nov), ideas only, no code copied, and every claim benchmarked against the originals. Tracked under\nepic E9 Usage and savings (#91).

## 1. What the tools do

| Tool | Side | Mechanism | Their number | Independent number |
|---|---|---|---|---|
| **RTK** 1.2 (Rust; CLI Apache-2.0, engine and proxy BUSL-1.1) | input | A PreToolUse hook rewrites shell commands to `rtk <cmd>`; 100+ per-command filters: test runners keep failures and assertions, git status and log become one line per item, diffs lose context, grep and linters are grouped by file or rule, long lines are cut. A filter ships only if it cuts 20%. Never prints more than the raw command; a filter that fails passes the raw output and the exit code through. Truncated output names where the full copy is (`rtk recall`). `rtk gain` and `rtk discover` report savings and missed commands, with tokens estimated as bytes / 4 | 60 to 90% of covered command output | **-2.7% billed cost**, CI [-5.6, -0.1], success unchanged (arXiv 2607.12161, 712 runs per arm). Only touches Bash: Read, Grep and Glob pass untouched |
| **RTK-ML** (the paper's ML retrieval arm on top of RTK) | input | Ranks and drops lines of search output | -38.4% tool-output tokens | **+6.8% billed cost**: first edit 0.94 turns later, more re-retrieval and repeated searches |
| **Headroom** (Apache-2.0, Python) | input | Type-aware compressors: JSON arrays reduced to representative rows keeping errors and boundaries, AST compression of code keeping signatures, a small local model for prose. **Reversible**: originals hashed into local SQLite, the model calls `headroom_retrieve` (CCR). **CacheAligner** keeps prompt prefixes stable so provider caches hit | 60 to 95% (JSON), 21 to 57% (benchmark) | **+48.4% billed cost** in every task family, cache-read context 49% higher at the first edit |
| **context-mode** (MCP server) | input | Runs commands and code in a sandboxed subprocess; only what the script prints enters context. Large outputs are indexed in SQLite FTS5 and searched (BM25) instead of dumped. Snapshots session state for compaction | 315 KB to 5.4 KB on its fixtures | none found |
| **sqz** (Rust) / **llmtrim** | input | Deterministic per-command formatters, byte-exact `expand`; never caches output with credentials, stack traces or migrations. llmtrim reverts any step that does not save | 24.7% over 3,003 real compressions (sqz) | none found |
| **TOON** | input | Tabular notation for uniform JSON | -40% tokens | Accuracy parity or small losses; worse than compact JSON on nested data |
| **Caveman** (MIT) | output | Skill that makes replies telegraphic (lite, full, ultra), keeps code, paths and errors exact; `caveman-compress` rewrites CLAUDE.md and memory files tersely, keeping a backup | -65% reply tokens, -46% memory-file tokens | Output is 0.6 to 2.5% of a session's billed tokens; the skill adds about 1 to 1.5k input tokens per turn |
| **Ponytail 5** (installed here) | output | A "does this need to exist" ladder before writing code: reuse, standard library, one line, then minimum code; never cuts validation, security or accessibility; shortcuts are logged as debt | 39 tasks x 5 runs, Opus 5.5: lines -53%, output tokens -45%, **cost -26%**, time -41%, hidden checks 97% vs 96% | none found, but it is the only measured win on billed cost |
| **Graphify / code graphs** | input | Query a graph instead of re-reading and grepping files | not quantified | none |

## 2. What the evidence says

1. **Token counts are not cost.** In tool-heavy agent runs, cache creation and reads are about 87% of the bill, generated
   output about 11%, uncached input about 1% (arXiv 2607.12161). 76 to 85% of cache reads is framework overhead (system prompt,
   tool schemas, CLAUDE.md, skills) that no tool-output filter touches.
2. **Removing evidence the model needs costs more than it saves.** Dropped lines come back as re-reads, extra searches and
   later edits. Aggressive compression cut patch application from 27/40 to 15/40; byte-exact windows around edit anchors brought
   it back to 83%. A pytest rule that saved 31.9% kept only 56.1% of failure markers.
3. **Never rewrite output another program reads.** All four real failures in the paper's routing arm came from a shell pipe
   parsing rewritten search output.
4. **Writing less code is the measured win.** Ponytail's -26% cost comes from the agent doing less, not from compressing text.
5. **Cost per accepted task is the only honest metric** (and Johan's own rule for the Flow benchmark, 18 Nov): paired tasks,
   billed cost split into cache write, cache read, uncached input and output, turns, success, with confidence intervals.

6. **Our own pilots agree: the saving is in calls, not in text** (Footnote Flow complexity ladder, Luna medium, one run
   per arm, RTK, Ponytail and the global configuration on in every arm, so the gaps are what the harness adds on top of them).
   L1: Footnote plus skills took 4 model calls against 9 for baseline, 31 s against 51 s, 143k tokens against 319k. L2: Flow
   was the fastest accepted delivery (48 s, 305k tokens) and baseline stopped incomplete. Uncached input was nearly the same in
   every arm (38k to 41k); the totals moved with the number of calls, each of which resends the cached context. Sources:
   `LUNA-PILOT.md` and `DEPENDENCY-PILOT.md` in the footnote-framework workspace.
## 3. What Footnote builds

Working name: **Footnote Lean** (name to confirm). One Rust layer in the existing binary and hooks, in five parts, ordered by
measured payoff, each shipped only after it beats the baseline on cost per accepted task.

### A. Measure the bill first (`footnote gain`)
- Read Claude Code and Codex session transcripts (they log cache creation, cache read, input and output per call) and split
  each session's real cost into the four buckets, per turn and per tool. Extends `footnote usage` (#223).
- Measure the fixed per-turn overhead: system prompt, tool schemas, CLAUDE.md, skills, and Footnote's own hook injections.
- Log each Lean action (bytes in, bytes out, retrieved later or not). An output that is retrieved after being shortened
  counts as a miss.
- Ledger only, never an invented per-repo saving (Ponytail's honesty rule).

### B. Write less (output side, the measured win)
- **Lean code rules**: Footnote's own ladder in the Flow skills (reuse, platform, one line, minimum code, debt comments),
  with the cuts it never makes. Benchmarked against Ponytail on the same tasks.
- **Lean prose levels** (lite, full, ultra), off for anything a human keeps (docs, commits, PR bodies, user-facing text). No
  recaps of tool output, no restating the question.
- **Edits, not rewrites**: diff-sized edits and no reprinting of files the user can open.
- **Routing**: mechanical work (issue creation, bulk edits) to a cheap model after a strong one has structured it (Johan, 17
  Nov); effort set per task class.

### C. Read less (the part only Footnote can do)
- **Graph first**: a symbol's body, callers and callees from Footnote Code instead of whole-file reads and grep loops; the
  outline of a large file before any slice. Measured as fewer read and search tokens and fewer turns before the first edit.
- **Memory first**: past fixes and decisions from Footnote Memory before rediscovery ("last time this broke, the cause
  was..."). Measured as fewer exploratory turns on repeated work.
- **Smaller fixed overhead**: compress always-loaded files (caveman-compress idea) and keep Footnote's own injections under a
  budget: no memory injected twice in a session, nothing below a relevance threshold, a compact tool list.
- **Cache-stable injections** (CacheAligner idea): deterministic, sorted, no timestamps, appended after the cached prefix,
  so Footnote never breaks the provider cache.

### D. Shrink shell output (RTK-class, preservation first)
- `footnote run -- <cmd>` behind a PreToolUse hook, filters per command family (tests, build, git, search, linters,
  package managers), written in Rust.
- Rules taken from the evidence: never more than the raw output; raw output and exit code on any filter error; never touch
  output that feeds a pipe or a redirect; keep tracebacks, failing assertions, error lines and byte-exact edit anchors whole;
  each filter ships only with marker recall of at least 99% on a test corpus and at least 20% saving.
- **Reversible**: the full output goes to a local store in Footnote's private state folder, shortened output names it, and
  `footnote output <id> [--grep pattern]` returns a slice (context-mode's search instead of dump). Outputs that contain
  credentials are never stored (sqz rule); the store expires.
- Code-aware extras no shell filter has: a failing test or compiler error is linked to its symbol in the code graph, so the
  agent jumps to the function instead of opening files.

### E. Prove it, and learn when to switch it on
- Arms on the Flow benchmark (#222 to #227): bare, RTK, Caveman, Ponytail, Footnote Lean, and stacked, on paired tasks,
  reporting cost per accepted delivery with the four-bucket split, turns, re-reads and time to first edit.
- Activation proof per arm (the hook really ran), and a per-layer share (#227).
- Run on the existing complexity ladder (#225: four levels, reasoning medium and high, three model tiers) rather than a
  new benchmark. Its output is an activation table: for each task level and reasoning effort, which layers pay for
  themselves. A local change may not need the harness; continuity and cross-module work may. No routing is built before
  that table exists (MODEL-STUDY rule).
- The headline number is turns and calls per accepted delivery next to cost: the pilots show the harness saves by cutting
  calls.

## 4. Not building, and why
- **An LLM-based compressor or an API proxy** (Headroom's Kompress, RTK's proxy). Headroom's arm cost 48% more; a proxy
  sits on every request, adds latency and is a security surface; RTK's proxy is BUSL.
- **TOON-style formats for agent input** until a measured gain on our own outputs: independent tests show accuracy parity or
  losses.
- **Re-implementing RTK's 100 filters wholesale.** RTK already works here and saved 2.7%; part D starts with the command
  families where Footnote can add something (tests, builds, search linked to the code graph) and adds others only when the
  ledger shows their volume.

## 5. Order
1. A (measure) on Johan's real sessions: where the money goes decides the order of B, C and D.
2. B (write less): cheap, already measured by Ponytail.
3. C (read less): Footnote's unique advantage.
4. D (shell output): only the families the ledger shows are large.
5. E (head-to-head benchmark) for each part before it is on by default.

## Sources
- arXiv 2607.12161, "Token Reduction Is Not Cost Reduction" (RTK v0.44.1, RTK-ML, Headroom v0.27.0, 712 runs per arm).
- github.com/rtk-ai/rtk README; Ponytail 5 benchmark (installed plugin, `ponytail-gain`); Caveman and caveman-compress
  (JuliusBrussee/caveman); Headroom (chopratejas/headroom); context-mode (mksglu/context-mode); sqz (PyPI), llmtrim; TOON
  (toon-format/toon) and arXiv 2603.03306; tomron.net 6 Jul 2026 and besthub.dev comparisons.
