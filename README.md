<div align="center">

# Footnote<sup>1</sup>

**Local memory for AI coding agents. Every answer carries a footnote.**

Search your own notes and documents from Claude Code, Codex, Hermes or any MCP host.<br>
Each hit comes back with the file, the line, the content hash and whether the file has changed since.

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-A8281C)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-1B1915)](Cargo.toml)
[![MCP](https://img.shields.io/badge/protocol-MCP%20stdio-1B1915)](#connect-your-agent)
[![Offline](https://img.shields.io/badge/network-none-2F6B3A)](#security-model)

[Install](#install) · [Proof](#proof) · [How it works](#how-it-works) · [Connect your agent](#connect-your-agent) · [Security](#security-model) · [Reproduce](#reproduce-every-number)

</div>

---

```text
agent ▸ footnote_recall("why does the WAL checkpoint stall under write load")

¹  Checkpoint latency is amortized through batching; a long reader holds the
   WAL open and the checkpoint cannot finish.
   notes/sqlite.md · §WAL · line 42 · rev 4 · current · sha256 3fa9c1d07b2e

²  After a reader-heavy night, run a passive checkpoint before the batch job starts.
   ops/runbook.md · page 3 · rev 1 · stale (file changed since indexing) · sha256 9b0e77a41c58
```

<sup>1</sup> *Illustrative output. The fields are the real ones the tools return: `path`, `heading`, `page`, `line`, `revision`, `sha256`, `freshness`.*

## What it is

An agent that remembers is only useful if it can tell you where it read something, and whether that source is still true.
Footnote is a single Rust binary that indexes Markdown, text and PDF on your machine and serves them to your agent over MCP.

- **Cited.** Every hit is bound to the exact bytes it came from. If the file changed, the hit says `stale`. If it is gone, it says `missing_or_unreadable`.
- **Local.** One SQLite file, one search index, one embedding helper on your CPU. No account, no cloud, no telemetry, no network calls.
- **Read-only for agents.** An agent can recall and fetch. It cannot write to your corpus, and every call is appended to a tamper-evident audit log.
- **French and English.** Built and measured for mixed-language work, not as an afterthought.

## Proof

nDCG@10 on the official test sets of public benchmarks, measured on the shipped binary through one resident MCP session, the way an agent uses it.
"Earlier build" is this project before the retrieval work below, the same code path with every query word required to match.

| Benchmark (language) | Earlier build | BM25, published | Footnote |
|---|---:|---:|---:|
| SciFact, science claims (EN) | 0.017 | 0.665 | **0.747** |
| NFCorpus, nutrition and medicine (EN) | 0.189 | 0.325 | **0.368** |
| Alloprof, school questions (FR) | – | – | **0.478** |
| Syntec, business documents (FR) | 0.000 | – | **0.795** |
| French and English in one index, 11,462 documents | – | – | **0.489** |

How to read this, honestly:

- The jump from 0.017 to 0.66 on SciFact is one rule. The earlier build required every word of a question to match, so 295 of 300 questions returned nothing. Any-word matching with BM25 ranking is the published baseline, and Footnote reproduces it (0.663 against the published 0.665) before adding anything.
- On top of BM25, two small embedding models are blended with it. On held-out data, with 95% bootstrap intervals that all exclude zero, the blend beats the single-model version by +0.012 to +0.019 nDCG@10 ([experiments](bench/experiments/RESULTS.md)).
- We tested 15 other ideas, one at a time, and most of them did not help: query pruning, pseudo-relevance feedback, title boosts, adaptive weights, fewer dimensions. They are listed, with their numbers, in the same file. A change ships only if it clears a threshold written down before the run.
- We have **not** run other memory tools on these sets, so we make no claim against them by name. The comparison points are the ones with public numbers: published BM25 (BEIR) and published embedding-model scores (MTEB), reproduced in [`bench/anchors.json`](bench/anchors.json).

### Fast at 100,000 chunks

Two measurements, kept apart because they answer different questions.

**The shipped binary, end to end**, through one resident MCP session, every query encoded by both models, 300 real queries, median and 95th percentile:

| Stored chunks | Strategy | p50 | p95 |
|---:|---|---:|---:|
| 5,183 (SciFact) | exact scan | 20 ms | 23 ms |
| 40,018 | exact scan | 45 ms | 67 ms |
| 100,044 | BM25 shortlist of 1,000 | 52 ms | 79 ms |

The 40,018 and 100,044 rows use real text and real queries but **random document vectors** (encoding 100,000 chunks with two models takes hours and does not change the cost of a search), so they measure speed, not precision ([script](bench/experiments/scale_product.py), [rows](bench/sweeps/scale_product.jsonl)).

**The lexical index alone**, against SQLite FTS5, which most local tools use; median warm query on synthetic corpora cut from real documents:

| Documents | SQLite FTS5 | Footnote's index (Tantivy) |
|---:|---:|---:|
| 100,000 | 43 ms | **0.38 ms** |
| 1,000,000 | 423 ms (p95 1.7 s) | **3.0 ms** (p95 7.7 ms) |

At one million documents the Tantivy artifact is 304 MB versus 1,715 MB for the FTS5 database ([data](bench/dbbake/results.jsonl)). The FTS5 test stores document text while Tantivy stores identifiers and search structures without document text, so the 5.6× ratio is not a like-for-like index-only or complete-product storage comparison. These corpora are not judged: they show speed, not precision.

Up to 50,000 stored entries every vector is scored exactly, from a matrix decoded once per process. Beyond that, a BM25 shortlist of 1,000 chunks is re-scored from vectors fetched by id, so memory does not grow with the corpus and the query time grows slowly.

## How it works

```text
 notes, docs, PDF                                          your agent
        │                                                       ▲
        ▼                                                       │ MCP (stdio, read-only)
  ┌──────────┐   ┌─────────────────────────────────────────────┴───────────────┐
  │  index   │──▶│  SQLite (system of record)  ·  Tantivy BM25 (derived)       │
  │  chunk   │   │  chunks, hashes, revisions     any-word, stemmed, accents   │
  │  hash    │   │  int8 vectors from 2 local models, blended 50/50            │
  └──────────┘   └─────────────────────────────────────────────┬───────────────┘
                                                                │
        fuse:  0.55 · dense  +  0.45 · BM25   (z-scores over each top 100)
                                                                │
                                          cite: path · line · sha256 · revision · freshness
```

| Choice | Why | Evidence |
|---|---|---|
| Any-word BM25, English stemmer, accents folded | Natural questions rarely share every word with the answer | SciFact 0.017 → 0.663; the English stemmer beat per-language and combined stemming on a mixed French and English index |
| Tantivy for BM25, SQLite as the record | Comparable judged-set precision and 110 to 140 times faster lexical queries at 100k to 1M synthetic documents; artifact-size measurements use different stored content and do not compare complete databases | [bake-off](bench/DECISION.md) |
| Two embedding models, blended | A multilingual model (bekko-a8m) and an English one (gte-small) fail on different questions | +0.012 to +0.019 on four held-out sets, all intervals exclude zero |
| int8 vectors | A quarter of the memory, the same ranking | identical within 0.0004 nDCG |
| Exact scan up to 50,000 entries | Finds paraphrases and other-language matches that share no word with the question | covered by a test: a French question finds the English note |
| No language model in retrieval | Retrieval stays local, deterministic and cheap; the agent does the reasoning | every run is repeated and compared |

Models are public, pinned to a commit and checked against a SHA-256 before every use: [`bekko-embedding-v1-a8m`](https://huggingface.co/hotchpotch/bekko-embedding-v1-a8m) (multilingual) and [`gte-small`](https://huggingface.co/Xenova/gte-small) (English).

## Install

Requirements: Rust 1.85 or later, Python 3.12 (only for the embedding helper), about 200 MB of disk for the two models.

```bash
git clone https://github.com/bobcat88/footnote && cd footnote
cargo build --release                      # target/release/footnote
export PATH="$PWD/target/release:$PATH"     # this shell; use an absolute path for MCP
mkdir -p ~/.footnote

python -m venv .venv                       # the embedding helper (onnxruntime, tokenizers, numpy)
.venv/bin/pip install -r requirements-hybrid.lock          # Windows: .venv\Scripts\pip
python tools/provision_embeddings.py bekko-a8m gte-small   # public models, hash-checked

export DB=~/.footnote/knowledge.db
footnote --db $DB index --root ~/notes --allow ~/notes
footnote --db $DB rebuild-embeddings --model artifacts/models/bekko-a8m --model artifacts/models/gte-small
footnote --db $DB search "your question" --mode hybrid \
  --model artifacts/models/bekko-a8m --model artifacts/models/gte-small
```

The commands above use a POSIX shell. On Windows, add `target\release` to `PATH` for the session and create `%USERPROFILE%\.footnote` before indexing. Use absolute executable and database paths in MCP configuration.

Lexical search works without Python or models; hybrid search needs them. Re-run `index` whenever your files change (only changed files are re-read) and `rebuild-embeddings` afterwards (only new or changed chunks are embedded).

## Connect your agent

```bash
footnote --db $DB install      # registers Footnote in Claude Code, Codex CLI and Gemini CLI if it finds them
```

For any other MCP host, add this entry to its MCP settings:

```json
{ "mcpServers": { "footnote": { "command": "/path/to/footnote", "args": ["--db", "/path/to/knowledge.db", "mcp"] } } }
```

| Host | How |
|---|---|
| Claude Code | `install`, or `claude mcp add footnote -- /path/to/footnote --db /path/to/knowledge.db mcp` |
| Codex CLI | `install`, or `codex mcp add footnote -- /path/to/footnote --db /path/to/knowledge.db mcp` |
| Gemini CLI | `install` (writes `~/.gemini/settings.json`) |
| Hermes | add the same command and args under `mcp_servers:` in `config.yaml` |
| Cursor, Windsurf, Zed, VS Code, Cline | paste the JSON entry above into the host's MCP settings |

`install` never overwrites an existing entry and keeps a backup of every file it edits. An entry named `borg` from before the rename still points at the old executable; delete it.

### Tools the agent sees

| Tool | Purpose |
|---|---|
| `footnote_recall` | Search. `mode`: `lexical` (default) or `hybrid` with `model`; optional structured filters |
| `footnote_get` | One chunk by id, with hash, revision, page, line and freshness |
| `footnote_get_record` | One structured finding with its pinned evidence and validation state |
| `footnote_wake_up` | A compact starting context for a project |
| `footnote_stats`, `footnote_health` | Counts and state, so the agent can pick a strategy |
| `footnote_export` | Bounded export of records |
| `footnote_propose` | **Refused over MCP.** Writes need an explicit CLI command from you |

The previous `borg_*` tool names keep working as aliases.

## Security model

Footnote is built on the assumption that retrieved text may contain instructions aimed at your agent.

- **Untrusted by default.** Every retrieved passage is labelled as data. Nothing in a document can make the server do anything.
- **Read-only surface.** The MCP tools cannot write. Proposals are refused and logged.
- **No network.** The tool opens no outbound connection. The only download is the explicit, hash-checked model provisioning script you run yourself.
- **Private state.** The audit log, its MAC key and the daemon token live in a user-only folder created on first use.
- **Authenticated daemon.** The shared daemon listens on a local socket only, with a mutual HMAC handshake; agents never see the token.
- **Tamper-evident audit.** Every call is appended to a hash-chained, MAC-signed log. `footnote audit verify --file <audit.jsonl> --mac-key <audit.key>` verifies it offline.
- **Bounded everything.** Query length, result counts, helper output size and timeouts are capped; the embedding helper runs with a scrubbed environment.
- **Verified models.** Model files are hashed against the manifest at every helper start and every rebuild.

## Reproduce every number

Every figure above comes from a stored receipt in [`bench/receipts`](bench/receipts): dataset and split, pinned dataset hash, binary hash, harness hash, determinism check, latency, hardware. The protocol, the decision rules (written before the results) and the full experiment log are in [`bench/`](bench/README.md).

```bash
bench/.labenv/Scripts/python.exe -I bench/run.py scifact shipped-ensemble-mcp   # one dataset, the shipped binary
python bench/report.py                                                          # regenerate bench/RESULTS.md
```

## Limits, stated plainly

- The English stemmer is applied to every language. On the mixed French and English index it still beat per-language stemming, but a French-only corpus may want a French stemmer.
- Hybrid search needs a Python environment for the embedding helper. A pure-Rust encoder is planned; the helper keeps the model hash checks in one place until then.
- A reranker helps French questions (+0.04 nDCG@10 on Alloprof) but costs 1 to 3 seconds per query on a CPU; it is not in the default path. Reranking only the 60% of queries where the two best dense scores are close keeps 87% of that gain ([E12](bench/experiments/RESULTS.md)).
- Precision at 100,000 chunks and above was not measured: the judged benchmarks have at most 11,462 documents, and the large-corpus rows above measure speed only.
- PDF text layers and OCR for scans are supported through optional helpers; extraction quality is reported separately from retrieval quality.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE) for third-party components.
