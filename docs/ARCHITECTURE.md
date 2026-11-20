# Architecture and schema

## Crates

One workspace, dependencies pointing one way ([ADR 0001](adr/0001-repository-structure.md)):

| Crate | Role |
|---|---|
| `crates/core` (`footnote-core`) | State folder, keys, hash-chained audit log, metrics, safe file access |
| `crates/search` (`footnote-search`) | The one schema and its migrations (`store.rs`), Tantivy keyword index, embeddings (ONNX, optional llama.cpp), fusion, records |
| `crates/library` (`footnote-library`) | Footnote Library: extraction, folder indexing |
| `crates/memory` (`footnote-memory`) | Footnote Memory: remember, recall, pack, forget |
| root package `footnote` (`src/`) | The executable: command line, MCP server, daemon, hook, install |

Products are features of the executable, on by default: `cargo build --release --no-default-features --features library` builds
Footnote Library alone (no memory commands, MCP memory tools or hook).

## Authorities

SQLite owns structured optimization records and their transition history. Files own human-edited notes and original binaries. `sources`, `chunks`, and their FTS rows are derived data; findings are not. Binary bytes are not copied into SQLite. Original paths, SHA-256, revision, processing status, extractor version/helper hash, and page/heading/line references bind citations to the bytes that were indexed. Search results (CLI and MCP) carry only id, path, heading, page, line, text and freshness, with the untrusted-data label once per response; `get <id>` returns the full provenance (hashes, revision, status, extractor).

The Rust executable handles scope, filesystem hashes, transactions, lexical retrieval, JSON bounds, and contribution rules. The optional Python subprocess receives a Rust-owned snapshot of one binary, not the original path. It extracts PDF text with PDFium (via `pypdfium2`); RapidOCR/ONNX handles image transcription, including rasterized scanned/mixed PDF pages. No LLM sees documents. A 120-second timeout kills/reaps the direct helper; output, source bytes, pages, pixels, and text size are bounded. The Python libraries can still allocate native memory; this is not an adversarial parser sandbox or Windows Job Object memory limit.

## Schema migrations

`PRAGMA user_version` is the migration authority. Writes initialize/migrate under SQLite transactions; reads refuse an old schema rather than upgrading it implicitly. Unknown future versions fail closed. Current schema: **4**.

| Version | Objects |
|---|---|
| 1 | `sources(id, root, path UNIQUE, sha256, revision, status, extractor)`; `chunks(id AUTOINCREMENT, source_id FK, heading, page, line, text)`; `search_fts(text, heading)` |
| 2 | `records(id, fingerprint UNIQUE, payload JSON text, payload_sha256, status CHECK, revision, validation JSON text)`; `record_history(record_id, revision, snapshot)`; `records_fts(body)` |
| 3 | Indexes on source content hash/status and chunk source ID for provenance joins/deduplication |
| 4 | Derived `embedding_models(identity, manifest, helper_sha256, dimension)` and `embeddings(model, kind, id, signature, vector)`; authoritative records/history unchanged |
| 5 | `memories(source_id FK, kind CHECK, scope, observed_at, session, supersedes FK)`: agent memories, stored as sources with a `memory:` path and one chunk each |

SQLite foreign keys are enabled on every connection. Readers use `SQLITE_OPEN_READ_ONLY`, without a WAL mode change. Default rollback journal avoids a background service and live-WAL synchronization concerns. Writers use `BEGIN IMMEDIATE` plus a five-second busy timeout. Corpus mutation is one transaction per scan: traversal/read/bound errors roll back the corpus batch. Initial schema migrations can already have committed before a later allowlist failure; no documents have then been imported.

FTS row IDs map directly to chunk/finding IDs. Text chunks target 2,000 Unicode characters and preserve long-line tails. H1 document title and current heading are indexed as context, without inventing new source text. Heading changes flush the current chunk. Exact text and source hash remain separately returned. Page is null for Markdown/TXT, otherwise 1-based. `line` is an original text line for text sources, an extracted-text line for binaries; it is not a PDF geometric coordinate. Long-line slices can share the same line reference; the chunk ID plus hash disambiguates them.

## Incremental behavior

Every explicit index scan reads bounded source bytes and compares SHA-256. It does not rely on mtime. Unchanged successful files skip extraction. UTF-8 chunker versions and the binary helper hash/chunker version participate in reuse decisions. Binary extractor strings record actual pypdfium2/RapidOCR/ONNX versions, OCR use and empty pages. Reprovision exactly from the pinned environment lock; a manually changed interpreter/dependency environment is not automatically re-fingerprinted before reuse. For such changes, reindex a new database or change the extractor revision before reuse.

Changed sources replace their derived chunks and increment revision. Failed/empty/unavailable extraction removes previous searchable chunks, records the processing status and is retried later. Missing paths are tombstoned only after a successful complete scan of their owning root. A rename is represented as an old deleted path and a new source path; it is not a stable document-ID rename operation. Same content at multiple paths remains in provenance but is deduplicated during search by content hash and exact chunk coordinates/text. A deterministic minimum current chunk ID supplies the returned citation.

Search/get recompute hashes on selected sources; record retrieval checks pinned artifacts. Freshness is a point-in-time result, not a guarantee the file cannot change after return. Reading hashes costs I/O, including repeated raw-evidence hashes for findings. There is no persistent freshness cache that could silently hide changed bytes.

## Structured evidence

Payload hashes are SHA-256 of canonicalized `serde_json` serialization of the stored payload; they are not raw import-file hashes. Original raw-file evidence hashes remain in `evidence`/`origin`. Validation binds the payload hash, record ID and revision; it requires all explicit checks to pass and rehashes named artifacts. Hash syntax is validated before lookup. Scope checks use canonical absolute paths and reject symlinks/reparse ancestors. There is no arbitrary SQL endpoint and no implicit promotion by schema validity alone.

Proposal insertion/deduplication and validation/history insertion are transactional. A second stale/concurrent validator observes the changed revision/status after the writer lock and fails. The history contains the proposal and validated snapshot. `superseded` is a reserved database value only; safe revision/edit/withdrawal commands remain future work.

## Hybrid: shipped as an explicit experimental mode

[Implementation, source-bound receipt, two-model comparison and limitations](HYBRID.md). Search defaults to hybrid when the corpus has vectors from installed models, otherwise FTS5. Explicit `rebuild-embeddings --model LOCAL_DIR` stores derived 384-dimensional CPU ONNX embeddings in SQLite JSON float64 arrays; `search --mode hybrid --model LOCAL_DIR` fuses lexical top20 and cosine top60 with RRF k=60. This is an exact bounded scan, not sqlite-vec or ANN. Model/recipe/item identities prevent stale vector reuse; structured filters precede ranking. Read commands never rebuild implicitly.

Both pinned multilingual candidates (E5-small and MiniLM) improved macro recall@5 on the fixed synthetic holdout, with fresh-process p50 near one second and no abstention on missing answers. The benchmark retained raw outputs and checked citation metadata/hashes against the source/corpus snapshot. MiniLM had better early-result ordering in this small run; no universal model selection or rollout is justified. Document compatibility, source authority and withdrawn advice are not inferred from similarity.

The pilot benchmark also tested isolated local vector stores with 384-dimensional embeddings against SQLite JSON arrays and matched ordered top5 across 32 queries per model. Reranking, calibrated answerability, and multi-store federation remain separate design tracks.

## Public references

- [SQLite FTS5](https://www.sqlite.org/fts5.html): tokenization, MATCH, BM25.
- [rusqlite 0.38 backup API](https://docs.rs/rusqlite/0.38.0/rusqlite/struct.Connection.html#method.backup): exercised with `rusqlite::MAIN_DB` and coherent restore-by-open test.
- [PDFium](https://pdfium.googlesource.com/pdfium/) via [pypdfium2](https://pypdfium2.readthedocs.io/) text extraction (permissive replacement for PyMuPDF, which is AGPL-3.0).
- [RapidOCR](https://github.com/RapidAI/RapidOCR): project-local `rapidocr-onnxruntime` wheel and actual extracted-image tests.

Public page extraction via the configured web-extract backend was unavailable; the SQLite official page was fetched directly and the installed rusqlite 0.38 source inspected for the actual API. No documentation fetch transmitted local files.
