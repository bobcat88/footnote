# Code graph: how competitors do it, and a proposal for Footnote

14 November 2026. Research note before any code. After the knowledge corpus and agent memory, the third product is **project graph memory**: a code graph that agents query for structure (who calls this, what breaks if it changes, how does A reach B). It is meant to replace `codebase-memory-mcp` (DeusData, MIT), today's incumbent, in the medium term.

Method: competitors' READMEs and design docs read from their repositories (no installs, no runs), two third-party benchmarks that grade call graphs against compiler output, two papers, and a read-only look at our own `codebase-memory-mcp` index. Vendor numbers are marked as such; both third-party benchmarks were also run by vendors (repowise, PipesHub) and include their own product.

## 1. What a code graph is worth to an agent

Start from the evidence, not the feature lists.

- **Finding the file to fix: text search is already enough.** PipesHub gave one model 300 SWE-bench Lite issues. With text search and file reads it found the file to fix on **89.7%**; with code-graph tools added, **90.3%** (p = 0.80, not significant), at **44% more prompt tokens and 51% more time**. On ContextBench the graph tools added 0.4 to 1.6 points, none significant. [PipesHub benchmark]
- **Inside a ranker, graph signals help a little.** Adding the symbols and paths an issue mentions to hybrid search (BM25 + embeddings) raised correct-file-first from 49.6% to 54.3% on 407 held-out SWE-bench Verified issues (p = 0.004). [PipesHub benchmark]
- **Structural and architecture questions are where it pays.** CodeGraph's benchmark (7 repositories, Claude headless, 4 runs per arm, vendor-run) asks one architecture question per repository: 88% fewer tool calls, 62% fewer tokens, 44% lower cost, zero file reads. The saving tracks how much discovery the question needs: 57 to 78% where the file-reading agent needed 28 to 43 tool calls, near-even where it needed 7. [CodeGraph README]
- **The hidden cost is context that stays.** The same vendor measured that graph answers leave about **80% more retrieval context resident** at the end of a session (VS Code: 67k tokens against 18k): one dense payload stays in the window, where many small grep results get evicted. [CodeGraph README]
- **Research agrees on the split.** SWE-Explore (848 issues, 10 languages) finds agentic explorers a clear tier above classical retrieval; file-level localization is already strong, and line-level coverage and ranking are what separate the best. [SWE-Explore] LocAgent (ACL 2025) parses a repository into a typed graph that an agent walks, reaching 92.7% file-level accuracy and +12% Pass@10 on issue resolution. [LocAgent]

**Consequence for Footnote:** do not sell the graph as a localizer. It earns its place on callers, impact, paths and architecture, and it must keep its answers short.

## 2. The five competitors, and the incumbent

Picked for what each does best, not by stars. Stars and licences read on 14 November 2026.

| Tool | Best at | How the graph is built | Storage | Stays current by | Agent surface | Licence |
|---|---|---|---|---|---|---|
| **CodeGraph** (colbymchenry, ~77.6k stars) | Speed and freshness | Tree-sitter, native Rust kernel for 20 languages, cross-file resolution | SQLite + FTS5 in `.codegraph/` | OS file watcher (debounce 2 s), staleness banner, hash reconcile on connect | 1 tool by default (`codegraph_explore`), 8 in all | MIT |
| **repowise** (~8.1k) | Edge accuracy | Tree-sitter capture, then typed resolution with 39 named origins, each with one confidence; declines to guess at the bottom | SQLite + LanceDB | Post-commit hook | 10 core tools | AGPL-3.0 |
| **GitNexus** (~48.5k) | Depth of analysis | 19-phase pipeline: parse, scope resolution, inheritance order, DI, Leiden communities, execution flows ("processes") | LadybugDB (ex-Kuzu) graph DB | Re-index; incremental groups | 19 tools incl. `impact`, `detect_changes`, raw Cypher | PolyForm Noncommercial (our previous incumbent) |
| **Serena** (~31.4k) | Compiler-grade answers and edits | No stored graph: asks live language servers (40+ languages) at query time | None | Nothing to re-index | 53 tools incl. `find_referencing_symbols`, `rename_symbol`, symbol-level edits | GPL-3.0+ since v2.0.0 |
| **Graphify** (~147k) | Code and docs in one graph | Tree-sitter for code (no model); a model pass for Markdown, PDFs, images; `# WHY:`/`# NOTE:` comments and ADR citations become nodes | `graph.json` (Neo4j optional) | Git hook or watch | 10 tools incl. `shortest_path`, `get_pr_impact` | Apache-2.0 |
| *codebase-memory-mcp* (incumbent, ~48.6k) | Breadth and build speed | 162 tree-sitter grammars in one C binary, plus LSP type resolution; indexes infra and CI files | SQLite in `~/.cache/codebase-memory-mcp/` | Shared daemon with watchers | 15 to 17 tools incl. `trace_path`, Cypher-like `query_graph` | MIT |

Others read but not shortlisted: **PipesHub** (precision-first graph in Neo4j/ArangoDB for organisations, best F1 in its own benchmark), **code-review-graph** (PR risk), **CodeGraphContext** (graph DB, weakest accuracy measured), **jCodeMunch** (paid licence), hosted **Sourcegraph** (SCIP, the compiler-grade reference format) and **Augment** (semantic retrieval, no graph).

## 3. How they do it

### Capture and resolution: where accuracy is won or lost

Every tool parses with tree-sitter or a language server. The hard part is the second step: deciding which definition a call refers to, without building the project.

Two graded runs against compiler truth:

| Benchmark | What was graded | Incumbent result | Best result |
|---|---|---|---|
| repowise, Go/TS compilers, 5 repos, 37,853 edges [repowise comparison] | Call-edge precision / recall | Highest recall on gitleaks (0.967) and syft (0.542), but precision 0.635 on syft | repowise on the precision/recall frontier in all 7 cells (precision 0.943 to 0.992) |
| PipesHub, SCIP truth, 9 repos, 4 languages [PipesHub benchmark] | F1 on calls, imports, inheritance (hand-written code) | 52.9 to 92.5; weakest on TypeScript (57.8 excalidraw, 52.9 PipesHub frontend) and guava (55.8) | PipesHub 79.0 to 94.9; leads most on deep hierarchies (guava 79.0 vs next 55.9) |

What the leaders share:
- **Every edge says how it was made.** repowise stamps one of 39 origins with a fixed confidence (0.95 same file, 0.90 through an import, 0.50 "unique name somewhere"); Graphify tags `EXTRACTED` or `INFERRED`. An agent can drop guesses below a threshold and know what it dropped.
- **A missing edge is cheaper than a wrong one.** PipesHub and repowise leave an ambiguous call out rather than guess. A miss costs the agent a search; a wrong edge sends it down the wrong path.
- **Compiler output is the answer key.** Both benchmarks build their truth with SCIP indexers (`scip-python`, `scip-typescript`, `scip-go`, `scip-java`). jCodeMunch and CodeGraphContext can import SCIP directly.

### Storage

SQLite is enough for one developer: CodeGraph, repowise, code-review-graph and the incumbent all use it. Graph databases (LadybugDB, Neo4j) buy Cypher and organisation-wide sharing, not accuracy. Build cost on Home Assistant (29,195 files): incumbent 110 s at 5 GB peak memory, PipesHub 271 s, Graphify 1,880 s at 10 GB. [PipesHub benchmark] CodeGraph reports the Swift compiler repository (27k files) in about 100 s and the Linux kernel on a 2-core VPS in under 12 minutes. [CodeGraph README, vendor]

### Staying current

- **CodeGraph's three layers** are the best design read: an OS watcher that batches bursts of edits; a warning banner in any answer that touches a file still waiting for sync, telling the agent to read it directly; and a size/mtime/content-hash reconcile when the server reconnects, which absorbs edits made while no agent ran. A one-file save syncs in about 0.3 s on a 4,400-file project. [CodeGraph README, vendor]
- **Parity must be tested, not assumed.** After seven scripted changes (rename across 15 files, deletes, moves, re-exports), PipesHub's and the incumbent's updated graphs matched their fresh builds exactly; Graphify missed up to 29 edges and CodeGraphContext up to 474. Parity is not accuracy: the incumbent's updated graph scored 80.9 F1 against truth. [PipesHub benchmark]

### Agent surface

The tool list costs prompt tokens on every turn: CodeGraph lists 1 tool, the incumbent 15 to 17, Serena 53, jCodeMunch 90+ behind a 6-tool front door. CodeGraph also found that agents delegating exploration to file-reading sub-agents bypass the graph entirely, so its instructions tell them to query it directly. [CodeGraph README]

### What sits next to the graph

- **History:** repowise mines git for ownership, hotspots and files that change together, and keeps co-change edges apart from imports.
- **Rationale:** Graphify turns `# WHY:` comments and ADR references into nodes; repowise answers `get_why`; the incumbent stores ADRs.
- **Flows:** GitNexus and the incumbent group symbols into communities (Leiden) and execution flows, which is what makes "how does X work" answerable in one call.

## 4. What our own index shows

Read-only look at `~/.cache/codebase-memory-mcp/D-Projets-GovernanceOS.db` (incumbent 0.10.8 format, 14 November 2026):

| | Value |
|---|---|
| GovernanceOS tracked files | 11,932, of which **276 code files (4.2 MB)**; the rest is PNG evidence, JSON, logs, SQLite fixtures |
| Index file | **3.69 GB** (56,333 pages of 64 KB) |
| `lsp_surface` table | **3,407.6 MB** of payload in 3,168 rows: the language-server layer, cached per file |
| Graph proper | nodes 24.4 MB + edges 7.0 MB + vectors 7.3 MB |
| Nodes by source | **50,978 of 67,815 from `.json` files**; 549 from `.rs`, 376 `.ts`, 241 `.py`, 7,166 `.js` |
| Leftovers | 11.2 GB of orphaned `.stage` databases from interrupted runs, deleted the same day; 0.12.0 now reclaims them |
| Processes | One thin MCP frontend per agent session (13 seen at once, 10 from Hermes profiles) sharing one daemon |

So **99% of the disk is a cached LSP layer, and three quarters of the graph describes data files, not code**. The code graph itself is about 31 MB. This is the clearest place to do better.

## 5. Proposal for Footnote

**Scope: code, not artifacts.** Index source files and build/infra manifests only, honour `.gitignore`, skip generated and vendored code (PipesHub's rule). JSON data, logs, screenshots and evidence go to Footnote's document corpus if wanted, never into the graph. Set a disk budget (proposal: graph ≤ 10× the code bytes) and report when a project exceeds it instead of growing silently.

**Two-speed resolution: a fast labelled pass, then compiler truth where it is cheap.**
1. A tree-sitter pass in-process (Rust crates, parallel per file), with repowise-style origins and fixed confidences, declining below a threshold. Usable within seconds.
2. A precise pass in the background when the toolchain is present: import SCIP indexes from `rust-analyzer scip`, `scip-typescript` and `scip-python` (Johan's three main languages) as edges with origin `compiler`, replacing guesses on the same call sites. This gets compiler precision without Serena's per-query language-server latency and without persisting a language-server surface (the incumbent's 3.4 GB).

**Freshness: copy CodeGraph's three layers onto Footnote's existing daemon.** Debounced OS watcher, per-file stale banner in answers, content-hash reconcile on connect (Footnote already hashes sources with `sha2`). Build every rebuild into a temporary file in the same folder and rename it into place; sweep orphans at start-up. Prove parity with PipesHub's seven-change protocol in CI.

**Storage: the stack Footnote already has.** One SQLite file per project (`rusqlite`), symbol names in the existing Tantivy index, no graph database server. Short write transactions so WAL files stay small.

**Surface: two tools, short answers.**
- `footnote_code`: a symbol, file or question in; signatures, callers and callees with confidence, line ranges and execution flows out. Bodies only on request, to limit resident context.
- `footnote_impact`: a symbol or a git diff in; blast radius with confidence and the tests it reaches.
One shared daemon for every agent (already how Footnote runs), with a thin MCP frontend.

**The part nobody else has: one store for code, documents and memory.** Footnote already holds the document corpus and agent memory with citations. Link them to the graph: ADRs, design notes and `# WHY:` comments attached to the symbols they name; agent-memory fixes and decisions attached to the files they concern; git co-change and ownership kept as a separate edge family (repowise). Then "why is this built this way" and "what did we decide about this module" are answered with cited sources, next to who calls it.

**Speed targets to beat, before any claim:**

| Measure | Bar to beat | Source of the bar |
|---|---|---|
| Fresh index, Home Assistant (29k files) | 110 s, 5 GB peak | incumbent, PipesHub run |
| Fresh index, Footnote's own repo and GovernanceOS code | under 5 s | proposal |
| One-file sync | 0.3 s on 4,400 files | CodeGraph, vendor |
| Disk | ≤ 10× code bytes (GovernanceOS today: about 880× its 4.2 MB of code) | proposal |
| Edge F1 against SCIP | 52.9 to 92.5 incumbent; 79.0 to 94.9 best | PipesHub run |

## 6. Measurement before claiming anything

Same discipline as the retrieval work: final-task results with frozen protocols, retrieval-style metrics only as diagnostics.

1. **Edge accuracy against SCIP** on PipesHub's public set: its harness is MIT ([codegraph-bench](https://github.com/pipeshub-ai/codegraph-bench)), so Footnote, the incumbent 0.12.1 and CodeGraph can be scored by the same scorer.
2. **The same on Johan's repositories** (BorgKnowledge in Rust, GovernanceOS in Rust and TypeScript, AlphaBar), with SCIP truth built locally.
3. **Freshness parity** over the seven-change protocol, plus a kill-during-sync test.
4. **Cost:** fresh build time, one-file sync time, peak memory, disk, per repository.
5. **Agent end-task:** a frozen set of structural questions (callers, impact, path, architecture, "why"), with and without the graph, scored on correct and cited final answers; tokens processed **and** tokens resident at session end. A localization control set where no gain is expected, to catch overhead.

## 7. Open decisions for Johan

1. **Languages with the precise pass:** Rust, TypeScript and Python first, everything else tree-sitter only?
2. **Running local toolchains:** may Footnote run `rust-analyzer scip` / `scip-typescript` / `scip-python` in the background? It costs CPU and needs the project to build.
3. **Default scope:** code and manifests only, with artifacts routed to the document corpus?
4. **Switch-over rule:** replace `codebase-memory-mcp` once Footnote matches or beats it on section 6 items 1 to 3 on Johan's repositories, then move the MCP wiring.

## 8. Positioning (Johan, 14 November 2026)

The differentiator is the **decision layer**: competitors ship code graphs, document search and agent memory as separate tools that do not connect. Footnote links them, with accountability as the organising idea (Johan's PMO background): who decided what, when, on which source, and which code it governs. Every answer is backed by a documented decision and a hashed source.

Target: EU organisations that must show conformity. Where this fits, stated carefully:
- **GDPR accountability (Art. 5(2))** asks a controller to be able to demonstrate compliance; a cited, hash-checked trail from decision to code is that kind of evidence.
- **AI Act:** providers of high-risk systems must keep technical documentation (Art. 11) and logs (Art. 12). Since the Digital Omnibus (in force 27 July 2026), Annex III high-risk obligations apply from **2 December 2027** and Annex I from **2 August 2028**; Article 50 transparency applied from 2 August 2026. Most use of a coding agent is not itself high-risk: the strongest fit is teams *building* high-risk systems, and regulated sectors (banking under DORA, NIS2 entities).
- **Beyond compliance: knowledge that survives turnover.** The decision layer is an **opt-in feature per project**, useful to any team, small or large, that wants to know why something was built the way it is. Teams lose that knowledge whenever people leave. Once enabled in a project, the harness captures decisions as work happens and serves them to whoever comes next, so knowledge retention is enforced by the tooling instead of depending on someone remembering to write it down. Compliance is the strongest paid case; retention is the broad one.
- **Limits to keep honest:** Footnote produces evidence, it does not make anyone compliant. A SHA-256 proves a source has not changed since it was hashed, not when it was hashed; a legally stronger date would need a trusted timestamp (eIDAS qualified timestamping), not built today.

## Sources

- [PipesHub benchmark] "How Accurate Is Your Code Graph? A Benchmark of 4 Tools", pipeshub.com/blog/code-graph-benchmark; harness github.com/pipeshub-ai/codegraph-bench. Incumbent 0.12.0 for accuracy, 0.11.0 for scale.
- [repowise comparison] "Best Code Graph MCP Servers for Coding Agents (2026)", repowise.dev, published 29 October 2026; graph design in repowise `docs/layers/GRAPH.md`.
- [CodeGraph README] github.com/colbymchenry/codegraph, README benchmark and sync sections (re-measured 2026-08-05).
- GitNexus `README.md` and `ARCHITECTURE.md`, github.com/abhigyanpatwari/GitNexus.
- Serena `README.md`, github.com/oraios/serena. Graphify `README.md`, github.com/Graphify-Labs/graphify.
- [SWE-Explore] Zhang et al., arXiv 2606.07297 (abstract read).
- [LocAgent] Chen et al., ACL 2025, arXiv 2503.09089 (abstract read).
- Local: `D-Projets-GovernanceOS.db` read through SQLite in read-only mode; GovernanceOS file counts from `git ls-files`.

## 9. Johan's answers (16 November 2026)

1. **Languages:** at least JavaScript, TypeScript, Java, Rust and Python with the precise pass (scip-typescript covers
   JavaScript and TypeScript, scip-java, rust-analyzer, scip-python); CSS as tree-sitter edges between selectors and the markup
   or components that use them. COBOL (helping banks refactor legacy systems) is a study item: parsers and grammars to check.
2. **Background toolchains:** yes, Footnote may run the SCIP indexers.
3. **Scope, with an addition:** the graph maps the code and, separately, what it reads and writes (files, configuration,
   environment variables, database tables, HTTP routes) as input/output nodes linked to the code that uses them, shown on
   demand ("show inputs/outputs", split view). Artifact contents stay in the document corpus; only their names and links enter
   the graph. Static detection of paths is a guess when paths are built at run time: such edges carry a confidence label.
4. **Switch-over:** replace codebase-memory-mcp when Footnote is at least as good on the measured items.
5. **Hands-on test:** go, on BorgKnowledge, BorgInvestor and GovernanceOS (mostly Rust), plus PilotageProjet (JavaScript/HTML)
   for JavaScript coverage; tools run locally only, telemetry off (PilotageProjet is earlier work for his employer).
6. **Design session:** after the technical tests.

How impact is measured (Johan asked how an impact tool can be judged): three answer keys, from strictest to broadest.
- **Mutation:** break one function on purpose, run the tests; the tests that fail are the true impact on tests. Compare with
  the tests the tool predicted (precision and recall).
- **Compiler truth:** the transitive callers from SCIP are the exact static blast radius of a symbol.
- **History:** for past commits, the files changed together and the tests that broke in CI.

## 10. Stage 1 built: the precise pass for Rust (16 November 2026)

Johan asked for a base. Built first: the part every later stage needs, and the answer key the hands-on comparison
([CODE-GRAPH-STUDY.md](research/CODE-GRAPH-STUDY.md), part 2) needs anyway.

- `footnote code-index [root]` runs `rust-analyzer scip` (shipped with the Rust toolchain), reads the SCIP index with a small
  hand-written protobuf reader (`crates/code/src/scip.rs`, no new dependency) and writes one SQLite graph per project in the
  state folder (never inside the repository), built beside the old one and renamed into place.
- Stored: files with their SHA-256, definitions (name, kind, signature, line range), references with the item whose body
  holds them (the caller) and `origin = compiler`. Local variables and references to code outside the project are left out.
- `footnote code <name> [--root]`: each definition with that name, its signature and lines, its callers (with call lines),
  its callees in the project, and `stale`: files of the answer changed since indexing (the agent should read them directly).
- Opt-in build feature `code`, not released; CLI only, no MCP tool yet (design: at most two, `footnote_code` and
  `footnote_impact`).

First measures on BorgKnowledge (52 Rust files, 523 KB of code): 857 definitions, 2,452 references; index in 4.3 s warm
(10.5 s cold, almost all of it rust-analyzer), graph 412 KB (0.8 times the code; target at most 10 times), a lookup in 58 ms
including process start. Callers of `store::lean` matched a manual grep exactly (5 references, 4 calling functions).

Not built yet, in order: the MCP tool; TypeScript, JavaScript, Python and Java through their SCIP indexers; the tree-sitter pass
(fast, labelled guesses) for files and languages with no compiler index; inputs and outputs (files, configuration, environment
variables, tables, routes); freshness by watcher; then the comparison against codebase-memory-mcp on the measured items.

## 11. Architecture map (Johan, 16 November 2026; research, nothing built)

Johan wants two views behind one toggle: the **useful view**, an architecture map (blocks of code, how much each depends on the
others, the outside services it calls, as in repowise's map), and the **cool view**, the whole-graph "hairball" as ambient motion.
Method (Johan): study how 5 to 10 projects do it, take ideas, write our own in Rust, bench ours against theirs; no copied code.

**How open projects build it (read 16 Nov from their READMEs; none run):**

| Project | Licence | How it groups and draws | Idea for us |
|---|---|---|---|
| dependency-cruiser (JS/TS, 7.4k stars) | MIT | Module imports, then "collapse" to chosen folders (depth or pattern) for an architecture-level graph | Deterministic grouping by folder or package: no guessing |
| Emerge (12 languages, 1.2k) | MIT | Module graph, coloured by Louvain modularity (communities found in the graph), with fan-in/fan-out | Communities as an alternative grouping when folders say little |
| CodeBoarding (2.7k) | MIT | Static analysis for relationships; a language model names and describes the components; nested diagrams | Naming blocks is the hard part; their way sends code excerpts to a model provider |
| arkit (JS/TS, 1k) | MIT | Groups modules into components from configured patterns, outputs SVG or PlantUML | Grouping rules the user can edit |
| repowise | AGPL-3.0 | Layer cards with import counts, scores and outside services (ideas only) | The look Johan liked; its scores' method is unknown, so not copied |

**What Footnote already has for it:** every SCIP symbol names its package (`<indexer> <manager> <package> <version> <path>`), so
references from project code to outside packages (OpenAI, httpx, serde...) are already in the index Footnote reads; today
`write()` keeps only references to project symbols. The map needs: (1) keep references to outside packages, counted per package;
(2) roll symbol references up to the folder, crate or package level, with counts on each link; (3) group by folder first
(dependency-cruiser's way), communities as an option; (4) a small curated table classing known packages (model provider, network,
database) for the dashed outside-service nodes; (5) block names from folders by default; a local model may suggest names, never
a cloud one by default, since naming sends code excerpts.

**Languages:** the precise pass covers languages with a SCIP indexer (Rust today; Python, TypeScript and JavaScript, Go and Java
have the indexers used in the benchmarks above). A module-level map needs only imports, which a parser gets for many more
languages (Emerge does 12), so the map can cover more languages than the call graph.

**The cool view** costs little: the same nodes and links drawn with a force layout, no labels, slow motion, in mini mode.
