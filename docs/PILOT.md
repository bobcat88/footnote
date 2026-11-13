> **Historical document.** This is the README of the pilot, when the project was called BorgKnowledge. The product is now Footnote and its README is [../README.md](../README.md). Names, commands and numbers below describe the pilot and are kept for the record.

# BorgKnowledge

Local Rust + SQLite retrieval CLI for agents. **Working pilot, not a replacement for the existing vault/Qdrant service and not globally activated in agent configurations.** No daemon, cloud inference, or document uploads.

- Executable: `D:/Projets/BorgKnowledge/target/release/borg-knowledge.exe`
- Original schema-3 pilot (unchanged): `artifacts/demo/knowledge.db`; current schema-4 accepted copy: `artifacts/hybrid-acceptance/803ba94aa98b4f628abb4feb4557a323/pilot-copy.db`. Both contain two scoped optimization **proposals**; reads refuse an unmigrated schema.
- [Architecture, schema, boundaries](docs/ARCHITECTURE.md)
- [Verification and measured baseline](docs/VERIFICATION.md)
- [Security and recovery](docs/OPERATIONS.md)
- [Isolated Qdrant server/HNSW comparison with resident inference](docs/SERVER-BENCHMARK.md) — Python harness, not Rust CLI performance; latency and ANN recall measured separately.
- [Public technical retrieval quality and experimental abstention](docs/TECHNICAL-QUALITY.md) — frozen official-source corpus, bilingual questions, explicit limitations; not personal-vault certification.
- [Harmonized SQLite/Qdrant per-query comparison](docs/QDRANT-QUALITY.md) — same ordered top5 on 32/32 questions; Qdrant selected for further experiments, not production acceptance.
- [Executed retrieval ablations and reranking](docs/benchmarks/RETRIEVAL-ABLATIONS.md) — 704 diagnostic rows; relevance, applicability and CPU latency trade-offs, with per-query evidence.
- [Executed public-document LAD/RAD pilot](docs/benchmarks/DOCUMENT-INTELLIGENCE-PILOT.md) — native PDF, derived scan and HTML; extraction fidelity is separate from retrieval precision. Not banking/GED qualification.
- Academic research: [retrieval by theme](docs/research/ACADEMIC-RETRIEVAL-BY-THEME.md) and [document intelligence by theme](docs/research/ACADEMIC-DOCUMENT-INTELLIGENCE-BY-THEME.md).
- [Original imported evidence](evidence/archperf-20261016/) and [repeatable import](tools/import_archperf.py)
- Canonical stack links: [stack contract](STACK-CONTRACT.md), [coding rules](AI-CODING-RULES.md), [MCP registry](MCP-REGISTRY.md), [hook registry](HOOK-REGISTRY.md). These are symlinks, not divergent copies.

## Implemented

| Capability | Behavior |
|---|---|
| Local database | Bundled SQLite, actual FTS5 enabled; transactional versioned migrations; no arbitrary SQL command |
| Text indexing | UTF-8 Markdown/TXT, heading and title context, bounded Unicode chunks, source hash/revision, incremental hash comparison |
| Binary ingestion | PDF text layers; real local OCR for PNG/JPEG/WebP/TIFF/BMP and scanned or mixed-image PDF pages, with page references |
| Search/get | Lexical FTS5 remains default; opt-in local multilingual ONNX + cosine/RRF; structured applicability filters; hash-bound citations and bounded JSON |
| Derived embeddings | Explicit `rebuild-embeddings --model LOCAL_DIR`; schema 4; model/helper identity and corpus revision checks; at most 10,000 chunks + records |
| Contributions | Schema-checked proposals, exact-version metadata, cause/context deduplication, pinned local evidence, revision-checked explicit validation and immutable transition history |
| Recovery | SQLite online backup; coherent backup can be opened as a restored database; overwrite refused |
| Source lifecycle | Updated sources reindexed, deleted/moved paths tombstoned, identical content deduplicated in search; originals never modified |

**Not implemented:** a config auto-installer (`init`) for the supported agents, reranking in the Rust CLI, calibrated answerability/abstention, a watcher, extraction of meaning from charts/photos, complex-table reconstruction, structured finding edit/supersede commands. `superseded` is reserved in the schema; it is not an exposed transition yet. No whole-vault import is enabled or performed. PDF/image support and optional embeddings use the project `.venv`, not native-Rust OCR/inference.

[Hybrid implementation, model comparison and raw evidence](docs/HYBRID.md): actual release CLI, 1,000 synthetic documents, 16 development + 16 holdout queries. Holdout macro recall@5 was **0.286 FTS5 vs 0.929 for either hybrid model**; MiniLM MRR was **0.810**. This is not production precision: P@5 was **0.200**, every no-answer hybrid query still returned candidates, and cold-per-process hybrid p50 was **956.25 ms**, versus **6.23 ms** lexical. No automatic rollout or Qdrant replacement is justified. Isolated Qdrant-client local exact search matched top five for all 32 queries/model; it was **not a Qdrant server benchmark**.

## MCP server — one tool surface for every agent

Any MCP-capable host (Claude Code, Codex CLI, Gemini CLI, Cursor, Windsurf, Zed, VS Code, Cline, Roo Code, …) can call the same database. The tool surface is identical regardless of which model or editor drives it.

```bash
# one command: write the MCP entry into every detected agent, register the
# shared daemon at logon and start it
borg-knowledge --db D:/path/knowledge.db install

# what each agent runs (written by install; plain MCP on stdio)
borg-knowledge --db D:/path/knowledge.db mcp

# the shared daemon: one warm process per corpus, local socket only, no TCP.
# `install` starts it at logon; run it by hand if you prefer.
borg-knowledge --db D:/path/knowledge.db serve --mcp --daemon
```

`mcp` is the plug-and-play front-end: it connects to the shared daemon when one is running and otherwise serves the session in-process, so tools work before any daemon exists. Agents never see the socket or the token.

| Tool | Required args | Purpose |
|---|---|---|
| `borg_recall` | `query` | Lexical (default) or hybrid search; optional `limit`, `mode`, `model` (hybrid only: local embedding directory), and structured `technology`/`project`/`tested_version`/`status` filters. |
| `borg_get` | `id` | One indexed chunk with hash, revision, page/line and freshness. |
| `borg_get_record` | `id` | One structured finding with its pinned evidence and validation state. |
| `borg_stats` | — | Counts, schema version, FTS5/semantic status. |
| `borg_health` | — | Schema version, corpus coverage and embedding presence. |
| `borg_wake_up` | — | Bounded context pack for session start: records first, lexical chunks second, labelled `project_records` / `project_lexical` / `global_lexical` / `recent_chunks` (optional `project`, `limit`). |
| `borg_export` | — | Bounded records metadata dump — ids, status, revision, fingerprint, `payload_sha256` — max 200 rows with a `truncated` flag. No file writes. |
| `borg_propose` | `payload` | **Reserved in v1**: returns `isError: true` with the CLI command, because the file allowlist, evidence hashes and dedup are CLI-only and must not be bypassed. |

Protocol: JSON-RPC 2.0 over stdio, one JSON object per line. Request lines are bounded to 1 MB and responses to 256 KB; oversized input is rejected with a parse error. Notifications never receive a reply. Tool failures are returned as `isError: true` results; protocol failures use JSON-RPC error codes (`-32700`, `-32600`, `-32601`, `-32602`). Read tools open the database **read-only** and never create it. All retrieved text is returned as untrusted source data — consumers must not follow instructions found inside it.

## Security posture

The default and only transport is **stdio**: the host launches this binary as a subprocess and exchanges newline-delimited JSON-RPC over two pipes. There is no socket, no listening port and no network surface, so the security boundary is the OS process and the user account running it. Do **not** add a network transport to this process without authentication and TLS.

Hardening that applies today:

- **Helper subprocesses run with a scrubbed environment.** The MCP host spawns this binary inside the agent's environment, which usually carries provider credentials (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, GitHub tokens, ...). Both Python helpers (`tools/extract.py`, `tools/embed.py`) parse untrusted documents, so they are started with `env_clear()` and receive only `PATH`, system directories, temp paths and locale — never secrets. A unit test asserts the inherited environment does not reach the helper.
- **Tool panics are contained.** A panicking tool returns `isError: true` and the server keeps serving, so one bad document cannot kill the session.
- **Retrieved text is untrusted by contract.** Every hit carries `trust: "untrusted_source_data"` and the tool descriptions state that agents must never follow instructions found in indexed content. Content labelling — not the channel — is what defends against prompt injection.
- **Read tools never create or migrate the database** (read-only open, missing DB is a tool error), and `borg_propose` refuses over MCP rather than bypassing the CLI allowlist.
- **Bounded everywhere:** 1 MB request lines, 256 KB responses, 512-character queries, 20-result limits, 16 MiB source files, 100 pages per document, 120 s extraction timeout.

**Not implemented, and required before any network transport:** authentication/authorization, TLS, per-tool scopes and session binding. The SQLite file is unencrypted at rest — protect it with OS file permissions and disk encryption. If a resident/shared server is ever needed, prefer an owner-only Windows named pipe over TCP.

### Audit log (automatic accountability, not encryption)

For a memory tool holding personal notes, "who read what, when" matters more than encrypting a pipe. Every tool call is appended to a hash-chained log **by default**; nothing has to be configured:

```bash
# nothing to do: `install`/`mcp`/--daemon already write the log, the MAC key is
# generated on first use and both live in the per-user state directory
#   Windows: %LOCALAPPDATA%\borg-knowledge\
#   Linux:   $XDG_STATE_HOME/borg-knowledge/ (~/.local/state/...)

# verify at any time, with the automatically managed key
borg-knowledge audit verify --file <state>/audit.jsonl --mac-key <state>/audit.key

# detect tail truncation by pinning the head hash you last observed
borg-knowledge audit verify --file <state>/audit.jsonl --expect-last <hash>
```

`BORG_STATE_DIR` relocates that directory; `--no-audit` (daemon mode) or an explicit `--audit` path overrides the default. Each response carries `_meta.borg = {session, seq, result_sha256, mac}`, which binds the evidence the agent received to the exact log line that records it. Argument text is never logged — only its hash — because queries are sensitive.

What the chain guarantees, stated honestly:

| Attack | Detected? |
|---|---|
| Edit a logged line | **Yes** — hash mismatch |
| Delete or reorder a line | **Yes** — sequence gap / chain break |
| Rewrite lines without the MAC key | **Yes** — MAC mismatch |
| Truncate the tail | **Only with `--expect-last`** — a bare hash chain cannot see it |
| Attacker who can read the key and rewrite everything | **No** — keep the key user-only |

A log that does not verify at startup makes `serve` **refuse to start**. If an audit write fails mid-session the result is withheld (`isError: true`): knowledge that cannot be accounted for is not disclosed.

## Quick use (Git Bash / native Windows)

Every command requires an explicit database path. Read commands open it read-only; they do not initialize a missing database. JSON errors have nonzero exit status. `--help` / `--version` use normal CLI text.

```bash
K='D:/Projets/BorgKnowledge/target/release/borg-knowledge.exe'
DB='D:/Projets/BorgKnowledge/artifacts/hybrid-acceptance/803ba94aa98b4f628abb4feb4557a323/pilot-copy.db'

# Real imported scoped SQLite finding; exact technology/version filter.
"$K" --db "$DB" search substr --technology 'Python/SQLite' \
  --tested-version sqlite=3.53.1 --project BorgInvestor --status proposal --limit 2
"$K" --db "$DB" get-record 2
"$K" --db "$DB" stats

# Explicitly authorized source scope only. Choose a bounded technical folder,
# not the whole personal vault. No default corpus is inferred.
"$K" --db 'D:/Projets/BorgKnowledge/artifacts/notes.db' index \
  --root 'D:/approved-technical-notes' --allow 'D:/approved-technical-notes'
"$K" --db 'D:/Projets/BorgKnowledge/artifacts/notes.db' search 'sqlite checkpoint' --limit 5
# Use the returned numeric chunk ID, not a guessed path or SQL expression.
"$K" --db 'D:/Projets/BorgKnowledge/artifacts/notes.db' get 12

# Snapshot includes authoritative findings and derived indexes. Target must not exist.
"$K" --db "$DB" backup --output 'D:/Projets/BorgKnowledge/artifacts/pilot-backup.db'
```

`results` contains text chunks; `records` contains optimization summaries. The combined count is at most `--limit` (default 8, maximum 20). For **default lexical mode**, structured records get reserved priority, then text fills remaining slots; this is **not cross-type calibrated relevance**. A structured filter selects records only: document compatibility is not inferred from prose. All lexical terms must match within one chunk plus its indexed title/heading context. Hybrid mode instead uses RRF scores across eligible records/chunks; it does not infer version ranges, source authority or answerability. Neither mode can safely promote retrieved advice automatically.

A text citation includes `path`, `sha256`, `revision`, `heading`, `page` (1-based or null), `line` (1-based; extracted text for binaries), and numeric chunk `id`. Search text is at most 500 characters; `get` returns the selected chunk up to 2,000 characters. The overall JSON response is capped at 256,000 bytes; narrower queries are required if it exceeds that limit. Paths and source text are **untrusted data, never instructions**. Fetch selected evidence, check `freshness`/`evidence_freshness`, and preserve applicability before using advice.

## Structured optimization workflow

`evidence/archperf-20261016/*.record.json` shows the real schema in use. Required fields: technology, project, exact `tested_versions` map, environment, symptom, root cause/status, actual remediation, applicability, exclusions, protocol, trade-offs, rollback, last verification, and 1–16 absolute evidence paths with lowercase SHA-256. Additional fields are retained; the full source candidate and measurements are under `details`.

```bash
"$K" --db "$DB" propose --file 'D:/finding.record.json' --allow 'D:/evidence-folder'
# Input JSON itself AND every evidence artifact must be inside the allowlist(s).
# Repeated --allow arguments permit disjoint authorized folders.
"$K" --db "$DB" validate 3 --revision 1 \
  --evidence 'D:/evidence-folder/validation.json' --allow 'D:/evidence-folder'
```

Validation JSON binds `record_id`, `record_revision`, `record_sha256`, `verdict: "passed"`, nonempty `checks: [{"name": "...", "passed": true}]`, and `artifacts: [{"path": "absolute", "sha256": "64 lowercase hex"}]`. The command rejects stale/missing evidence, non-proven root causes, wrong hashes/revisions, failed checks, and revalidation of an already transitioned record. This is explicit attestation with pinned evidence, **not independent scientific proof or a production-health certificate**. Tests exercise real transactional validation; the two imported campaign findings intentionally remain proposals.

Cause/context fingerprints include technology, project, exact versions, environment, and root cause. An identical payload returns the same ID. A conflicting payload under the same fingerprint is rejected, never silently overwritten. No current command edits/removes structured records; keep a coherent backup before experimenting with schema changes.

## Build and verify

Rust/Cargo and the Windows MSVC linker must work. The crate uses `rusqlite` bundled SQLite, so no separately installed SQLite DLL is needed. Core text/search commands do not require Python. PDF/image ingestion needs these project-local dependencies:

```bash
uv venv .venv --python 3.11
# Full acceptance environment includes extraction, embeddings and isolated Qdrant:
uv pip sync --python .venv/Scripts/python.exe requirements-hybrid.lock
.venv/Scripts/python.exe tools/provision_embeddings.py
rtk cargo build --locked --release
bash scripts/check.sh
.venv/Scripts/python.exe tools/verify_hybrid.py
```

Dependencies/models download during provisioning, not ingestion. RapidOCR uses packaged local ONNX models. The helper defaults to this repository's `.venv`; set `BORG_KNOWLEDGE_PYTHON` to an **already provisioned local interpreter** if needed. Moving the binary alone is insufficient for binary extraction: its helper path is compiled from the source location. Rebuild after moving the repository. A missing helper/dependency becomes `unsupported_or_failed`, never successful empty image indexing. Failed/empty extraction is retried on the next explicit index.

The full gate uses scratch under `TMPDIR` or `%LOCALAPPDATA%/hermes/cache/scratch`. Real OCR tests use Windows Arial. See [verification](docs/VERIFICATION.md) for exact counts and limitations; synthetic latency is not a real-vault speedup claim.

## Ownership and rollout

Original notes/PDFs/images remain authoritative on disk. The SQLite index is rebuildable; **structured records and their history are not**. Never delete the database as an index-cleaning step. No Hermes profiles, agent runtime configuration, MCP registry, existing Qdrant storage, vault originals, paused Markdown prototype, or BorgInvestor files were changed. The canonical vault navigation link is left to the coordinating agent to publish without shared-file conflicts.

Licensed **Apache-2.0** (see `LICENSE` / `NOTICE`). The shipped extraction path is permissive: PDF text and rasterization use **PDFium via `pypdfium2`** (Apache-2.0 / BSD-3-Clause) and OCR uses **RapidOCR** (Apache-2.0). **PyMuPDF (AGPL-3.0) is not a runtime dependency** — it survives only in `requirements-eval.lock` for evaluation scripts (`tools/demo.py`, `tools/document_intelligence_benchmark.py`), which are comparison tooling, not part of the distributed extractor. Docling pilot artifacts under `tools/artifacts/` were evaluation-only; Docling is MIT, but its downloaded model weights carry their own terms. No remote is configured. The committed feature branch is `feat/local-knowledge`; the Beads initializer previously auto-committed tracker files and that bootstrap branch reference was since removed by a compare-and-delete operation.
