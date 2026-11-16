# Footnote backlog

Last updated 15 November 2026. One prioritised list; Beads stays the authority for tracked tasks. Order inside a tier is the order
of work. "Johan" marks items waiting for his go.

## Recently done

- Test 5 (llama.cpp models, [rule and results](../../bench/experiments/LLAMACPP-MODELS.md)): no model adopted through an HTTP server.
- Test 6 ([results](../../bench/experiments/LLAMACPP-INPROCESS.md)): Jina v5 nano in-process, wired into the product
  (`--features llama`), held-out quality passes on the product: Alloprof +0.103 [+0.090, +0.115], mix-fr-en +0.083 [+0.074, +0.093].
- Audit-chain fork fixed (two concurrent MCP sessions); Rust-encoder build with metrics installed (15 Nov).

## P0: protect and finish what exists (small, this week)

1. ~~Commit and push~~ done by Johan, 15 Nov.
2. ~~Make memory work visible~~ done 15 Nov: once per session the hook asks agents to say when a memory shapes the answer; the save
   request ends with a one-line receipt (count and ids).
3. ~~Zero retention in the hook~~ done 15 Nov: old journals deleted, session state keeps only counters. Also fixed: long accented
   prompts failed recall (cut at 500 characters, limit is 512 bytes), the 4 recall failures of the first days.
4. **`bk-gsh`, hybrid recall on each prompt:** the memory baseline gave hybrid any@5 0.504 against keyword 0.384.
   Self-contained writing guidance added in the repository after a [600-memory bilingual synthetic pilot](../../bench/experiments/MEMORY-WRITTEN-RESULTS.md).
   Raw retrieval improves in that controlled fixture; the no-answer injection gate still fails. Live hybrid recall remains off;
   real agent-written memories and independent relevance review are still needed.
5. **Jina nano latency on an idle machine** (Johan says when), then ship it as the opt-in French/mixed profile.
6. **Memory review** around 21 November (baseline: memory 134).

7. ~~Documentation tool for agents~~ done 15 Nov (Johan): Context7 removed from Claude, Codex and the shared agent contracts;
   documentation lookup goes through Firecrawl developer search (docs, repos, issues, PRs). Still open, low to mid priority: a
   performance pass on our code with it, Windows first, then Linux and macOS; rust-docs-mcp (local, exact versions) only if
   Firecrawl falls short on our pinned crates.

## P1: product structure (next)

Plan and decisions: [P1-PLAN.md](P1-PLAN.md) (engine commands first, then design, then the Tauri launcher).

8. **Separate products, features on or off:** Library, Memory, Code, Alias enabled independently; nothing loaded that is not used.
9. **Embedding model manager:** install, switch, remove; recommend from the detected hardware (GPU, VRAM, CPU, RAM) and corpus language,
   from Footnote's own results (the model matrix: licence × hardware × language). Today: the pair by default, nano for French/mixed,
   EmbeddingGemma 1 on a GPU.
10. **Hook in Rust:** `footnote hook` live in Claude, Codex and DeepSeek since 15 Nov (Python hook p50 39 ms, p95 225 ms, plus
    interpreter start; first Rust recall 24 ms). Done: `tools/memory_hook.py` and its tests deleted, docs point to `src/hook.rs`.
    Kept on purpose: `tools/embed.py`, no longer on any product path, stays the reference recipe that benchmarks import and fingerprint
    (`rust_encoder.py`, `helper_speed.py`, `server_benchmark.py`, `engines.py` cache key).
11. **Icons:** one family, one icon per product.

## P2: new products

12. **Footnote Alias** (pseudonymisation before the LLM): desk study first (Presidio, LLM Guard, GLiNER PII, French accuracy), then a
    French + English prototype with underline-and-explain. Design notes below.
13. **Footnote Code:** code-graph study ([plan](../research/CODE-GRAPH-STUDY.md)); hands-on after Johan's go.
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
