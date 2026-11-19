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

## 11. Architecture map (Johan, 16 November 2026; engine side built 17 November)

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

**Engine side built (17 November 2026, overnight item 2).** `footnote code-map [--root .] [--depth 2]` reads the same compiler
index: blocks are the first `depth` folders of each file (`crates/code`, `src`), with file and symbol counts; links between
blocks are counted in references to items (bare paths to a crate or module are left out, so `a::base()` counts once); outside
packages come from the package field of each SCIP symbol, counted per package and version, with the blocks that use them and a
class from a short hand-written list (database, network, model, standard library). On this repository it gives 8 blocks,
`src → crates/core` (67 references) and `src → crates/search` (59) as the heaviest links, and serde_json, anyhow, rusqlite and
tantivy as the most used outside packages. The standard library does not appear: rust-analyzer emits no `std` symbols here.
Graphs built before this need `footnote code-index` again (the map says so). Not built: clustering by links instead of folders
(Emerge's Louvain idea), names for blocks, the launcher view (overnight item 6), the benchmark against the projects above.

## 12. Second pass: a parser for every language, compiler indexers only as an upgrade (Johan, 17 November 2026)

**Why.** Two of the four SCIP indexers broke on Windows: scip-python on one bad regex (worked around in #181), and scip-java on at
least five open upstream bugs (scip-code/scip-java #991, #992, #1012, #1066, #1067, #1068; #179). The SCIP indexers are built for
Sourcegraph's Linux servers. Each language adds an outside tool, a toolchain (Node, JDK and Maven) and often the project's own
build. Most smaller languages have no SCIP indexer at all (PHP, Swift, Dart, Lua, COBOL), and the ones that exist are Linux and
macOS first (scip-clang, scip-ruby). Betting the graph on them means a new dependency, a new security review and a new chance to
break with every language.

**Rejected: running the indexers in a Linux VM** (WSL, Docker, microVMs). It would need admin rights or a paid licence, a second
copy of the toolchain inside the VM, and hundreds of MB to call a tool smaller than the VM. It still does nothing for languages
without an indexer. WSL may stay an opt-in extra for people who already have it.

**Decided.**
1. **Base layer, every language: tree-sitter inside the Footnote binary** (#111, moved up from "later"). Definitions,
   references, calls and imports come from the parse tree, resolved by scope and name. The same code runs on Windows, Linux and
   macOS, with no outside tool and no build run. A project always gets a graph.
2. **Upgrade layer: precise indexers where they run natively.** rust-analyzer, scip-typescript and scip-python (with the #181
   fix) replace the base layer's links when they are installed and succeed. When one is missing or fails, Footnote keeps the
   base graph and says which upgrade is missing, instead of returning an error.
3. **Precision is measured, not assumed.** The base layer's references and callers are scored against the compiler-graded
   graphs from the upgrade layer on Rust, TypeScript and Python (#111's acceptance test), on this repository and on public
   sample projects. That number decides whether Java and the smaller languages need anything more.
4. **Later, if the number calls for it:** language servers that editors use on Windows (jdtls, pyright, gopls) as a middle
   tier; WSL as an opt-in route for scip-java.

The 16 November decision (section 9, a precise pass for JavaScript, TypeScript, Java, Rust and Python) still holds, now as the
upgrade layer rather than the floor.

**Base layer built (17 November 2026).** `crates/code/src/ts.rs`: tree-sitter 0.27.1 with the official grammars for Rust,
TypeScript/TSX, JavaScript, Python, Java and Go, using each grammar's own tags query for definitions and calls plus small import
queries. It reads the files git does not ignore. A call links to a same-file definition, through the file's imports (project
module, or an outside package counted for the map), through `Type::f`, or to a name defined once in the project. It never links a
method on a value of unknown type, a name the caller binds itself, or a Rust macro. `code-index` uses it whenever the precise
indexer is missing or fails, and says how to upgrade. Measured on this repository against rust-analyzer (66 Rust files, edges as
reference file and line to target name and file): **precision 0.79, recall 0.38**, in 0.6 s against about 12 s. Precision is a
lower bound: some extra links are real calls in feature-gated code the compiler did not index. Recall is low by design, since
calls only (no type references, no functions passed as values) and no guessing. TypeScript and Python are not measured on a large
project yet.

**Against the incumbent, same files and same scoring (17 November 2026).** codebase-memory-mcp 0.12.1 (tree-sitter plus its own
resolution), freshly indexed in `fast` mode, its `CALLS` edges exported with their line: **precision 0.52, recall 0.39** (1,424
links, 733 right). Footnote's base layer: **0.79 and 0.38** (918 links, 728 right). Same recall, about half the wrong links. Its
errors are the ones the base layer was fixed for: `x.map()` or `x.contains()` linked to a project function of that name through
its `suffix_match` and `unique_name` strategies. Scripts: `bench/experiments/code-graph/` (`compare_graphs.py` for two
Footnote graphs, `score_cbm.py` for a codebase-memory export); both keep only files in the compiler graph.

**Second round, rule by rule (17 November 2026).** Each link now names its rule (`FOOTNOTE_CODE_RULES=<file>` writes them;
`score_rules.py` scores them). On the first round, "a name defined once in the project or folder" was right 0 times in 12:
dropped. Same-file links scored 0.80 overall but 0.98 outside `tests/`. The gap is on the compiler side: our rust-analyzer graph
has no calls to helper functions inside integration tests (`fn run`, `fn tmp`), so scores leave `tests/` out until that is fixed.
The Rust tags query has no pattern for path calls (`footnote_core::hash()`), so the parser now adds one and resolves the
qualifier to a workspace crate (its `Cargo.toml` name) or a module file. `mod x;` is now a module, not a type, so it no longer
owns `Type::f()`. Tried and dropped: light type tracking (`let x = Store::open()` then `x.f()`), only 8 links at 0.50. Also added:
in a mixed repository the compiler graph is completed by the parser for every other file (here 317 Python, JavaScript and
script files beside the Rust), each link keeping its own grade.

| without `tests/` (37 Rust files) | precision | recall |
|---|---|---|
| Footnote base layer, round 1 | 0.79* | 0.38* |
| Footnote base layer, round 2 | **0.95** | **0.49** |
| codebase-memory-mcp 0.12.1 (fast and full give the same calls) | 0.58 | 0.39 |

\* round 1 was scored on all 66 files. On all 66 files, round 2 scores 0.82 and 0.50, and codebase-memory 0.50 and 0.37.
By rule, without `tests/`: module path 218 links at 0.99, same file 553 at 0.95, import 6 at 1.00, `Type::f()` 16 at 0.63
(most misses are feature- or Windows-gated code the compiler did not index).

**Three tools, clean truth (17 November 2026).** Once a repository is graphed with rust-analyzer installed, its graph also holds
parser links for other files, so every scoring script now takes only `compiler`-graded links, on the files the compiler covered.
Same Rust files, same scoring:

| | precision | recall | precision | recall |
|---|---|---|---|---|
| | without `tests/` | | all files | |
| **Footnote base layer** | **0.98** | 0.50 | **0.84** | 0.50 |
| CodeGraph 1.6.3 (colbymchenry, MIT; telemetry off) | 0.88 | **0.55** | 0.78 | **0.55** |
| codebase-memory-mcp 0.12.1 | 0.61 | 0.41 | 0.52 | 0.39 |

CodeGraph is the real competitor: a little more recall, about six times the wrong links. Not run: GitNexus (PolyForm
Noncommercial, not usable for a commercial product, so left out by Johan's rule) and Serena (it asks a language server, here
rust-analyzer itself, one symbol at a time: no exportable graph, and on Rust it would score the compiler against itself). CodeGraph
review: npm package with SLSA provenance, a bundled `node.exe` and readable JavaScript; no install scripts; telemetry to
`telemetry.getcodegraph.com`, switched off with `DO_NOT_TRACK=1` and `CODEGRAPH_TELEMETRY=0`; its `install` command writes into
agent configurations, so only `init` was run, on a `git archive` copy of the repository. Script: `score_codegraph.py`.

**What makes it more than the competition (Johan, 17 November 2026).**
- **The base layer resolves names through scopes and imports, not by bare name.** `save` in a file that imports it from
  `./store` links to `store.save`, not to every function called `save`. This is the stack-graphs idea. CodeGraph and
  codebase-memory-mcp match by name. Its precision is published against the compiler-graded graphs.
- **Every link carries its grade:** `precise` (compiler indexer) or `resolved` (scopes and imports), so an agent knows how far
  to trust it.
- **Links across Footnote are the core advantage:** a symbol connects to its documentation in Library and to past fixes and
  decisions in Memory ("last time this broke, the cause was…"). No code-graph tool has the user's memory or corpora, so this
  goes first in what we build and show.
- **Upgrades start on detection and need consent.** When a Java, TypeScript or Python project is found and its precise indexer
  is missing, Footnote offers to set it up and says what it needs (a Java developer already has the JDK and Maven or Gradle).
  It installs nothing on its own. Java's precise layer waits for the upstream Windows fixes (to be sent as PRs to scip-java).
- **The architecture map must be redrawn.** Johan's verdict on the 17 Nov version: far from the reference picture, and it reads
  as AI slop. The visual makes or breaks the feature, more than the decorative whole-graph view does. Visual reference: LikeC4's
  diagrams (nested boxes with titles and descriptions, labelled relationships, clean automatic layout, drill-down from one level
  to the next). Used as a reference only: Footnote draws its own, and takes no code unless it is reviewed and credited. The
  redraw goes through the design studio and QLoop before it ships.

**TypeScript and Python measured (18 November 2026, #216).** Scored like Rust (#214, #215) on 10 of Johan's projects and 6
held-out online ones (hono, nest, zustand; click, httpx, rich). TypeScript had no parser graph until now: its tags query must
be combined with the JavaScript one. Receiver typing was ported (class method tables, field types from annotations and
constructors, typed locals and parameters, `await`, `Optional`). On held-out code, 6 of 13,449 comparable links conflict with
the compiler; receiver typing adds 1,814 Python and 2,153 TypeScript links there with none. Analyzed precision per rule is 0.95
to 1.00, except held-out Python imports (0.79) where the compiler resolves the package to its installed copy or misses
re-exports. Details: `bench/experiments/CODE-TS-PY-PRECISION-2026-11-18.md`.

**Speed and precision against Graphify and codebase-memory-mcp (18 November 2026, #246).** Call links only, scored on the
files the compiler graph covers, classes as in #215. Each cell: indexing time / correct links / precision on analyzed lines /
conflict rate. Times are medians of 3 runs on an idle machine, scratch copies without `.git`.

| Project | Footnote parser | Graphify 0.9.112 | codebase-memory-mcp 0.12.1 |
|---|---|---|---|
| zustand (TS, 51 files) | **0.31 s** / **198** / 0.947 / 0% | 1.5 s / 56 / **0.966** / 0% | 3.8 s / 111 / 0.841 / 0% |
| hono (TS, 427) | **1.1 s** / **3,076** / 0.905 / 0.0% | 5.0 s / 834 / **0.906** / 0.2% | 4.3 s / 1,141 / 0.774 / 3.1% |
| rich (Python, 212) | **0.5 s** / **5,229** / **0.999** / 0.0% | 3.4 s / 2,861 / 0.987 / 0.5% | 4.3 s / 3,169 / 0.961 / 1.5% |
| brotli (Rust, 70) | **0.5 s** / **1,424** / **0.987** / 0.1% | 1.6 s / 1,037 / 0.871 / 0.5% | 4.5 s / 1,250 / 0.743 / 7.4% |
| BorgInvestor (mixed, 2,387 incl. build output) | **1.7 s** / **8,837** / **0.996** / 0% | 18.9 s / 7,307 / 0.984 / 0% | aborted ("indexing aborted before publication") |

The BorgInvestor parser time was 62 s before #246: each bare call scanned every definition of the project and decoded its
file again (quadratic), and outside git the walk read dot-named build copies (`.next-qloop-heavy`, 43 MB in all). The real
repository, where git applies `.gitignore`, holds 806 code files and takes 1.9 s. Its precision was measured on the full copy.
Graphify review: Apache-2.0 (Graphify-Labs/graphify, PyPI `graphifyy`), `graphify update` builds the code graph with no LLM
and no network, its only processes are `git ls-files` and `git config`, query logging is off by default; installed from wheels
into a throwaway environment and run on scratch copies only. Scripts in `bench/experiments/code-graph/`: `graphify_to_tsv.py` and `cbm_to_tsv.py` export each tool's call edges, `score_three.py` scores the three.

**Does Footnote Code cover codebase-memory-mcp's uses? (19 November 2026, #114).** Tool by tool, for the uses agents had:

| codebase-memory-mcp tool | Footnote Code | Covered |
|---|---|---|
| `search_graph` (find a symbol) | `footnote_code name` (definitions, signature, place) | yes |
| `trace_path` (who calls, what it calls) | `footnote_code name` callers and callees; `impact` for several levels with grades | yes |
| `get_code_snippet` | `footnote_code name` with `body` (#260) | yes |
| `get_file_outline` | `footnote_code outline` (#260) | yes |
| `get_architecture` | `footnote code-map` (blocks, links, outside packages) and the launcher's map | yes |
| `detect_changes` / `index_status` | `stale` in every answer; `code-index` rebuild | yes |
| `search_code` (text search) | none; agents' own Grep does it | no need |
| `query_graph` (Cypher) | none | not used by agents here |
| `manage_adr` | Footnote Memory (decisions as memories, anchored to code) | different, covered |

Measured against the compiler graph on the same five projects, Footnote's parser is faster (0.3 to 1.7 s against about 4 s), finds
more correct links (1.2 to 3 times) and conflicts far less (0 to 0.1% against 0 to 7.4%), and codebase-memory-mcp aborted on
the largest project (section above). Recommendation: switch. Removing codebase-memory-mcp from the agents' MCP configuration
(Claude Code, Codex) is Johan's step; it also drops its tool definitions from every session.
### Java receiver typing (#179, 19 November 2026)

Java calls now link through the receiver's written type (locals, parameters, fields, `this`, `new`, casts, `var`, for-each; lambda
parameters, patterns and anonymous classes abstain). Truth is javac itself (`bench/experiments/code-graph/JavacOracle.java`, the
Compiler Tree API on `src/main/java`, no build run), scored by `score_java.py` on three projects the rules were not tuned on.
Edges are file, line, method name and target file; JDK 21, parser only, no indexer.

| project (oracle edges) | before: links, precision, recall | after: links, precision, recall |
|---|---|---|
| gson (1,979) | 1,127 · 0.75 · 0.42 | 1,654 · **1.00** · **0.84** |
| jsoup (5,908) | 2,090 · 0.76 · 0.27 | 3,373 · **1.00** · **0.57** |
| commons-lang (5,024) | 5,148 · 0.72 · 0.74 | 3,823 · **1.00** · **0.76** |

| Spring Framework (core, beans, aop, expression, context: 2,057 files, 28,138) | 13,696 · 0.70 · 0.34 | 16,579 · **1.00** · **0.59** |

The Spring run first scored 0.97, not 1.00: 391 `receiver_type` links sent a `java.util.Set` or `List` field to a project class
named `Set` (spring-core's nested `Attribute.Set`). Cause: a Java import counted as the project's own when the first word of its
package (`java`) was also a folder name (`src/main/java`). Fixed by matching Java imports against the project's package folders,
with a regression test. Spring indexes in 4.7 s (2,057 files). `receiver_type` links: 774, 1,789 and 213, at 1.00, 1.00 and 0.99. Bare calls (`foo()`) now count as calls on `this`, which also
removed the wrong same-file links the old rule made to a same-named method of another class. Index time 0.5, 0.7 and 2.0 s.
Not measured: Graphify and the other tools on Java, Kotlin and Scala (no grammar yet), calls to inherited methods (recall gap in
commons-lang), and projects that need dependencies on the classpath. Still open for #179: Kotlin and Scala grammars (licence and
security review first) and the scip-java upgrade.

**Overloads by argument count (#179, 19 November 2026).** Of Spring's missed calls, 46% went to a method with overloads. A call
now picks the overload whose parameter count equals the call's argument count, only when exactly one matches; varargs methods
and equal-arity overloads (`notNull(Object, String)` against `notNull(Object, Supplier)`, Spring's commonest) still abstain.
Same javac scoring, precision stays 1.00 everywhere; recall: gson 0.84 to 0.85, jsoup 0.57 to 0.65, commons-lang 0.76 to 0.78,
Spring 0.59 to 0.63. The rest of the overload misses need argument types. Next: inherited methods, then the receivers that
go through generics.

**Inherited methods (#179, 19 November 2026).** A call on `this` (bare, or `this.m()`) now walks the class's `extends` and
`implements` chain, up to 8 levels, and links the first level where a class defines the name, only when that class has exactly
one narrowed candidate. A bare call also links to a method of an enclosing class, so outer and nested classes no longer lose it.
Same javac scoring, precision stays 1.00 everywhere; recall: gson 0.85, jsoup 0.70, commons-lang 0.79, Spring 0.71 (from 0.85,
0.65, 0.78, 0.63). Next: the same rule on Python and TypeScript against scip-python and scip-typescript, then receivers that go
through generics.

**Inherited methods in Python and TypeScript (#179, 19 November 2026).** The superclass walk from Java now also reads
`class A(B)` (Python), `extends` and `implements` (TypeScript, JavaScript) and interface `extends`. A `self.m()` or `this.m()`
that the class does not define reaches the nearest base that does, across files, and stays unlinked when a base is outside the
project. Scored on the six held-out projects of #216 (hono, nest, zustand; click, httpx, rich) against fresh scip-typescript and
scip-python graphs, with and without the rule. A class that declares the name as a field (hono's `match = match`) now stops the
walk: the first run linked 13 calls to the interface's method instead, the compiler's conflict rate rose to 0.006, and the field
check brought it back to 0.000. Final: TypeScript `receiver_type` 2,164 to 2,384 links, 2,041 to 2,197 correct, precision on
analyzed lines 0.954 to 0.950, no conflicts; Python 1,814 to 1,855 links, 1,711 to 1,752 correct, 0.953 to 0.954 (36 same-file
links became receiver links). Recall over every compiler reference in the covered files (not calls only): TypeScript 0.0969 to
0.0992 (+156), Python 0.1775 to 0.1776 (+5). Small gains: most inherited calls in these projects were already same-file links.
Next: receivers through generics.

**Confidence tiers, unresolved-call report, existing-index overlay (#298, 19 November 2026).** Full tables and the failed attempts:
`bench/experiments/CODE-TIERS-2026-11-19.md`. The default tier is byte-identical to before on every scored project (the rule tsv
of old and new binaries compared). New, all opt-in or additive:
- **Tier `guess_receiver_name`** (`FOOTNOTE_CODE_GUESSES=1` at `code-index`; own table, shown as `guessed_callers` with its rule):
  `store.save()` links to `Store.save` when the receiver is named like a project type that has the method. Precision on analyzed
  lines 0.95 Python, 0.84 TypeScript, 0.93 / 0.97 / 0.98 on gson / jsoup / commons-lang (javac); recall gain +0.10 Python, +0.05
  TypeScript, +0.03 / +0.05 / +0.06 Java. Below the default's precision, so never default.
- **Tried and removed:** a method name defined once in the project (precision 0.04 to 0.61), a bare name defined once in another
  file (0.00 to 0.65). Single-implementation interface dispatch cannot be scored against a static compiler graph.
- **`footnote code-unresolved <file|function>` / MCP `unresolved`:** the calls the graph did not link, with the project definitions
  that share the callee's name to grep, or none.
- **`FOOTNOTE_CODE_SCIP=<file>`:** read an existing SCIP index as the compiler graph. The compiler was already the top tier.
- **Miss buckets:** a method on a value of unknown type is the largest real one (51% Python, 54% TypeScript of missed calls).
  TypeScript recall is now scored without `<constructor>`/`<get>`/`<set>` references, which are not calls.
