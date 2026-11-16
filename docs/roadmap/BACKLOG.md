# Footnote backlog

Last updated 16 November 2026. One prioritised list; Beads stays the authority for tracked tasks. Order inside a tier is the order
of work. "Johan" marks items waiting for his go.

Every item belongs to the free or the commercial edition ([EDITIONS.md](EDITIONS.md), decided 16 Nov): company-only features
(sign-on, rights, hierarchy, many team indexes, compliance) go into a separate commercial crate, never into the free engine.

## Recently done

- Test 5 (llama.cpp models, [rule and results](../../bench/experiments/LLAMACPP-MODELS.md)): no model adopted through an HTTP server.
- Test 6 ([results](../../bench/experiments/LLAMACPP-INPROCESS.md)): Jina v5 nano in-process, wired into the product
  (`--features llama`), held-out quality passes on the product: Alloprof +0.103 [+0.090, +0.115], mix-fr-en +0.083 [+0.074, +0.093].
- 16 Nov, PRs #21 to #29: Jina nano installable (`footnote models install jina-v5-nano`, release builds with `--features llama`);
  hybrid recall on every prompt through the resident daemon; Library search hybrid by default when the corpus models are installed;
  scope and tags as pre-filters before ranking; Alias rules plus OpenAI Privacy Filter pass on held-out; the tag engine (tags
  guessed from the prompt) built, failed its test ([MEMORY-TAGS.md](../../bench/experiments/MEMORY-TAGS.md)), left off.
- Audit-chain fork fixed (two concurrent MCP sessions); Rust-encoder build with metrics installed (15 Nov).
- Documentation optimisation (16 Nov, [protocol and results](../../bench/experiments/DOCS-OPTIMIZE-2026-11-16.md)), PRs #19
  and #20: adaptive crawl (x3.6), heading path (+0.044), structure-aware chunks (+0.051), one passage per page (+0.029),
  extraction fix for footer-only pages, lean search output (-42 % characters, rankings identical). Installed (metrics build).
- **Sealed final against Grounded Docs: worse** (-0.203 nDCG@5): equivalent on Hono, lost on Flask, whose crawl Cloudflare
  blocked after Grounded's scrape; Footnote also trails on Grounded's own Flask pages (-0.073). Next: look at Sphinx-built
  docs (Flask, Python) on the development sets, and fix the final's design (alternate order or cool-down between systems)
  before a new sealed set.

## P0: protect and finish what exists (small, this week)

1. ~~Commit and push~~ done by Johan, 15 Nov.
2. ~~Make memory work visible~~ done 15 Nov: once per session the hook asks agents to say when a memory shapes the answer; the save
   request ends with a one-line receipt (count and ids).
3. ~~Zero retention in the hook~~ done 15 Nov: old journals deleted, session state keeps only counters. Also fixed: long accented
   prompts failed recall (cut at 500 characters, limit is 512 bytes), the 4 recall failures of the first days.
4. ~~`bk-gsh`, hybrid recall on each prompt~~ done 16 Nov ([MEMORY-RECALL-SWITCH.md](../../bench/experiments/MEMORY-RECALL-SWITCH.md)):
   live keyword recall had no gate either, so the switch was judged on recall: on 100 questions over Johan's real memories,
   any@3 0.91 against 0.77 (+0.14 [+0.08, +0.21]; French 0.84 against 0.58), +13 ms per prompt at p95, daemon about 450 MB.
   `memory.resident` is now on by default. Still open: a no-answer decision so irrelevant memories stop being injected (both modes).
   Also 16 Nov ([MEMORY-TAGS.md](../../bench/experiments/MEMORY-TAGS.md)): project scope is now a pre-filter before ranking, and memories
   carry up to 10 tags that narrow recall (untagged ones stay candidates). Tag engine (tags inferred from the prompt) built and
   tested the same day: fails (any@3 0.91 to 0.87, worse with incomplete tags), so `memory.infer_tags` stays off.
5. ~~Ship Jina nano as the opt-in French/mixed profile~~ done 16 Nov (PR #22): `models install jina-v5-nano`
   (CC-BY-NC-4.0, shown at install), release builds with `--features llama`. Linux build untested while CI is blocked.
6. **Memory review** around 21 November (baseline: memory 134).

7. ~~Documentation tool for agents~~ done 15 Nov (Johan): Context7 removed from Claude, Codex and the shared agent contracts;
   documentation lookup goes through Firecrawl developer search (docs, repos, issues, PRs). Still open, low to mid priority: a
   performance pass on our code with it, Windows first, then Linux and macOS; rust-docs-mcp (local, exact versions) only if
   Firecrawl falls short on our pinned crates.

## P1: product structure (next)

Plan and decisions: [P1-PLAN.md](P1-PLAN.md) (engine commands first, then design, then the Tauri launcher).

8. **Separate products, features on or off:** Library, Memory, Code, Alias enabled independently; nothing loaded that is not used.
9. ~~**Model manager**~~ done: `models install / use / remove / list`, and `models recommend` from the detected hardware and
   the corpus language (the pair by default, Jina nano for French or mixed corpora; GPU reported, not used for search). Alias's
   detector installs through it too. The full model matrix (licence × hardware × language) stays a P3 benchmark.
10. **Hook in Rust:** `footnote hook` live in Claude, Codex and DeepSeek since 15 Nov (Python hook p50 39 ms, p95 225 ms, plus
    interpreter start; first Rust recall 24 ms). Done: `tools/memory_hook.py` and its tests deleted, docs point to `src/hook.rs`.
    Kept on purpose: `tools/embed.py`, no longer on any product path, stays the reference recipe that benchmarks import and fingerprint
    (`rust_encoder.py`, `helper_speed.py`, `server_benchmark.py`, `engines.py` cache key).
11. **Icons:** one family, one icon per product.
11b. **Phones and tablets** (Johan 16 Nov, [study of T3 Code's remote access](../research/REMOTE-ACCESS-STUDY.md)): today through T3
    Code, nothing to build. With the launcher: serve its screens on the LAN or tailnet with one-time QR pairing, per-device
    scoped sessions, private addresses only. No hosted relay unless a customer asks.

## P2: new products

12. **Footnote Alias** (pseudonymisation before the LLM): desk study done 16 Nov ([ALIAS-DESK-STUDY.md](../research/ALIAS-DESK-STUDY.md)):
    French identifier rules in Rust (no tool ships them), one small model for names and addresses chosen by measurement on French
    text (GLiNER2-PII, urchade multi PII, OpenAI Privacy Filter as reference), stable keyed pseudonyms. Done 16 Nov: evaluation
    set v1 and rule ([ALIAS-EVAL.md](../../bench/experiments/ALIAS-EVAL.md)); the Rust rules (`crates/alias`, `footnote alias-scan`)
    pass the identifiers part on held-out (1.000, no look-alike hit). Rules plus OpenAI Privacy Filter (ONNX q4) pass names on
    held-out (0.913, misses are lone first names). Done 16 Nov: both inside the binary (`alias-scan --model`, same held-out scores,
    34 ms per text) with stable keyed pseudonyms (`--pseudonymise`). Installed with `footnote models install openai-privacy-filter`
    (every file pinned). Next: the underline-and-explain prototype; GLiNER2-PII only on a new set v2 if names need more margin. Design notes below.
13. **Footnote Code:** code-graph study ([plan](../research/CODE-GRAPH-STUDY.md)). Stage 1 built 16 Nov
    ([design, section 10](../CODE-GRAPH-DESIGN.md)): `code-index` and `code <name>` from `rust-analyzer scip` (callers, callees,
    stale files), opt-in feature `code`. Next: the MCP tool, the other languages' SCIP indexers, then the tree-sitter pass.
    Its graph visualizer uses the family's design language, decided with the launcher in the M2 studio
    ([analysis, section 5](../research/LAUNCHER-DESIGN-ANALYSIS.md)).
14. **Documentation ingestion in Footnote Library** (Johan 15 Nov): crawl and index a documentation site on demand, one index per
    library version, so agents search docs locally, offline, without credits. Cheapest first: `llms.txt` and published `.md` pages,
    then sitemap and plain HTML to Markdown; JavaScript rendering only if needed. Rust crawler candidates to check (licence,
    Windows). Measure first against Grounded Docs (P3). Mistral's documented website-ingestion controls and the public-source
    limitations are recorded in [MISTRAL-LIBRARIES-CRAWLER-2026-11-15.md](../research/MISTRAL-LIBRARIES-CRAWLER-2026-11-15.md).

## P3: benchmarks, when a decision needs them

- Model matrix and setup playbook (feeds item 8).
- Test 5 stage A3 (Octen-4B, F2LLM-v2-1.7B, harrier-oss-v1) and the llama.cpp speed grid (`-fa`, `-ub`, runtime versions).
  Recheck Qwen3-4B with a single terminal token and a fresh embedding cache: the earlier server configuration appended one automatically
  in addition to the lab suffix. The historical double-token measurements remain recorded in `LLAMACPP-MODELS.md`.
  llama.cpp b12421 (15 Nov) ships Windows builds for Vulkan and, new for the RX 6800, ROCm 10.0 (HIP): re-run the GPU speed on
  both against b11901 Vulkan. The product's `llama-cpp-2` crate is still 0.1.161 (its own bundled llama.cpp): upgrade when it follows.
- Storage backends: SQLite + Tantivy against Postgres + pgvector, LanceDB, HelixDB, Omnigraph; Qdrant for large companies only.
  Johan authorized PostgreSQL and Docker installation for this work. PostgreSQL + pgvector were provisioned in an isolated benchmark
  environment; Docker has not been installed. Component checks and their limits are recorded in
  [POSTGRES-PGVECTOR-2026-11-16.md](../../bench/experiments/POSTGRES-PGVECTOR-2026-11-16.md),
  [QDRANT-STORAGE-2026-11-15.md](../../bench/experiments/QDRANT-STORAGE-2026-11-15.md) and
  [GRAPH-STORES-2026-11-16.md](../../bench/experiments/GRAPH-STORES-2026-11-16.md).
- Memory rematch (mem0, Cognee graph, Hindsight, Graphiti, OpenViking, EverOS), after the review.
- Documentation search against Grounded Docs (`arabold/docs-mcp-server`, MIT, TypeScript, SQLite FTS5 + sqlite-vec, embeddings
  through external providers, telemetry on by default), on its own published set: 59 queries over react, python, fastapi, vite and
  a fifth library, MRR, Recall@k, nDCG@5, Hit@3. Same pages indexed by both. Parked by Johan 15 Nov (about 2 hours); protocol and
  rule ready in [DOCS-VS-GROUNDED.md](../../bench/experiments/DOCS-VS-GROUNDED.md), needs his go to run their tool through npx.
  Feeds item 14. **Run 15 Nov:** not shown equivalent (nDCG@5 0.564, 0.600 with hits grouped by page, against 0.635). Johan: good
  enough for a V1. Later, each measured before adoption: hits grouped by page (first), crawler improvements, page-cleaning passes
  before embedding, more retrieval techniques, and a larger embedding model for documentation only (indexed rarely, per version,
  so a slower build for better precision is an acceptable trade).
  **Versions (Johan, 16 Nov; post-V1, after the full stack harness ships):** reuse vectors across documentation versions (a cache keyed by chunk-text hash and model, so a
  new version only embeds changed passages), then measure how much two versions share (React 18/19, Python 3.12/3.13) before
  deciding whether versions should share stored passages (a common store per source with a list per version, as git does)
  instead of one full database each; the saving adds up across many stacks and versions.

## Later: company deployment (not urgent, Johan 15 Nov)

**Access control and data tiers**, before the first company deployment:
1. Every document and memory carries a scope (owner, team, department) and a classification tier, on the client's own grid.
2. Filtering happens in the engine before ranking, never by dropping hits from a top-k afterwards.
3. A memory inherits the highest tier of its sources; some tiers never enter agent memory.
4. An agent session carries the clearance of its user, never more.

Advice for every company: classify data before connecting agents; adding it later means re-tagging everything and auditing memory.

**Compliance kit**, to bring the French approval path (about a year) down to four to six months:

| Reviewer | What they get |
|---|---|
| DPO | Record of processing (purpose, data, no retention beyond the index), data-flow diagram, DPIA inputs, Alias and the CJEU SRB argument |
| IT security (SSI) | Architecture (local, no outbound network), hash-chained audit log, threat model, dependencies, vulnerability audit (not run yet) |
| Compliance | AI Act position, licences of every model and dependency, what the tool cannot do |
| CSE / CSEC | No per-employee monitoring; usage statistics aggregated and off by default |

**Cost at company scale** (Johan 15 Nov; measure on a more mature product of the family, not now): RAM, CPU and wall time per hook
and MCP call, measured on an idle machine and turned into a cost per 1,000 users. Shared virtual desktops (VDI, Citrix) host dozens
of users per server, so a light per-call footprint decides whether agents can be switched on for everyone; chained agents also put
every hook on the critical path. Feeds the IT security (SSI) entry of the compliance kit.

**Shared daemon throughput** (measured 16 Nov on an idle machine, `bench/work/mcp-load-idle-20261116/`, React docs, hybrid):
the shared daemon plateaus at 281-286 requests/s from 20 clients on, p50 rising in step with the clients (70 ms at 20, 140 ms at
40), while one server per client reaches 776 requests/s at 20 clients for 9.5 GB summed working set (daemon: 0.7 GB). Cause,
from the code: every query's inference runs while holding the one global encoder-pool lock (`semantic.rs`, `ENCODERS`), and
the keyword search holds the Tantivy cache lock (`lexical.rs`, `OPEN`), so about 3.5 ms per request is serialized across all
sessions. Fix when company scale matters: a lock per model and a small pool of query sessions (2 threads each), the Tantivy
reader shared without a lock (it is thread-safe); verify with the same load bench and identical rankings. Not needed for a
handful of agents on one machine.

## Ideas, unranked

- **AI sobriety: one view of usage, cost and savings** (Johan, 15 Nov). Explore a Footnote-family workspace that combines the
  token/cost visibility Johan values in T3 Code with optimizations inspired by RTK, caveman and ponytail. Show input, output and
  cached tokens, model/provider and cost where authoritative usage and applicable prices are available; distinguish invoice or API
  usage from local estimates and subscription usage whose marginal cost is unknown. Attribute RTK-style filtered output, smaller
  context and response policies to their source without counting the same saving twice. For each saving, show the baseline,
  assumptions and uncertainty: the avoided model tokens and cost are counterfactual estimates, not invoices. Compare equivalent
  tasks with and without optimizations, including correctness and task completion so token cuts do not hide worse outcomes.
  Support multiple agents/providers in one view when their usage data is available; identify gaps explicitly. Local-first,
  opt-in aggregation with no per-employee monitoring or prompt retention by default. Research the T3 Code implementation, RTK
  accounting, provider data, pricing and licences before choosing integrations or presenting quantified business value.
- UI study of T3 Code (https://github.com/pingdotgg/t3code): rounded look, typography, links in chat; check the licence; take
  structure, layout and motion.
- Footnote Roadmap (visual roadmap from memories and Beads) and the project node map.
- Baseline audit across projects: AI contracts and rules, security-reviewed skills, projects behind on push.
- Progressive context levels (summary, overview, full on demand), after OpenViking.
- **Leaner agent output** (Johan, 16 Nov). Field trimming done (PR #20, `store::lean`): one Vite search (5 results) went from
  4,211 to 3,510 characters, and paths lost the Windows `\\?\` prefix. What remains is mostly the passage text. Formats on what
  remains are worth a few points only, because results are mostly free text, not uniform short values.
  - CSV: no (multi-line text). TOON: about 4 points more, quoting-heavy on Markdown and code.
  - JTON (arXiv 2604.05865, one author, April 2026; Rust implementation with Python bindings, code licence not stated): JSON
    superset with a header-once "Zen Grid" for arrays of same-shaped objects; 15-60 % fewer tokens on such arrays (o200k_base),
    0 % on GitHub data without bare strings. Comprehension 59.1 % JSON vs 59.4 % JTON over 350 questions on 10 models (one
    question; GPT-5.1 -8.6 pp), no confidence intervals, no Claude comprehension test, no accuracy comparison with TOON.
  - If a format is tested: pre-registered agent-answer bench on our own questions with Claude, lean JSON vs Markdown vs JTON,
    accuracy and tokens. MCP's JSON-RPC envelope stays JSON; only the tool result text can change, opt-in.
- **Internal serialization: not pursued** (Johan's options of 16 Nov: MessagePack, Bincode, CBOR, FlatBuffers). Vectors are
  already binary blobs in SQLite (388 bytes each), chunks are SQLite columns, Tantivy has its own format; the JSON left (jobs,
  config and policy, crawl-hosts, receipts, CLI output) is meant to be read by people, the launcher and scripts, and is a few KB
  written rarely. Bincode is not self-describing (a struct change breaks stored data). Reopen only if profiling (index build,
  the daemon's ~285 requests/s plateau) shows JSON on a hot path; MessagePack first then.
- Verify the DeepSeek memory hook. Product page v2 (`design/site/index.html`): QLoop review before commit.

## Footnote Alias, design notes

- **Legal anchor** (verified 15 Nov): CJEU C-413/23 P EDPS v SRB, 4 Sept 2025, para 86: pseudonymised data is not personal data for a
  recipient with no reasonable means to re-identify. The client still names the LLM provider as a recipient (para 112) and signs a
  no-re-identification clause. EDPB guidelines 01/2025 (consultation) still disagree; national regulators may be stricter.
- **Johan's principle:** processing is justified by its purpose; the AI tool must not identify the person, the business application
  may. The identity link stays in the client file; Footnote and the LLM see stable pseudonyms keyed with the company's secret (statistics
  and history still work, access and erasure requests stay answerable). The mapping never leaves the machine or enters a log.
- **Product:** optional module, France first (French + English), plug and play in host apps; runs before any text reaches an AI; a very
  good detection rate, never promised as 100 %.
- **UX leaning:** underline while typing; hovering explains the type, the GDPR rule and what to write instead, in plain French,
  editable by the company's DPO. One engine serves live warning, on-demand check and save-time review.
