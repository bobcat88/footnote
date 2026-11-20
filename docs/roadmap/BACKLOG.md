# Footnote backlog

Last updated 17 November 2026. Since 17 Nov the authority for tracked work is GitHub Issues: 12 epics and their stories, built from
[EPICS.md](EPICS.md), done per the [Definition of done](../DEFINITION-OF-DONE.md). This page keeps the context and the reasons. Order inside a tier is the order of work.

Every item belongs to the free or the commercial edition ([EDITIONS.md](EDITIONS.md), decided 16 Nov): company-only features
(sign-on, rights, hierarchy, many team indexes, compliance) go into a separate commercial crate, never into the free engine.

Next prioritized wave: [20 feature goals](NEXT-20-FEATURES.md), including the personal-encryption and recovery decisions. Its open goals are
now stories on GitHub Issues.

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
- Sphinx docs, sealed final 2 (20 Nov, Scrapy and pytest, [results](../../bench/experiments/DOCS-SPHINX-SEALED2-RESULTS-2026-11-20.md)):
  Footnote's own crawl 0.768 against Grounded Docs 0.721 nDCG@5 (+0.047, interval -0.054 to +0.155): inconclusive, not a claim; on the same
  pages 0.712 (level); crawl 94% faster, queries 37% faster.

## P0: protect and finish what exists (small, this week)

1. ~~Commit and push~~ done 15 Nov.
2. ~~Make memory work visible~~ done 15 Nov: once per session the hook asks agents to say when a memory shapes the answer; the save
   request ends with a one-line receipt (count and ids).
3. ~~Zero retention in the hook~~ done 15 Nov: old journals deleted, session state keeps only counters. Also fixed: long accented
   prompts failed recall (cut at 500 characters, limit is 512 bytes), the 4 recall failures of the first days.
4. ~~Hybrid recall on each prompt~~ done 16 Nov ([MEMORY-RECALL-SWITCH.md](../../bench/experiments/MEMORY-RECALL-SWITCH.md)):
   live keyword recall had no gate either, so the switch was judged on recall: on 100 questions over a real user memory database,
   any@3 0.91 against 0.77 (+0.14 [+0.08, +0.21]; French 0.84 against 0.58), +13 ms per prompt at p95, daemon about 450 MB.
   `memory.resident` is now on by default. Still open: a no-answer decision so irrelevant memories stop being injected (both modes).
   Also 16 Nov ([MEMORY-TAGS.md](../../bench/experiments/MEMORY-TAGS.md)): project scope is now a pre-filter before ranking, and memories
   carry up to 10 tags that narrow recall (untagged ones stay candidates). Tag engine (tags inferred from the prompt) built and
   tested the same day: fails (any@3 0.91 to 0.87, worse with incomplete tags), so `memory.infer_tags` stays off.
5. ~~Ship Jina nano as the opt-in French/mixed profile~~ done 16 Nov (PR #22): `models install jina-v5-nano`
   (CC-BY-NC-4.0, shown at install), release builds with `--features llama`. Linux build untested while CI is blocked.
6. **Memory review** scheduled milestone.

7. ~~Documentation tool for agents~~ done 15 Nov: replaced external Context7 dependency with Footnote docs & search tooling.

## P1: product structure (next)

Plan and decisions: [P1-PLAN.md](P1-PLAN.md) (engine commands first, then design, then the Tauri launcher).

- **Launcher (M3), first build 17 Nov** ([apps/launcher](../../apps/launcher/README.md)): Overview, Code (architecture map and
  symbol view), Projects, Models, Activity, Usage, the Enterprise page, and mini mode; read-only, through an allowlist of engine
  commands.
- **Separate products, features on or off:** Library, Memory, Code, Alias enabled independently; nothing loaded that is not used.
- ~~**Model manager**~~ done: `models install / use / remove / list`, and `models recommend` from the detected hardware and
  the corpus language (the pair by default, Jina nano for French or mixed corpora; GPU reported, not used for search).
- **Hook in Rust:** `footnote hook` live across supported agents (Rust recall p95 far faster than Python).
  Kept on purpose: `tools/embed.py`, no longer on any product path, stays the reference recipe that benchmarks import and fingerprint.
- **Icons:** one family, one icon per product.
- **Phones and tablets** ([study of T3 Code's remote access](../research/REMOTE-ACCESS-STUDY.md)): with the launcher, serve its screens on the LAN or tailnet with one-time QR pairing, per-device scoped sessions, private addresses only. No hosted relay unless a customer asks.

## P2: new products

- **Personal corpus, planned:** encrypted personal corpus after a sensitivity triage, with
  originals in secure storage, one editable Markdown per original linked by content hash, and benchmarked high-precision OCR.
  Six gated steps in [PERSONAL-CORPUS-PLAN.md](PERSONAL-CORPUS-PLAN.md); each step explicitly user-approved.

12. **Footnote Alias** (pseudonymisation before the LLM): **moved to Footnote Enterprise on 19 Nov (#144): the code, desk study and evaluation now live in the private `footnote-enterprise` repository, and this free build has no Alias.**
13. **Footnote Code:** code-graph study ([plan](../research/CODE-GRAPH-STUDY.md)). Stage 1 built 16 Nov
    ([design, section 10](../CODE-GRAPH-DESIGN.md)): `code-index` and `code <name>` from `rust-analyzer scip`, opt-in feature `code`.
14. **Documentation ingestion in Footnote Library**: crawl and index a documentation site on demand, one index per
    library version, so agents search docs locally, offline, without credits. Cheapest first: `llms.txt` and published `.md` pages,
    then sitemap and plain HTML to Markdown. Mistral's documented website-ingestion controls and public-source
    limitations are recorded in [MISTRAL-LIBRARIES-CRAWLER-2026-11-15.md](../research/MISTRAL-LIBRARIES-CRAWLER-2026-11-15.md).

## P3: benchmarks, when a decision needs them

- Model matrix and setup playbook (feeds item 8).
- Test 5 stage A3 (Octen-4B, F2LLM-v2-1.7B, harrier-oss-v1) and the llama.cpp speed grid (`-fa`, `-ub`, runtime versions).
  Recheck Qwen3-4B with a single terminal token and a fresh embedding cache.
- Storage backends: SQLite + Tantivy against Postgres + pgvector, LanceDB, HelixDB, Omnigraph; Qdrant for large companies only.
  Results in [POSTGRES-PGVECTOR-2026-11-16.md](../../bench/experiments/POSTGRES-PGVECTOR-2026-11-16.md),
  [QDRANT-STORAGE-2026-11-15.md](../../bench/experiments/QDRANT-STORAGE-2026-11-15.md) and
  [GRAPH-STORES-2026-11-16.md](../../bench/experiments/GRAPH-STORES-2026-11-16.md).
- Memory rematch (mem0, Cognee graph, Hindsight, Graphiti, OpenViking, EverOS).
- Documentation search against Grounded Docs (`arabold/docs-mcp-server`, MIT, TypeScript, SQLite FTS5 + sqlite-vec), on its own published set: 59 queries over react, python, fastapi, vite and a fifth library ([DOCS-VS-GROUNDED.md](../../bench/experiments/DOCS-VS-GROUNDED.md)).
  **Versions (post-V1):** reuse vectors across documentation versions, and measure cross-version chunk sharing.

## Later: company deployment

**Access control and data tiers**, before the first company deployment:
1. Every document and memory carries a scope (owner, team, department) and a classification tier, on the client's own grid.
2. Filtering happens in the engine before ranking, never by dropping hits from a top-k afterwards.
3. A memory inherits the highest tier of its sources; some tiers never enter agent memory.
4. An agent session carries the clearance of its user, never more.

**Compliance kit**, to shorten enterprise security review cycles:

| Reviewer | What they get |
|---|---|
| DPO | Record of processing (purpose, data, no retention beyond the index), data-flow diagram, DPIA inputs, Alias and the CJEU SRB argument |
| IT security (SSI) | Architecture (local, no outbound network), hash-chained audit log, threat model, dependencies, vulnerability audit |
| Compliance | AI Act position, licences of every model and dependency, what the tool cannot do |
| CSE / CSEC | No per-employee monitoring; usage statistics aggregated and off by default |

**Cost at company scale**: RAM, CPU and wall time per hook and MCP call, measured on an idle machine and turned into a cost per 1,000 users.

**Shared daemon throughput** (measured 16 Nov on an idle machine, React docs, hybrid):
the shared daemon plateaus at 281-286 requests/s from 20 clients on. Future optimization: a lock per model and a small pool of query sessions, sharing the thread-safe Tantivy reader lock-free.

## Ideas, unranked

- **Usage tracking (build phase 1 done):** `footnote usage [--days N]` reads local Claude Code, Codex, and DeepSeek session files incrementally, keeps counts only, and prices them from official list-price tables.
- **UI design & layout:** T3 Code study and `DESIGN.md` tokens.
- **Leaner agent output:** `store::lean` field trimming and portable path normalization done; future work in Epic #91 (Footnote Lean).
- **Internal serialization: keep JSON/SQLite blobs:** vectors are already compact binary blobs (`388` bytes) and JSON configs/receipts stay small and human-inspectable.

## Footnote Alias, design notes

- **Legal anchor** (verified 15 Nov): CJEU C-413/23 P EDPS v SRB, 4 Sept 2025, para 86: pseudonymised data is not personal data for a recipient with no reasonable means to re-identify.
- **Core principle:** processing is justified by its purpose; the AI tool receives stable keyed pseudonyms, while the identity mapping stays isolated on the client machine and never enters logs.
- **Product:** optional Enterprise module, plug and play in host apps before text reaches external models.
