# Code graph study: plan

Written 14 November 2026, before any hands-on run. Status: desk study approved; hands-on gated by explicit review.

## Why this study

Footnote Code is the code part of one engine, not one more tool next to the others. The question is not "which of the 10 to 20 code-graph
servers to install" but "what does Footnote Code need, and which ideas from them are proven". Constraints that come from the Footnote vision:

- **One engine, one MCP server, a small front door.** Every tool an MCP server lists costs prompt tokens on every turn, and overlapping
  servers make the agent hesitate. Modules (Library, Memory, Code, the project map) switch on or off; one question gets one cited answer
  from every module that has something, instead of several servers each fetching the same context.
- **Sobriety:** fewest calls and tokens per answered question; performance measured, not claimed.
- **Every language visible:** structure where a parser exists, exact references where a compiler-grade indexer exists, and Footnote
  Library's keyword and meaning search as the fallback for any text, config or document.
- **Local, cited, read-only for agents,** like Library and Memory; engine-agnostic (any LLM, local or cloud).
- **Works at three scales:** solo, small team on a free engine, company after a needs study.

## Candidates

Studied in depth (six): **codebase-memory-mcp** (MIT; C; 162 tree-sitter grammars plus language-server types; SQLite), **Graphify**
(Apache-2.0; code and docs in one browsable graph), **GitNexus** (PolyForm Noncommercial; impact, tracing, Cypher), **Serena** (GPL-3.0
since v2.0.0; live language servers, no stored graph; can edit), **CodeGraph** (MIT; Rust core; one tool by default; file watcher),
**repowise** (AGPL-3.0; graph plus git history: ownership, hotspots, health, close to the project map and roadmap ideas).

Noted for specific ideas only: **jCodeMunch** (fetch by symbol and byte offset; paid for companies), **code-review-graph** (PR blast
radius), **CodeGraphContext** (graph database), **tokensave**, Aider's repository map (tree-sitter plus ranking to fit a token budget).

## Licence rule for this study

Ideas, documented behaviour and published measurements can be learned from any of them. Code is reused only from MIT or Apache-2.0
projects, with attribution. No code from GPL, AGPL, PolyForm Noncommercial or custom-licence projects goes into Footnote (Apache-2.0);
for those, only their documentation and observed behaviour are studied.

## Part 1: desk study (now)

For each of the six, from its code and docs, without running it:

1. How the graph is built: parser, name resolution, type information, what an edge means, confidence per edge.
2. Languages: list, and which get exact references versus name matching.
3. Storage and freshness: database, size, incremental update (watcher, git hook, manual).
4. Agent surface: number of tools listed by default, tokens of their schemas, how answers are shaped (whole files, symbols, byte ranges).
5. Beyond code: docs, infrastructure files, git history, multiple repositories.
6. Terms: licence, telemetry default, paid tiers.
7. One or two ideas worth taking, one or two to avoid.

Output: one table and a short verdict per tool, in this folder.

## Part 2: hands-on (gated)

Each tool installed only after the usual review (real upstream, code read, no install hooks run), in an isolated environment.

- **Repositories (three languages):** Footnote (Rust and Python) and representative TypeScript/JavaScript/Python projects.
- **Accuracy:** references found per symbol against a compiler-grade oracle (rust-analyzer SCIP for Rust, scip-python, scip-typescript),
  as precision and recall, on 50 symbols per repository drawn at random before the run.
- **Tokens:** the same ten questions per repository, asked through the same agent and model with one server at a time: tool-schema tokens
  per turn, total tokens, tool calls, and whether the answer is right (graded against the oracle or by reading the code).
- **Cost:** index build time, index size, update time after a one-file change.

## Decision rule for Footnote Code (written now, applied after part 2)

Footnote Code is built only to meet all of these on the three repositories:

1. Reference recall at least the best studied tool's minus 0.05, and precision at least 0.95 where a compiler-grade indexer exists.
2. Fewer tokens per correct answer than every studied tool on the ten questions.
3. Code adds at most two tools to Footnote's MCP front door (today 10 tools for Library and Memory), and their schemas cost fewer
   prompt tokens per turn than the leanest studied tool's default set.
4. Index build no slower than the median studied tool; update after a one-file change under one second.

If no design can meet them, the study says so, and the best permissive tool is recommended alongside Footnote instead.
