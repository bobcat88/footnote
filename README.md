<div align="center">

# Footnote<sup>1</sup>

**Local memory for AI coding agents. Every answer carries a footnote.**

Search your own notes and documents from Claude Code, Codex, Hermes or any MCP host.<br>
Each hit comes back with the file, the line, the content hash and whether the file has changed since.

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-A8281C)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-1B1915)](Cargo.toml)
[![MCP](https://img.shields.io/badge/protocol-MCP%20stdio-1B1915)](#connect-your-agent)
[![Network](https://img.shields.io/badge/network-only%20when%20asked-2F6B3A)](#security-model)

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
- **Local.** One SQLite file, one search index, embeddings computed inside the binary on your CPU. No account, no cloud, no telemetry. The network is used only by two commands you run: `footnote models install` (hash-checked model download) and `footnote crawl` (a documentation site you name).
- **Your documents are read-only for agents.** An agent can recall and fetch. It cannot write to your corpus; it can only add to its own memory database. Every call is appended to a tamper-evident audit log.
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

Requirements: Rust 1.92 or later, about 200 MB of disk for the two models. Python 3.10+ is used only to download the models once and for PDF extraction and OCR; search, embeddings and the agent memory hook need none.

```bash
git clone https://github.com/bobcat88/footnote && cd footnote
cargo build --release                      # target/release/footnote
export PATH="$PWD/target/release:$PATH"     # this shell; use an absolute path for MCP
mkdir -p ~/.footnote

python tools/provision_embeddings.py bekko-a8m gte-small   # one-time download of the public models, hash-checked

export DB=~/.footnote/knowledge.db
footnote --db $DB index --root ~/notes --allow ~/notes
footnote --db $DB rebuild-embeddings --model artifacts/models/bekko-a8m --model artifacts/models/gte-small
footnote --db $DB search "your question" --mode hybrid \
  --model artifacts/models/bekko-a8m --model artifacts/models/gte-small
```

The commands above use a POSIX shell. On Windows, add `target\release` to `PATH` for the session and create `%USERPROFILE%\.footnote` before indexing. Use absolute executable and database paths in MCP configuration.

Lexical search works without Python or models; hybrid search needs them. Re-run `index` whenever your files change (only changed files are re-read) and `rebuild-embeddings` afterwards (only new or changed chunks are embedded).

**Update and rollback.** `footnote update --from <new footnote binary>` checks that the new binary runs, keeps the current one beside it as `footnote.exe.replaced-<time>` (only the newest is kept) and puts the new one in place, even while agents are using it. `footnote rollback` puts the previous one back. Your data lives in the state folder (`%LOCALAPPDATA%\footnote` on Windows, `~/.local/state/footnote` elsewhere, or `FOOTNOTE_STATE_DIR`), apart from the binary, so neither touches it.

**Uninstall.** `footnote uninstall --memory` removes the memory hook and MCP server from every agent (memories and settings are kept). Then delete the binary, and the state folder if you no longer want your data, indexes and models.

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
| `footnote_recall` | Search. `mode`: `hybrid` by default when the corpus has vectors from installed models (else `lexical`), or forced; optional `model` and structured filters |
| `footnote_get` | One chunk by id, with hash, revision, page, line and freshness |
| `footnote_get_record` | One structured finding with its pinned evidence and validation state |
| `footnote_wake_up` | A compact starting context for a project |
| `footnote_stats`, `footnote_health` | Counts and state, so the agent can pick a strategy |
| `footnote_export` | Bounded export of records |
| `footnote_propose` | **Refused over MCP.** Writes need an explicit CLI command from you |
| `footnote_remember`, `footnote_forget` | Agent memory only: add or retire a memory in the memory database. Refused on a document corpus |
| `footnote_code` | Code: a symbol's signature, place, callers and callees, from the compiler's index of the project |

The previous `borg_*` tool names keep working as aliases.

### Agent memory

Besides your documents, Footnote can remember what an assistant learns while working with you: your preferences, project decisions and dates, problems and their fixes, what each session achieved. Memories live in their own database (`memory.db` in Footnote's per-user state folder), never in a corpus, and are searched with the same hybrid engine.

`footnote install --memory` sets it up for every detected agent: Claude Code, Codex, Grok and DeepSeek Harness. It creates the memory database, registers its MCP server, adds the hook and a `/remember` command, backs up each file it edits, and leaves existing entries alone. The agent itself writes its memories; no second model reads your conversation. A hook (`footnote hook`, [src/hook.rs](src/hook.rs)) organizes it:
- at session start it injects your preferences and the project's memories;
- on each prompt it adds the memories relevant to it;
- every 10 prompts, at the end of a turn, it asks the agent once to save what it learned with `footnote_remember`, and `/remember` does the same on demand.

Nothing leaves the agent you already use: the memories are written by that agent, in its own session, into a local file. Settings (`memory.json` in the state folder) are documented in [src/hook.rs](src/hook.rs), the design in [docs/AGENT-MEMORY-DESIGN.md](docs/AGENT-MEMORY-DESIGN.md).

**Encrypted memory (experimental, Windows, builds with `--features encryption`).** `footnote personal init --store <folder>` creates
an encrypted store (SQLCipher; the key in Windows Credential Manager, a recovery key shown once as text or QR for your password
manager). `footnote personal use --store <folder>`, then `footnote install --memory`, makes the hook, agents and launcher use it;
`footnote personal stop-using` goes back to the plain database, leaving both in place. What it protects and what it does not is in
[docs/roadmap/PERSONAL-ENCRYPTION-DESIGN.md](docs/roadmap/PERSONAL-ENCRYPTION-DESIGN.md). One limit matters in daily use: the
memories an agent receives are in its prompt, so the agent's own transcripts (for example under `~/.claude`) keep them in plain
text; encryption protects Footnote's store, not what agents write down.

### Usage

`footnote usage --days 7` counts the tokens your agents used per day, agent and model, from the session files Claude Code and Codex already keep on your computer, and what they would cost at list API prices (a subscription pays a flat fee instead). Only counts are stored; no prompt or answer text is copied. Prices: a table in [src/usage.rs](src/usage.rs), dated, from the providers' pricing pages. Each row splits that price into cache reads, cache writes, uncached input and output. `footnote usage --file <session.jsonl>` prices one Claude Code or Codex session on its own.

Its `limits` part shows the 5-hour and weekly limits that have not reset yet, with the share used and the reset time. It reads the same files: Codex writes the share used on every turn, while Claude Code writes a limit only when you hit it. No account endpoint is called.

### Code graph

Footnote Code answers "who calls this, and what does it call" from the compiler's own index, not from a guess. Each answer also lists, under `related`, the memories and Library passages that name the symbol as code. It picks the indexer from the project's files and says which one is missing:

| Project | Marker | Indexer to install |
|---|---|---|
| Rust | `Cargo.toml` | `rustup component add rust-analyzer` |
| TypeScript, JavaScript | `tsconfig.json` (Footnote never writes one into your project) | `npm install -g @sourcegraph/scip-typescript` |
| Python | `pyproject.toml`, `setup.py`, `setup.cfg` or `requirements.txt` | `npm install -g @sourcegraph/scip-python` |

Rust in a subfolder (a Tauri app's `src-tauri`, `crates/<name>`) gets its own rust-analyzer run beside the top folder's indexer; folders starting with a dot and vendored copies are skipped, and the answer's `roots` says which indexer read which folder.

On Windows, keep Node.js and npm's global launcher folder on `PATH`. Footnote loads the global Python indexer through Node
with a temporary compatibility fix for its invalid path-separator regex ([upstream PR #224](https://github.com/sourcegraph/scip-python/pull/224)).
The installed package is never edited. A fixed upstream regex is left unchanged; the native constructor is restored after
loading. Tested with scip-python 0.6.8 on Windows, including cross-file references and project paths with spaces.

```sh
footnote code-index .          # build the graph of the project here (seconds; rebuild after big changes)
footnote code lookup           # the symbol `lookup`: signature, file and lines, callers with call-site lines, callees
footnote code-map               # architecture map: folders as blocks, links between them, outside packages used
footnote code-impact <name>     # before a change: callers up to 3 levels, tests reached, files and memories affected
footnote stack [path]            # the libraries the project uses: manifests and lockfiles, else imports, else one question; recorded in Memory, a re-run shows only what changed
footnote stack write [path] [--yes]  # when the stack came only from imports: show the missing dependency file (requirements.txt, package.json, go.mod); --yes writes it, never over an existing file
```

`footnote stack` reads Cargo.toml/Cargo.lock, package.json/package-lock.json, pyproject.toml/requirements*.txt/poetry.lock/uv.lock and go.mod (versions are exact when a lockfile or `==` pins them, `exact: false` otherwise); with none of those it reads the imports (standard library and your own modules left out, no versions). Each library carries `name`, `version`, `ecosystem` and `source` (`manifest`, `import` or `user`). An empty folder gets `needs_answer` with one question; answer with `--library pypi:flask@3.0.0`. It reads files only and uses no network. When the stack came only from imports the result carries a `suggestion` per ecosystem: the dependency file, its proposed content and why it helps (reproducible, teammates and CI, exact versions for Footnote docs). `footnote stack write [path]` prints it and writes nothing; add `--yes` to create it. It never overwrites an existing file and records the write in project memory. Import names map to package names (`cv2` to `opencv-python`, `PIL` to `pillow`, `yaml` to `pyyaml`); versions stay unpinned unless the package is installed in `node_modules` or a `.venv`. For Rust, Cargo.toml is explained, not written (it needs a package); for Go only the module line is written. Agents get the stack, read-only, with `footnote_code` `stack: true`.

Agents get the same answer through `footnote_code`; files changed since indexing come back in `stale`. The tool reads only projects Footnote has recorded (every folder an agent session starts in) and refuses any other folder. Index only code you trust: for Rust, rust-analyzer builds the project, which runs its build scripts and procedural macros (scip-typescript and scip-python only read the code; the parser graph runs no project code). Design: [docs/CODE-GRAPH-DESIGN.md](docs/CODE-GRAPH-DESIGN.md).

### Shared documentation cache

`footnote docs add <start-url> [--site NAME] [--version V]` indexes a documentation site once per machine into `<state>/docs/<site>/<version>.db` and lists it in `footnote corpora`, so Search finds it from every project. Asking again for the same site and version reuses the database without crawling; a different version creates a new one. `--from <folder>` indexes a folder of Markdown instead of crawling. Site and version are lowercased and reduced to `a-z 0-9 . _ -` (anything else becomes `-`, leading dots are dropped), so a name cannot leave the cache folder. Old versions are kept until `footnote docs prune`, which deletes, per site, every version older than the newest (numbers compare as numbers, so 10.0 is newer than 9.2; a non-numeric version such as `latest` counts as newest). `footnote docs list` shows site, version, pages, size and date.

Keyword first, embeddings later: `docs add` returns as soon as the crawl and keyword index are built, so Search answers within seconds (by keywords, as for any corpus without vectors); it then starts one detached worker that embeds the library in the background (`docs.embed` job, progress in `footnote jobs`) and Search upgrades to hybrid by itself when the vectors are committed. `footnote docs ingest <url>...` does this for several libraries: each is searchable once indexed, and one worker embeds them in order, smallest first and the largest last; a library that fails does not stop the others. `footnote docs list` shows each corpus as `keyword-only`, `embedding n/m` or `ready`. Models are `--model <folder>` (repeatable) or the ones `footnote models use` selected; with none, the corpus stays keyword-only. `--no-embed` skips the background step. A worker that dies leaves the keyword corpus usable (its job shows as interrupted); run `docs add` again for the same site and version to embed it again. Embedding is one transaction per library, so a killed run keeps no vectors and starts over. A crawl that finds no pages is an error, not an empty corpus.

Prebuilt bundles (a ready index instead of a crawl): `footnote docs licence <library> --name MIT --redistributable true --url <docs site>` records the source licence beside the cache entry. `footnote docs bundle <library> [--version V] --key <private-key-file> [--out F]` builds one signed file (a manifest with library, docs site URL, version, indexing date, page count, licence name, redistributable flag, the database's sha256 and the embedding model id, a signature over the manifest, then the database), and refuses when no licence is recorded or it does not allow redistribution. `footnote docs install <file-or-https-URL>` accepts a bundle only if the ed25519 signature verifies against the key built into Footnote (or one you add with `--trust-key <hex>`) and the database's sha256 and page count match the manifest; any mismatch is rejected before anything reaches the cache, the library and version must already be plain cache names (no path can leave `<state>/docs`), an installed version is never overwritten, and a URL download stops at 1 GiB. An installed bundle is listed by `docs list` and `corpora` like any other. Signing keys: `footnote docs keygen <file>` makes one; the private key stays outside every repository and the public half is pinned in `src/bundle.rs`. A downloaded index is data and private like the rest of Footnote. No bundles are shipped or hosted yet.

### Desktop window

`apps/launcher` is Footnote One's window (Tauri 2): an overview of the three products with their on/off switches, one search
over Memory, Library and Code, your projects and connected agents, the code map and symbol view, models (installed from the
window, with size, licence, progress and cancel), memory activity and token usage, plus a small always-on-top mini mode. It runs
only a short list of `footnote` commands: reads, a product's switch and a model install. It lives in the tray: a left click opens
the mini panel, the menu opens either window, and closing a window hides it; "Quit Footnote" in the tray menu quits. Build steps: [apps/launcher/README.md](apps/launcher/README.md).
Windows only so far; no installer yet.

## Security model

Footnote is built on the assumption that retrieved text may contain instructions aimed at your agent.

- **Untrusted by default.** Every retrieved passage is labelled as data. Nothing in a document can make the server do anything.
- **Read-only documents.** No MCP tool can write to a document corpus. The only write tools add or retire agent memories, and they refuse any database that holds documents before touching it. Proposals are refused and logged.
- **Network only when asked.** Search, memory and the MCP tools open no outbound connection. Three commands you run do: `footnote models install` downloads a model and checks its hash, `footnote docs add <url>` crawls a site the first time (see below), and `footnote crawl <url>` fetches pages of that one site (same host, under the start path, http or https; robots.txt rules and `Crawl-delay` honoured; at most `--max-pages`, 1000 by default; up to 8 requests in flight, starting at 2 and halving on 429, 503 or timeouts; 30 s per request).
- **Private state.** The audit log, its MAC key and the daemon token live in a user-only folder created on first use.
- **Authenticated daemon.** The shared daemon listens on a local socket only, with a mutual HMAC handshake; agents never see the token.
- **Tamper-evident audit.** Every call is appended to a hash-chained, MAC-signed log. `footnote audit verify --file <audit.jsonl> --mac-key <audit.key>` verifies it offline. A chain proves that no entry was changed or removed in the middle; it cannot show that the newest entries were cut off the end. To catch that, keep a copy of the last verified entry (or its hash) somewhere the agent cannot write.
- **Bounded everything.** Query length, result counts, text sizes, helper output size and timeouts are capped; the PDF helper runs with a scrubbed environment.
- **Verified models.** Model files are hashed against the manifest every time a process loads a model, and loaded from the bytes that were hashed.

## Reproduce every number

Every figure above comes from a stored receipt in [`bench/receipts`](bench/receipts): dataset and split, pinned dataset hash, binary hash, harness hash, determinism check, latency, hardware. The protocol, the decision rules (written before the results) and the full experiment log are in [`bench/`](bench/README.md).

```bash
bench/.labenv/Scripts/python.exe -I bench/run.py scifact shipped-ensemble-mcp   # one dataset, the shipped binary
python bench/report.py                                                          # regenerate bench/RESULTS.md
```

## Limits, stated plainly

- The English stemmer is applied to every language. On the mixed French and English index it still beat per-language stemming, but a French-only corpus may want a French stemmer.
- A reranker helps French questions (+0.04 nDCG@10 on Alloprof) but costs 1 to 3 seconds per query on a CPU; it is not in the default path. Reranking only the 60% of queries where the two best dense scores are close keeps 87% of that gain ([E12](bench/experiments/RESULTS.md)).
- Precision at 100,000 chunks and above was not measured: the judged benchmarks have at most 11,462 documents, and the large-corpus rows above measure speed only.
- PDF text layers and OCR for scans are supported through optional helpers; extraction quality is reported separately from retrieval quality.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE) for third-party components.
