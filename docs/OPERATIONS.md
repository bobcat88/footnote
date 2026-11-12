# Operations, privacy and recovery

## Scope and trust

The caller chooses an explicit database and explicit root/allowlist. There is no automatic vault discovery, recursive personal-vault import, cloud upload, tool installation or runtime configuration. Indexing walks without following symlinks or Windows reparse points. Input/evidence path ancestors are checked too. Hidden path components and common infrastructure/private/secret/credential paths are excluded, including `.git`, `.env*`, `.qdrant_storage`, `node_modules`, `target`, `private`, and the paused `optimization-registry` subtree.

These name-based exclusions are defense in depth, not content-based secret detection. A password placed in an ordinary technical Markdown file is still user data; never approve a broad root merely because filters exist. Only index deliberately selected technical folders. Same-user filesystem races or malicious native parsing exploits are outside this pilot's containment boundary; do not ingest hostile files from untrusted writers. PDF JavaScript is not executed, but parsers/OCR remain native dependencies requiring maintenance.

Search and get return content with explicit `untrusted_*` labels. Prompt injection is stored as data and never executed by the CLI. An agent consuming results must still treat them as data: retrieved instructions cannot authorize shell actions, modify agent policy, or override the user's scope. No trust label replaces consumer-side instruction/data separation.

## Bounds and failure modes

- Source file: 16 MiB; text source/extracted document: approximately 1 million characters/UTF-8 text byte bound as enforced at each boundary.
- Scan: at most 250,000 accepted files and 4,096 MiB of accepted source bytes per `index` run by default (`--max-files`, `--max-mib` raise or lower them); exceeding a bound aborts and rolls back the corpus transaction.
- Binary extraction: at most 100 pages and 20 million pixels per raster/image; 120-second helper timeout; 8 MB protocol output cap, additional Rust page/text validation.
- Search: nonempty query up to 512 bytes; literal whitespace-delimited AND terms, not caller-supplied FTS operators. Limit 1–20 across text and records. Excerpts 500 characters; full chunks at most 2,000 characters.
- Hybrid: explicit local model directory; no entry cap (exact scan up to 50,000 stored entries, a BM25 shortlist of 1,000 beyond; see bench/experiments/RESULTS.md); request 64 MB, helper output 128 MB, 600-second direct-child kill/reap timeout, followed by the shared 256,000-byte CLI response cap. Python/native inference is not sandboxed or given a process memory limit. Model files are hashed locally for each invocation; manifest/helper/identity mismatches fail closed. A rebuild holds a write transaction throughout inference, so it can block other writers. No network download occurs in the inference helper.
- JSON responses: at most 256,000 bytes or a JSON error asking for a narrower query.
- Finding/validation import JSON: at most 64,000 bytes each; 1–16 evidence artifacts, each within the 16 MiB file bound. Checks are explicit and bounded. Large archives must be split or a smaller signed/hash manifest used; do not silently skip hashing them.

Statuses `empty`, `failed`, and `unsupported_or_failed` are not successful searchable extraction. The index response reports counts and up to 20 failure path/status details; `stats` reports processing-status totals. Empty images do not become successful empty documents. An empty page in an otherwise extractable PDF is recorded in the extractor coverage string; OCR transcription may contain errors, duplicates on mixed pages and imperfect reading order. Table/diagram semantics are not guaranteed.

A removed original yields `deleted` after a successful rescan; a changed original yields `stale` at retrieval before reindex. Missing/inaccessible sources report `missing_or_unreadable`. The last indexed text is not silently relabeled current. A selected chunk replaced by reindex is no longer retrievable under its old ID, rather than returning different text under the same ID.

## Backup and restore

Use `backup --output NEW_PATH`, not an ordinary copy of a live database. Online SQLite backup includes sources, FTS, authoritative findings and history coherently. Overwrite is refused. Keep snapshots separate from the live file, outside generic vault-sync mechanisms. The completed backup is SHA-256 verified by streaming, without the document-ingestion size bound. A regression test backs up and opens a database larger than 16 MiB.

To restore safely: stop writers, retain the current database and any journal files for diagnosis, and point the CLI at a verified snapshot under a **new path**. Opening the snapshot as the target database is the tested restore path. Check `stats`, run representative searches/gets and validate SQLite integrity using a separate trusted maintenance tool. No destructive in-place restore or database-delete command is exposed. Copy neither a live `.db` alone nor its transient journal into a running workspace.

Deleting/rebuilding the entire database destroys authoritative findings/history. Reindex selected folders to refresh derived content; do not use deletion as an index refresh. `rebuild-embeddings --model LOCAL_DIR` replaces only that model's derived vectors and preserves records/history; re-run after corpus/record changes to regain semantic coverage. New or changed items can still match lexical search before rebuilding. Hybrid scores are not confidence, never establish advice applicability or freshness, and do not implement no-answer abstention; see [measured risks](HYBRID.md). No automatic garbage collection or pruning of structured records exists.

## Evidence retention and honest scope

`tools/import_archperf.py` copies four specifically authorized BorgInvestor evidence files byte-for-byte into this repository, verifies SHA-256, validates ZIP CRCs, imports only the two `READY_CONFIRMED_SCOPED` candidates and checks original bytes again. It never opens the live run registry or modifies BorgInvestor. Exact source versions, original measurements, regressions and caveats are retained in the record payload. These imports remain **proposals**: overall campaign preservation is `NOT_CERTIFIED`, the subsequent live-registry writer is unknown, and production health is `UNKNOWN`. Synthetic scale results are not global performance claims; the actual small-workload regressions/cancellation trade-off remain visible.

The ordinary binary ingestion path snapshots one source into scratch, removes that scratch with its owning temporary directory, and leaves originals on disk. The helper launches with Python isolated mode. Provisioning dependencies downloads public packages/models; document ingestion itself uses local libraries and packaged models only. No agent runtime, Hermes profile, global MCP registration, service or Qdrant collection is installed/changed.

## Distribution and unresolved operational choices

No remote, distribution policy, service ownership or unattended schedule has been selected. The project is licensed Apache-2.0 and the shipped extraction path is permissive (PDFium via `pypdfium2`, RapidOCR). PyMuPDF is AGPL-3.0 and is confined to `requirements-eval.lock` for evaluation scripts only; do not move it back into the runtime locks or bundle it in a distributed product. Dependency versions are pinned in `Cargo.lock` and `requirements-extract.lock`, but a full RustSec/package vulnerability audit has not been run. There is no automatic updater. Keep the binary and its source/helper repository together or rebuild after relocation.

The attempted `AGENTS.md` creation was blocked by the protected-instruction approval timeout. It was not retried or bypassed. Canonical policy symlinks exist; the user/coordinator can separately approve a project agent-instruction file if desired.
