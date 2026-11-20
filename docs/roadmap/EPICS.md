# Epics and user stories

Source of truth for the GitHub Issues board. One epic per area, stories under it. Every story
also meets the shared [Definition of done](../DEFINITION-OF-DONE.md). Format per story: title; who/what/why; `AC:` acceptance
criteria. `[done #PR]` marks stories already delivered (created closed, for history).

## E1 Memory
- [done #67] Agents keep their memory tools when the daemon stops. AC: relay falls back in-process; test kills the daemon.
- [done #68] Switch Memory, Library and Code on or off independently. AC: off removes tools at once; status shows each switch.
- [done #69 #71] Abstention gate for irrelevant memories: tried, failed on dev, stopped. AC: result recorded in MEMORY-ABSTENTION.md.
- Memory review and restore. As a user, I see near-duplicates, old session summaries and unused memories, and can forget or restore
  each. AC: `memory-review` changes nothing; `memory-restore` undoes forget and supersede; test covers the round trip.
- Memory review pass. As a user, I prune memories with the review list. AC: decisions recorded; nothing deleted, only retired.
- Activity shows which memories reached which prompt. AC: launcher Activity lists recent recalls with ids, scope, date; no prompt text.
- Abstention, second attempt (later). AC: fixture of agent-written memories with written no-answer prompts; rule before results.
- Memory rematch against mem0, Cognee graph, Hindsight, Graphiti, OpenViking, EverOS (after the review). AC: same LLM both sides.

## E2 Library
- [done #73 #75] One search over Memory, Library and Code (`footnote find`) with a corpus list. AC: each product answers alone.
- [done #80] Reuse vectors by text across files and versions. AC: only changed text re-embedded; never across models.
- Re-register existing corpora. As a user, my existing corpora appear in Search. AC: a command lists and adds corpora by path.
- Documentation search rematch on Sphinx docs (Flask, Python). AC: dev sets first; sealed final with alternated order.
- Measure how much two doc versions share (React 18/19, Python 3.12/3.13) before deciding shared storage (post-V1).
- Larger embedding model for documentation only. AC: measured on held-out before adoption.

## E3 Code
- [done #44 #53 #54] Code graph, MCP tool, default build, code map.
- SCIP indexers for TypeScript/JavaScript, Python, Java. AC: each indexer security-reviewed; `code` answers on a sample repo.
- Tree-sitter pass for languages without SCIP. AC: precision measured against a compiler-graded set.
- Inputs and outputs map (files, config, env vars, tables, routes) shown on demand. AC: split view in the launcher.
- Whole-graph "cool view" toggle beside the useful map. AC: off by default; reduced motion respected.
- Switch from external code-graph MCP tools once Footnote Code covers its uses. AC: comparison recorded.

## E4 Launcher
- [done #55 #59 #60 #61] Overview, Code, Projects, Models, Activity, Usage screens and mini mode.
- [done #72] Product switches on the Overview cards. [done #74] Search screen. [done #78] Install a model from the window.
- [done #79] Tray: left click opens mini, menu opens either window or quits; closing hides.
- Independent critic review over every screen. AC: critic report; fixes merged.
- Icon family: one icon per product. AC: Footnote, Library, Memory, Code, Alias icons in the app and tray.
- Quiet support link (needs a support page). AC: one link in the footer; no pop-ups.
- Linux and macOS builds of the launcher. AC: builds and opens on each.
- Phones and tablets over LAN or tailnet with one-time QR pairing (later). AC: per-device scoped sessions, private addresses only.

## E5 Installer and updates
- Windows installer. As a new user, I install Footnote One in one step. AC: clean Windows profile; signing status stated.
- Update and rollback. AC: update keeps data; rollback to the previous binary works; uninstall documented.

## E6 Personal memory encryption
- [done #65] Encrypted personal store (experimental): SQLCipher, recovery key and QR, Windows key store, backup and restore.
- Wire encryption into hooks, MCP, daemon, status and installer. AC: encrypted memory works end to end behind a switch.
- Migrate live memory on a copy with rollback. AC: rehearsed; opt-in switch only after explicit user confirmation.
- Recovery in practice: QR saved in a password manager, restore on another computer. AC: both rehearsed.
- Crash and orphaned-credential recovery (CredWrite race). AC: tested failure paths.
- Derived-data leaks (plain index files, temp files, logs). AC: audit against the threat model; keyword index in memory.
- macOS and Linux key stores. AC: same tests as Windows.
- Independent security review. AC: findings recorded; never claimed passed without it.

## E7 Personal corpus — plan in PERSONAL-CORPUS-PLAN.md, each step user-gated
- Sensitivity triage tool. AC: classes and counts only; secrets flagged; nothing read by agents.
- OCR benchmark (French, accents). AC: character error rate per engine on an approved sample.
- Encrypted Library corpus. AC: no plaintext index files; deletion purges text, vectors and index.
- Conversion pipeline: originals in secure storage, editable Markdown linked by SHA-256. AC: tested on a copy of a subset.
- Migrate an approved subset, then the rest folder by folder. AC: originals preserved; rollback rehearsed.

## E8 Alias
- [done, Nov 16] French identifier rules and Privacy Filter in the binary, stable pseudonyms.
- Underline-and-explain prototype. AC: hover explains type, GDPR rule and what to write instead, in French.
- Evaluation set v2, GLiNER2-PII only if names need margin. AC: held-out comparison.
- Move Alias out of the free repository into Enterprise. AC: free build has no Alias code.

## E9 Usage and savings
- [done #56 #70] Token usage per day, agent and model; measured command output savings.
- Limits and reset timers (5-hour, weekly). AC: local reads; undocumented endpoints optional and off by default.
- Other agents (Grok, DeepSeek Harness, Gemini). AC: counts match an independent count.
- Footnote's own lean-output savings measured. AC: sizes before and after recorded per call.

## E10 Company deployment (later)
- Access control and data tiers: scope and classification on documents and memories, filtered before ranking.
- Compliance kit: DPO, IT security, compliance and works-council packs.
- Cost per 1,000 users (RAM, CPU, time per hook and MCP call) on an idle machine.
- Shared daemon throughput: a lock per model and a pool of query sessions. AC: same load bench, identical rankings.

## E11 Engineering
- GitHub Actions CI green on PRs.
- Changelog through git-cliff (`CHANGELOG.md`).
- Clean old test entries from metrics.jsonl and audit.jsonl during pre-release checks. AC: only real entries remain.
- Prune old `footnote.exe.replaced-*` binaries safely with explicit user confirmation.
- Issue tracker consolidated in GitHub Issues with Epic nesting.

## E12 Benchmarks (when a decision needs them)
- Model matrix (licence × hardware × language) and setup playbook.
- GGUF embedding benchmarks and GPU acceleration matrix.
- Storage backends: SQLite + Tantivy against Postgres + pgvector, LanceDB, HelixDB, Omnigraph.
