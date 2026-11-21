# PRODUCT
Last reviewed: 2026-11-21 · Owner: Footnote maintainers

## 1. Product family & editions
- **Footnote One (Apache-2.0, this repository):** one local Rust binary + MCP server + optional Tauri desktop Launcher covering four free products, each toggleable independently:
  - **Footnote Library:** hybrid search (Tantivy BM25 + local int8 embeddings) over local notes, documents, and crawled library documentation, with file, line, hash, and freshness on every result.
  - **Footnote Memory:** persistent per-machine agent memory (preferences, decisions, fixes, session handoffs) with automatic hooks across Claude Code, Codex, Gemini CLI, Grok, and Antigravity.
  - **Footnote Code:** compiler-grade (SCIP) and tree-sitter code graph for definitions, callers, callees, impact radius, file outlines, and architecture maps.
  - **Footnote Lean:** preservation-first command-output compaction (`footnote run` / `footnote hook lean`) and per-session token/cost usage accounting (`footnote usage` / `footnote gain`).
- **Footnote Enterprise (separate private commercial repository):** company-only modules — SSO/RBAC, multi-team federated indexes, compliance kits, and Footnote Alias (local PII pseudonymization). Never bundled into the free engine (`docs/roadmap/EDITIONS.md`).

## 2. Users and context
- Primary user: a developer or technical lead who works daily with AI coding agents (Claude Code, Codex, Cursor, Gemini CLI, Antigravity, or any MCP host), experienced, careful about what leaves their machine.
- Context of use: CLI, MCP stdio server, desktop Launcher (`apps/launcher`), and static product site (`site/`, mobile floor 390 px, desktop 1440 x 900). English first; French and multilingual corpora are first-class cases.

## 3. Top three tasks
1. Understand in one screen what Footnote One does and whether it beats existing local memory and search tools — success looks like a visitor who can say "local memory, docs, and code graph with cited, checkable answers, measured on public benchmarks" after 20 seconds.
2. Build, download models (`footnote models install`), and connect it to their agent (`footnote install --memory`) in under two minutes with exact commands on screen.
3. Verify trust without asking anyone — where data lives, what is read-only vs writable, what network calls exist (explicit model downloads and doc crawls only), and the benchmark receipts (`bench/INDEX.md`) behind every claim.

## 4. Tone
- Is: plain, exact, quietly confident, documentary.
- Is not: hyped, cute, "magical", vague about numbers.

## 5. Success metric
A visitor reaches the GitHub repository from the landing page ("View on GitHub") and connects one agent harness on a clean machine in under two minutes.

## 6. Non-goals
- Not a cloud service, not a hosted vector database, not a chat UI.
- No accounts, no telemetry, no silent outbound network calls.
- No claim without a receipt: every number carries its dataset, split, conditions, and source file in `bench/INDEX.md`.

## 7. Accessibility floor
WCAG AA. Keyboard-complete. Reduced motion honoured (everything rests, nothing stays hidden). Text contrast measured in `DESIGN.md`. Exceptions: none.
