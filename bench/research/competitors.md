# Competitor selection for Footnote head-to-head (besides ICM)

Research date: 2026-11-13. Method: GitHub API (`gh api repos/...`, read 2026-11-13), PyPI JSON API (read 2026-11-13), shallow clones of mem0 v2.2.1 (commit 94c3fe9f), Cognee v1.6.8 and basic-memory `main`, plus vendor docs fetched the same day.
Nothing was installed or executed. Every recipe below is "tested by reading source", not by running. Items I could not verify are marked UNVERIFIED.

## Decision

| # | Product | Why chosen | Architecture slot |
|---|---------|-----------|-------------------|
| 1 | **mem0** (`mem0ai`) | Most popular real agent-memory layer with a retrieval-capable local mode (68,898 stars). Documented raw-store mode `infer=False`. Python API, ids via metadata. | Flat memories, Qdrant (local file) dense vectors + BM25 sparse vector, additive score fusion, no FTS engine, no chunking |
| 2 | **Cognee** (`cognee`) | 33,287 stars, released yesterday. Only graph-family product with an officially documented "no LLM key" path (docs dated 2026-10-30). Different architecture from Footnote/ICM. | Relational (SQLite) + vector (LanceDB) + graph (Ladybug); chunk search = dense vector, separate in-memory lexical (BM25-style) search; graph exists but is not used by chunk search |

Caveat on slot 2: the graph is the architecture that "teaches something", but under a no-LLM setup the graph is built by a small NER model (GLiNER) and the `CHUNKS` search never touches it. A Cognee run measures "Cognee's chunk index", not "Cognee's graph retrieval". Say so in the report.

Reserve (optional third, not required): **basic-memory** (closest to Footnote architecturally; third-party harness with ready adapters exists). See runner-ups.

---

## 1. mem0

- Repo: https://github.com/mem0ai/mem0 . License: Apache-2.0 (https://github.com/mem0ai/mem0/blob/v2.2.1/pyproject.toml, `license = "Apache-2.0"`).
- Stars/forks: 68,898 / 8,194 (GitHub API, read 2026-11-13). Last push 2026-11-06T16:53Z.
- Latest release: Python SDK **v2.2.1, 2026-09-25** (https://github.com/mem0ai/mem0/releases/tag/v2.2.1); PyPI `mem0ai 2.2.1` uploaded 2026-09-25 (https://pypi.org/project/mem0ai/). The GitHub "Latest" badge points to the Node SDK ts-v3.3.1 (2026-09-25), ignore it.
- NOTE: API changed at v2: `search(query, top_k=, filters={"user_id":...})`; top-level `user_id=` on search raises ValueError (mem0/memory/main.py, `search` docstring, v2.2.1). Old blog snippets are wrong.

### Architecture (from source, v2.2.1)
- Storage: Qdrant in **local embedded mode** (`qdrant-client` `path=`), default path `/tmp/qdrant` (`mem0/configs/vector_stores/qdrant.py`), plus a SQLite `history.db` under `~/.mem0` (`MemoryConfig.history_db_path`, `mem0/configs/base.py`). Also a second hidden Qdrant store `~/.mem0/migrations_qdrant` for telemetry unless `MEM0_TELEMETRY=False` (`mem0/memory/main.py` lines ~516-541).
- Lexical: not an FTS engine. A Qdrant **sparse BM25 vector** computed by fastembed `SparseTextEmbedding("Qdrant/bm25")` (`mem0/vector_stores/qdrant.py` line ~97). Language is fastembed's default `"english"` (stemmer + stopwords; https://github.com/qdrant/fastembed/blob/main/fastembed/sparse/bm25.py), mem0 does not override it. Needs `fastembed` installed or BM25 is silently disabled (qdrant.py line ~102).
- Vector: dense, one vector per memory, cosine.
- Fusion (`mem0/utils/scoring.py`, `score_and_rank`): additive, `(semantic + bm25 + entity_boost) / max_possible`. **Candidate set comes only from the dense top `max(top_k*4, 60)`; BM25 only re-scores those candidates**, it cannot add documents the dense search missed. `threshold` (default 0.1) gates on the dense score before fusion. Entity boost needs spaCy (not installed by default) and an entity store that `infer=False` never fills, so it is inactive in raw mode.
- Default embedder: `openai` (needs key). Local option `fastembed`, default model there `thenlper/gte-large` (`mem0/embeddings/fastembed.py`). Force a small model with `BAAI/bge-small-en-v1.5`, 384 dim.
- Chunking: none. One `add()` message = one memory = one vector. The embedder truncates at its own max length (512 tokens for bge-small, from model card, UNVERIFIED in this session).

### Windows install (pinned; combination not run)
```powershell
uv venv --python 3.12 D:\bench\venv-mem0
uv pip install --python D:\bench\venv-mem0\Scripts\python.exe "mem0ai==2.2.1" "fastembed==0.9.1" "qdrant-client==1.20.0"
```
Versions read from PyPI 2026-11-13 (fastembed 0.9.1 = 2026-10-24, qdrant-client 1.20.0 = 2026-10-22). Python >=3.10 required. **Do not install `mem0ai[nlp]`/spaCy**: `mem0/utils/spacy_models.py` auto-downloads `en_core_web_sm` from the network when spaCy is present but the model is missing, and the lemmatizer is English-only. Without spaCy, `lemmatize_for_bm25` returns the text unchanged and `extract_entities` returns `[]` (verified in source). fastembed downloads ONNX models and the BM25 stopword files from Hugging Face on first use; pre-warm the cache, then set `HF_HUB_OFFLINE=1`.

### Recipe (no LLM call)
```python
import os
os.environ["MEM0_TELEMETRY"] = "False"        # BEFORE importing mem0: also disables GitHub-raw notice fetch + posthog
os.environ["MEM0_DIR"] = r"D:\bench\mem0\home"
from mem0 import Memory

CFG = {
  # LLM is constructed at init (default provider openai would raise without a key); it is never called when infer=False.
  "llm": {"provider": "openai", "config": {"api_key": "sk-unused", "model": "gpt-5-mini"}},
  "embedder": {"provider": "fastembed",
               "config": {"model": "BAAI/bge-small-en-v1.5", "embedding_dims": 384}},
  "vector_store": {"provider": "qdrant",
                   "config": {"collection_name": "bench", "embedding_model_dims": 384,   # default dims is 1536: must set
                              "path": r"D:\bench\mem0\qdrant", "on_disk": True}},
  "history_db_path": r"D:\bench\mem0\history.db",
}
m = Memory.from_config(CFG)

# (a) ingest: ids are random UUIDs, so carry our id in metadata
for doc_id, text in docs:
    m.add([{"role": "user", "content": text}], user_id="bench",
          metadata={"doc_id": doc_id}, infer=False)

# (b) query -> ranked ids
r = m.search(query, top_k=10, filters={"user_id": "bench"}, threshold=0.0)   # threshold 0.0 is allowed (0..1)
ranked = [x["metadata"]["doc_id"] for x in r["results"]]
```
Sources: `add()`/`_add_to_vector_store` raw branch, `_create_memory`, `_search_vector_store`, `MemoryItem` (mem0/memory/main.py, mem0/configs/base.py, v2.2.1). A third party uses the same pattern (`infer=False`, `filters={"user_id":...}`, `top_k`, metadata `source_doc_id`): https://github.com/basicmachines-co/basic-memory/blob/main/benchmarks/src/basic_memory_benchmarks/providers/mem0_local.py (read 2026-11-13). Their provider also calls `delete_all` and closes both Qdrant clients (`vector_store.client`, `_telemetry_vector_store.client`) between runs.

(c) On-disk store to measure: `D:\bench\mem0\qdrant` (Qdrant local files; I believe a SQLite-backed `storage.sqlite` inside the collection folder, UNVERIFIED) + `D:\bench\mem0\history.db` (+ `MEM0_DIR\migrations_qdrant` if telemetry was left on). Measure the three together; `history.db` duplicates every text.

### Caveats
- **Hidden LLM calls**: none with `infer=False` (raw branch does only `embedding_model.embed` + `vector_store.insert`). With `infer=True` (the default!) every `add()` calls the LLM. Forgetting `infer=False` = failure at first add without a key.
- **Network side effects** unless `MEM0_TELEMETRY=False`: posthog events (`mem0/memory/telemetry.py`), and `notices.py` fetches `raw.githubusercontent.com/.../oss_notices_config.json`. Also a FIXED-path telemetry Qdrant store: in one process a second `Memory()` dies with "Storage folder ... already accessed" (documented in the basic-memory harness comment; qdrant local mode allows one client per path).
- **No dedup/merge in raw mode** (new uuid every time, md5 stored in payload only), so ids do not change. Duplicated texts stay duplicated. Messages with role `system` are skipped; empty/None content skipped (warning).
- **Scale**: qdrant-client local mode warns above 20,000 points (`LARGE_DATA_THRESHOLD = 20_000`, https://github.com/qdrant/qdrant-client/blob/master/qdrant_client/local/qdrant_local.py). It is brute force, not an index. BEIR SciFact (5.2k) and NFCorpus (3.6k) are fine; larger corpora would not be representative.
- **Speed**: embedding is one text per call (`FastEmbedEmbedding.embed` is not batched), plus one SQLite history write and one BM25 encode per doc. Do not compare ingest wall time with a batching engine without noting this.
- **French (Alloprof, Syntec)**: BM25 stemmer/stopwords are English; default `bge-small-en-v1.5` is English. For a fair FR run you must swap the embedder to the same multilingual model as the other systems (any fastembed-supported model; set `embedding_dims` accordingly) and accept that mem0's lexical leg stays English.
- **Query length**: BM25 sigmoid parameters are tuned by lemma count (`get_bm25_params`, thresholds at 3/6/9/15 terms); long queries (SciFact claims, LongMemEval questions) get fixed midpoints. Not tunable without patching.
- Raw mode is **not how mem0 sells itself** (its headline numbers use LLM fact extraction). State this wherever results are shown.

### Published numbers (all vendor-reported unless noted)
| Claim | Source | Provenance |
|---|---|---|
| LoCoMo 92.5, LongMemEval 94.4, BEAM-1M 64.1 (README) / 70.1 pass-rate (benchmarks repo); "Mem0's managed platform" stack, top_200 retrieval, LLM judge | README at https://github.com/mem0ai/mem0/blob/v2.2.1/README.md ("New Memory Algorithm (April 2026)"); repo https://github.com/mem0ai/memory-benchmarks (page read 2026-11-13) | **Vendor-reported**, managed platform, LLM-extraction + LLM judge. Not reproducible keyless. |
| OSS self-hosted LongMemEval 88.6 to 91.0 depending on extraction LLM (GPT-5 91.0, GPT-OSS-120B 89.8, Llama 4 Maverick 88.6, Gemma 4 31B 88.6) | https://github.com/mem0ai/memory-benchmarks (summary of page, read 2026-11-13; I did not open the raw table) | **Vendor-reported**, needs LLM |
| Raw-add (`infer=false`) retrieval on LongMemEval-S stratified 60 questions: recall@5 0.979, recall@10 0.992, MRR 0.876, mean latency 146 ms; ConvoMem-274: recall@5 0.996, MRR 0.956 | https://github.com/basicmachines-co/basic-memory/blob/main/benchmarks/benchmarks/results/matrix-v2-summary.md (file last committed 2026-06-25) | **Competitor-run** (basic-memory's maintainers), 60-question subset, embedder is a local OpenAI-compatible endpoint (provider default `nomic-embed-text` 768d via Ollama; the run's exact embedder not stated in the summary). Not independent reproduction of mem0's claims. |
| "Mem0 ~85% R@5" on LongMemEval | https://github.com/doobidoo/mcp-memory-service/blob/v12.8.0/docs/BENCHMARKS.md | Unsourced row in a competitor's table. Do not cite. |
| BEIR / MTEB-FR | none found | -- |

---

## 2. Cognee

- Repo: https://github.com/topoteretes/cognee . License: Apache-2.0 (pyproject at tag v1.6.8).
- Stars/forks: 33,287 / 3,510 (GitHub API, read 2026-11-13). Last push 2026-11-13T06:12Z.
- Latest release: **v1.6.8, 2026-11-12** (https://github.com/topoteretes/cognee/releases/tag/v1.6.8; PyPI `cognee 1.6.8`, 2026-11-12, requires Python >=3.10,<3.15). Release cadence is very fast (1.x moving API, see caveats).

### Architecture (from source v1.6.8 + docs)
- Storage (defaults): relational **SQLite** (`DB_PROVIDER=sqlite`), vector **LanceDB** (`vector_db_provider = "lancedb"`), graph **Ladybug** (`graph_database_provider = "ladybug"`) (https://docs.cognee.ai/guides/local-setup, cognee/infrastructure/databases/*/config.py). Default roots are `.cognee_system` and `.data_storage` resolved relative to the **installed package** (`cognee/base_config.py` lines 24-25): override with env.
- Lexical: `SearchType.CHUNKS_LEXICAL`, a token-overlap/BM25-style scorer, **in-process scan of all `DocumentChunk` nodes read from the graph store** (cognee/modules/retrieval/lexical_retriever.py), tokenizer `re.findall(r"\w+", lower)`. Not dataset-scoped at load time and cached after first query (`_initialized`). Separate search type, no fusion with dense.
- Vector: `SearchType.CHUNKS` = dense vector search over chunks, raw backend cosine **distance** returned as `score` (lower is better) (cognee/modules/retrieval/chunks_retriever.py).
- Fusion: none for chunk searches. `HYBRID_COMPLETION` fuses chunk lane + graph but, without an LLM key, returns only the assembled prompt text (`only_context=True`), not a ranked id list (docs guide below).
- Default embedder when no key: fastembed `BAAI/bge-small-en-v1.5`, 384 dim, 67 MB (cognee/infrastructure/databases/vector/embeddings/config.py comment; https://docs.cognee.ai/guides/no-llm-remember-recall).
- Chunking: yes. Default `TextChunker` (paragraph-based), token budget auto-derived (`min(embedding token limit, llm_max_completion_tokens // 2)`; with no key the exact value is UNVERIFIED, so pass `chunk_size=` explicitly). One document yields N chunks, search returns chunks.
- Graph: built by `cognify`; keyless = GLiNER demo extractor (NER + relations) and summaries "from_extraction" (no LLM) (cognee/modules/cognify/config.py).

### Windows install (pinned; not run)
```powershell
uv venv --python 3.12 D:\bench\venv-cognee
uv pip install --python D:\bench\venv-cognee\Scripts\python.exe "cognee[gliner]==1.6.8" --extra-index-url https://download.pytorch.org/whl/cpu
```
`gliner` extra = `gliner2[local]`, `torch>=2.1`, sentencepiece, psutil (pyproject at v1.6.8 line ~295). If you skip the extra, cognee **installs CPU torch + GLiNER at first `cognify`** from `https://download.pytorch.org/whl/cpu` (`gliner_auto_install=True` default, cognee/modules/cognify/config.py); set `GLINER_AUTO_INSTALL=false` to forbid a runtime pip install. GLiNER model and fastembed model are also downloaded from the HF hub on first use (cognee/shared/model_download_notice.py). Pre-warm, then `HF_HUB_OFFLINE=1`. Heavy dependency tree (litellm, openai, lancedb, etc.): expect a multi-GB venv, UNVERIFIED size.

### Recipe (no LLM call, per the official keyless guide)
Doc: https://docs.cognee.ai/guides/no-llm-remember-recall (page `dateModified` 2026-10-30). Env must be set before `import cognee`.
```python
import os, asyncio, uuid
for v in ("LLM_API_KEY", "OPENAI_API_KEY"): os.environ.pop(v, None)
os.environ.update({
  "AUTO_FEEDBACK": "false", "TELEMETRY_DISABLED": "true",
  "SYSTEM_ROOT_DIRECTORY": r"D:\bench\cognee\system", "DATA_ROOT_DIRECTORY": r"D:\bench\cognee\data",
  "EMBEDDING_PROVIDER": "fastembed", "EMBEDDING_MODEL": "BAAI/bge-small-en-v1.5", "EMBEDDING_DIMENSIONS": "384",
  "GRAPH_EXTRACTOR": "gliner_demo", "GLINER_AUTO_INSTALL": "false",
})
import cognee
from cognee import SearchType
from cognee.tasks.ingestion.data_item import DataItem   # import path read from source; may also be exported at top level, UNVERIFIED

NS = uuid.UUID("00000000-0000-0000-0000-00000000b0b0")
to_uuid = {doc_id: uuid.uuid5(NS, doc_id) for doc_id, _ in docs}
from_uuid = {str(u): d for d, u in to_uuid.items()}

async def ingest(docs):
    items = [DataItem(data=text, label=doc_id, data_id=to_uuid[doc_id], literal_text=True) for doc_id, text in docs]
    await cognee.add(items, dataset_name="bench")                          # (a) no LLM
    await cognee.cognify(datasets=["bench"], extractor="gliner_demo", chunk_size=512)   # chunks are searchable after the indexing stage; GLiNER graph built after

async def query(q, k=10):
    hits = await cognee.search(q, query_type=SearchType.CHUNKS, datasets=["bench"], top_k=k * 3)   # dense
    # lexical leg: query_type=SearchType.CHUNKS_LEXICAL
    seen, ranked = set(), []
    for h in hits:                                   # envelope shape UNVERIFIED: smoke test on 3 docs first
        d = from_uuid[str(h["document_id"])]         # DocumentChunk.document_id = source Data id (cognee/modules/chunking/models/DocumentChunk.py)
        if d not in seen: seen.add(d); ranked.append(d)
    return ranked[:k]
```
Why I believe ids survive: `DataItem.data_id` is a pinned id honoured in `ingest_data` (cognee/tasks/ingestion/data_item.py, ingest_data.py lines ~308-371); chunks carry `document_id`/`document_name` (DocumentChunk docstring). `literal_text=True` stops a text that looks like a URL or path from being fetched. Not run; confirm `document_id` equals the pinned `data_id` on a 3-document smoke test. Fallback if not: document name derived from content hash (`text_<md5>.txt`, cognee/modules/ingestion/save_data_to_file.py), map via md5 of text.

Official no-key demo uses `cognee.remember(...)` then `cognee.recall(query, datasets=[...], top_k=3)`; with no key `recall` resolves to `HYBRID_COMPLETION` with `only_context=True`, whose `item.text` is a prompt, not ids. So use `search(..., CHUNKS)` for id lists.

(c) On-disk store to measure: whole `SYSTEM_ROOT_DIRECTORY` (SQLite relational DB + LanceDB dir + Ladybug graph dir) and `DATA_ROOT_DIRECTORY` (a copy of each raw document stored as a file, content-addressed). Report separately: the vector-only share is the LanceDB dir; the rest is cognee-specific overhead.

Alternative (UNVERIFIED, more work, avoids GLiNER entirely): build a custom pipeline with only `classify_documents` -> `extract_chunks_from_documents` -> `index_chunks_for_search` (cognee/modules/cognify/chunking_tasks.py, cognee/tasks/storage/index_chunks_for_search.py, docs example "Rebuild Add and Cognify by Hand"). The source docstring states these stages make no LLM call. Label such a run "Cognee chunk index only".

### Caveats
- **Hidden LLM calls**: with no key, defaults resolve to local GLiNER + fastembed (`GRAPH_EXTRACTOR=auto`). If any `LLM_API_KEY`/`OPENAI_API_KEY` leaks in from the shell, `auto` flips to the LLM extractor and spends money. Pin `GRAPH_EXTRACTOR=gliner_demo` and unset the keys. Media loaders (image/audio/video) need an LLM; plain text does not.
- **Ingestion cost**: GLiNER CPU inference over every chunk is the slow part; no figures measured. Plan a 100-document timing probe before committing SciFact/NFCorpus runs. Chunk indexing happens first (`early_chunk_indexing=True`), so queries could start earlier, but benchmark ingest time should include `cognify` completion.
- **Dedup**: Data ids are content-hash based by default; identical texts collapse. Pinned `data_id` avoids id drift but identical text under two ids may still be deduplicated, UNVERIFIED.
- **Chunk-level results**: several chunks per doc; you must `top_k * n` and dedupe to doc ids. Dense `score` is a distance (lower better); sort order is what matters.
- **Lexical leg is an in-memory O(N) scan of all chunks in the graph store, not scoped to the dataset** and cached; use one fresh `SYSTEM_ROOT_DIRECTORY` per corpus and one process per corpus.
- **API churn**: 1.x introduced `remember/recall/improve/forget`; `add/cognify/search/memify` are now listed under "Legacy Operations" in the docs nav (https://docs.cognee.ai/guides/no-llm-remember-recall page nav, read 2026-11-13). Pin `==1.6.8`.
- **Python 3.15 unsupported** (`<3.15`).
- Cognee does **not** offer a fused dense+lexical id ranking. Any RRF fusion of `CHUNKS` and `CHUNKS_LEXICAL` would be our own addition; report the two legs separately.

### Published numbers (all vendor-reported)
| Claim | Source | Provenance |
|---|---|---|
| LoCoMo 91.3% (1,762 questions; answers `gpt-5-mini`, judge `gpt-5.1`), BEAM-100K 0.79, BEAM-10M 0.67 (`gpt-5`, `text-embedding-3-large`) | https://docs.cognee.ai/core-concepts/evaluation-and-efficiency/accuracy-benchmarks (read 2026-11-13; page gives no date and no competitor) | **Vendor-reported**, paid OpenAI stack, LLM judge. Not comparable to a keyless run. |
| LongMemEval / BEIR / MTEB-FR | none found | -- |

---

## Runner-ups rejected (ranked)

| Rank | Product (stars read 2026-11-13, latest release) | Why not |
|---|---|---|
| 1 | **basic-memory** (4,563; v1.0.0, 2026-10-29; AGPL-3.0) https://github.com/basicmachines-co/basic-memory | Strong agent-memory product, runs fully local (SQLite FTS5 + sqlite-vec + fastembed bge-small 384d, score fusion `max(vec,fts)+bonus*min`, chunking: headers/bullets/~900-char prose; `docs/semantic-search.md`). Rejected only because it is architecturally nearest to Footnote/ICM and ingestion is files-on-disk + `bm reindex`. Notes if reserve is wanted: set `semantic_min_similarity` to 0 (default 0.55 hides results), ids via frontmatter `permalink`/`source_doc_id`, DB at `BASIC_MEMORY_CONFIG_DIR/memory.db`; AGPL (internal benchmarking only). Ready-made `bm-local` + `mem0-local` adapters in its `benchmarks/` folder. Its own results are competitor-run (above). |
| 2 | **txtai** (13,367; v9.14.0, 2026-10-06; Apache-2.0) https://github.com/neuml/txtai | Cleanest adapter (`index([(id, text, None)])` returns ids natively, hybrid dense + sparse BM25, SQLite content store), no LLM. Rejected: a general embeddings-DB framework, not an agent-memory product, and same hybrid family as us. Best choice if adapter simplicity or a calibration baseline matters more than "agent memory" realism. API recipe not verified in this session. |
| 3 | **MCP Memory Service** (doobidoo) (2,118; v12.8.0, 2026-11-10; Apache-2.0) | sqlite-vec + ONNX all-MiniLM-L6-v2, vendor LongMemEval-S session R@5 86.0% (v10.34.0, 2026-04-08, vendor-reported, https://github.com/doobidoo/mcp-memory-service/blob/v12.8.0/docs/BENCHMARKS.md). Semantic dedup rejected 4 of 85 writes in its own run, which changes ids unless disabled (`MCP_SEMANTIC_DEDUP_ENABLED=false`). Low popularity vs the two chosen. |
| 4 | **claude-mem** (116,636; v15.18.3, 2026-11-12; Apache-2.0) | Most stars, but ingestion is hook-driven and an LLM "observer" writes observations (README: "A second agent turns that activity into short observations"); SQLite FTS5 + Chroma. No (id, text) ingest API. |
| 5 | **Graphiti / Zep** (32,787; v0.31.2, 2026-10-20) | Ingest runs LLM entity/edge extraction; README requires an OpenAI key by default; needs Neo4j/FalkorDB/Neptune (Kuzu marked deprecated). No documented raw mode. |
| 6 | **LightRAG** (40,680; v1.5.7, 2026-09-02; MIT) | README: LLM key "Required"; entity extraction is one LLM call per chunk. No documented raw-store mode found. |
| 7 | **Letta** (25,428; 0.16.8, 2026-05-14) | Stateful agent runtime, memory is LLM-managed; not a retrieval-only layer. |
| 8 | **Khoj** (38,120; 2.0.0-beta.28, 2026-03-26; AGPL-3.0) | Chat/app product needing an LLM; last release 8 months old. |
| 9 | **Official MCP `memory` server** (monorepo 91,957 stars; v2.4.0, 2026-11-11) | JSONL knowledge graph with `search_nodes`; the README lines I read show no embeddings. Not a retrieval-quality peer (README at https://github.com/modelcontextprotocol/servers/tree/main/src/memory). Search semantics UNVERIFIED. |
| - | OpenMemory, Zep Community Edition, Chroma/LanceDB memory MCP servers | Not examined in depth; OpenMemory is a wrapper around mem0 (UNVERIFIED), so it adds nothing over choice 1. |

---

## Biggest risks of benchmarking each

**mem0**
1. Raw mode (`infer=False`) is not the product mem0 markets; its published numbers use LLM extraction on the managed platform. Dense-only candidate set, English-only BM25, default threshold 0.1 and per-query-length BM25 constants all disadvantage it on BEIR/French. Use `threshold=0.0` and say so.
2. Qdrant local mode is brute-force and warns above 20k points; embeddings are unbatched; telemetry and notice fetches break "offline" and the hidden fixed-path telemetry store breaks multi-instance runs unless `MEM0_TELEMETRY=False`. Storage must include `history.db`.
3. Ids are random UUIDs; mapping rests on `metadata["doc_id"]`. Long docs (e.g. session-level LongMemEval) are not chunked, embedder truncates.

**Cognee**
1. The keyless path installs/downloads torch + GLiNER + models at first use (network, multi-GB, CPU-slow ingestion). The graph that makes Cognee different is not exercised by `CHUNKS`; you benchmark a chunk index.
2. Results are chunk-level; mapping to doc ids depends on `DataItem.data_id` -> `document_id` (read from source, not run); the `search` return envelope is unverified; lexical leg is a global in-memory scan with caching; there is no fused ranking, so dense and lexical must be reported as two systems.
3. Three stores plus raw file copies under package-relative default dirs make "on-disk storage" non-comparable unless split (LanceDB only vs total); fast-moving 1.x API with legacy ops. Pin 1.6.8.

Reserve-product risk (basic-memory) if added: AGPL, default similarity cutoff 0.55 hides hits, file-based ingestion.
