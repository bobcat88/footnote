# Agent memory: how competitors do it, and a proposal for Footnote

14 November 2026. Research note before any code. Footnote today serves a **knowledge corpus** (documents, first ingestion, later sync). This note is about the second product: **agent memory**, what an assistant should remember about the user and the work across sessions. Sources: ICM 0.11.5 source and docs (`bench/.cache/icm-source`), mem0's prompts (`bench/.venv-mem0`, installed version), and the public docs of Hindsight, Zep/Graphiti, Letta, LangMem and claude-mem.

The four open questions are what to store, how, how often, and how to make sure the agent uses it.

## 1. What the others store

They converge on the same few kinds, under different names:

| Kind | Example | ICM | mem0 | Hindsight | LangMem / Letta |
|---|---|---|---|---|---|
| **Preference** (about the user, global) | "answers in French, short, no jargon" | topic `preferences`, importance critical | personal preferences | observation | semantic "profile" / core memory block |
| **Fact or decision** (about a project or the world) | "auth uses short-lived JWTs"; "v1.2 released 2026-11-03" | `decisions-<project>` | facts, plans | world fact | semantic "collection" |
| **Fix** (problem, cause, solution) | "Cargo not found: Rust is in Program Files, not on PATH" | `errors-resolved` | (not a category) | experience | episodic "episode" (situation, reasoning, action, result) |
| **Session summary** | "14 Nov: benchmarks finished, Cognee indexes deleted" | memories extracted at compaction | | experience | recall memory (Letta) |
| **Consolidated belief** | "this user always wants relative % gaps" | `consolidate` | | observation, opinion with confidence | sleep-time agent rewrites blocks |

All of them say what **not** to store: content readable in the code or documents, git history, temporary state of the current task (ICM's own agent instructions say this word for word, and so does Claude Code's built-in memory).

Two details matter for quality:

- **Dates are made absolute when written.** mem0's newest prompt: "'User went to Paris last week' is useless 6 months later"; every relative date is resolved against the conversation date. Zep goes further and stores, for each fact, when it was true and when it was learned, and marks a contradicted fact invalid rather than deleting it.
- **Memories are self-contained sentences.** Each one must make sense alone, years later, without the conversation around it.

## 2. How memories get written

Three patterns:

1. **The agent calls a store tool itself** ("hot path"). ICM, mem0 and LangMem all offer it. It is cheap, but agents forget to call it, and that is why everyone adds 2.
2. **Hooks hand the transcript to an extractor** ("background"). An LLM reads the recent turns and outputs memories:
   - ICM calls the user's own LLM command-line tool (`claude -p` with Haiku, no tools, no saved session) when a session ends or before compaction, with a keyword-based extractor as the fallback.
   - claude-mem runs a second agent beside the session.
   - mem0 runs an extraction LLM on every `add`.
3. **A periodic consolidation pass** merges duplicates and rewrites summaries: Letta's sleep-time agents, Hindsight's observations, ICM's `consolidate`.

How changes are handled differs:
- mem0's original pipeline asks the LLM to ADD, UPDATE, DELETE or leave alone each fact.
- mem0's newest prompt only adds, and links each new memory to related old ones.
- Zep invalidates the old fact and keeps its history.

## 3. How often

| Trigger | Who uses it | Verdict |
|---|---|---|
| Session end / Stop | ICM, claude-mem | Yes: one extraction per session, cheap |
| Before compaction | ICM | Yes: this is the last chance before the transcript is cut |
| Every N user turns | (maintainer requirement) | Yes, as a safety net for long sessions: N around 15 to 20 user turns |
| After every tool call | ICM (opt-in), claude-mem | No: ICM turned it off for Codex after ~14,000 events a day flooded the store (ICM issue #288) |
| Explicit "remember this" | everyone | Yes, immediate |

## 4. Making sure the agent uses its memory

Nobody relies on the agent deciding to search. The products that score well **inject** memory:

- **Session start:** a small "wake-up pack":
  - ICM: about 500 tokens of critical and high-importance memories for the current project, plus preferences.
  - claude-mem: an index of the last 10 sessions, with details fetched on demand.
- **Each user prompt:** a hook runs a recall on the prompt and adds the relevant memories before the model sees it (ICM's `hook prompt`, claude-mem's prompt hook).
- **Tool descriptions and instruction files** remain as a fallback for agents without hooks.

The risk of injection is noise: every injected memory costs tokens and can mislead. **This is where Footnote's lead matters most.** Only a memory relevant enough should be injected, so retrieval quality decides whether auto-injection helps or hurts. Footnote already has a calibrated "return nothing below this score" threshold method (`docs/TECHNICAL-QUALITY.md`).

## 5. How they are measured

- **LongMemEval-S** (500 questions, ~48 past sessions each). Footnote already leads ICM on retrieval: all@5 0.908 against 0.820 locally.
- **LoCoMo** (1,540 questions): ICM 92.9 %, Hindsight 92.0 %, answer accuracy with an LLM judge.
- **PersonaMem** (589 questions about a user's changing preferences): Hindsight 86.6 %, ICM 81.7 %. **This is the benchmark for the "remembers how I like to work" promise, and ICM's weak spot.**

These measure retrieval over stored conversations. None of them measures whether the extraction kept the right things. For that we need our own check: replay real sessions, extract, then ask the questions the user would ask later.

## 6. Proposal for Footnote

**Storage: directly in the database, not in files.** A memory is a source row with no file behind it. It uses the same chunks, keyword index and two-model vectors as documents, so it inherits the quality lead. Each memory carries:
- its kind (preference, fact, fix, session);
- its scope (global, or a project);
- the date it was observed, and optional dates when it became true and when it stopped being true;
- the session it came from;
- optional `supersedes` links.

Add-only with supersede links, as in mem0's newest pipeline and Zep: nothing is silently deleted, history stays auditable, which fits Footnote's citation and audit-log identity.

**Writing:**
- `footnote_remember` and `footnote_forget` agent tools, for the explicit cases.
- A hook command, `footnote hook extract`, at session end, before compaction and every ~15 user turns. It sends the new turns to the user's own LLM command-line tool with a fixed extraction prompt, then writes the result. The Footnote binary itself still makes no network call.
- The prompt asks for the five kinds above, with absolute dates and self-contained sentences, and passes the existing related memories so it can skip duplicates and mark what an old memory supersedes.
- Fixes reuse what Footnote already has: the `records` table already stores symptom, root cause, remediation and verification for validated findings. An agent "fix" memory is a light version that can later be promoted into a validated record.

**Recall:**
- Session start: a wake-up pack injected by a hook. All preferences, the current project's decisions, and the last session summary, capped around 500 to 800 tokens. The existing `footnote_wake_up` tool already does a version of this for records.
- Each prompt: auto-recall by hook, injecting only the memories above a calibrated score threshold, with their date and kind.
- Hits are labelled "agent memory" with their date, so the model can tell a memory from a cited document and an old fact from a new one.

**Measurement before claiming anything:**
1. LongMemEval-S with memories written through the new path, compared with today's numbers.
2. PersonaMem, against ICM's and Hindsight's published scores.
3. LoCoMo retrieval-only, like ICM's retrieval table.
4. Our own extraction check on real sessions (does it keep the preferences, fixes and dates a human would?).
5. Time from write to searchable, and the cost of the extraction call per session.

## 7. Decisions (14 November 2026)

1. **Extraction model: the user's own LLM.** Built-in route: the user's existing coding agent CLI; no separate hosted provider is maintained by Footnote. Any other provider goes through one generic route configured with an endpoint, a model and an API-key variable (OpenAI-compatible API, which also covers local servers such as Ollama or LM Studio). ~~A local keyword extractor is the fallback when no LLM is configured.~~ Removed 14 November 2026: memory is for agents and by agents, so it is written by an LLM or not at all. Measured before removal, the keyword fallback found the answer's session in the top 5 for 0.50 of LongMemEval questions, against 0.98 when sessions are kept whole (`bench/experiments/MEMORY-BASELINE.md`). When the LLM is missing or fails, the conversation stays pending and is retried.
2. **Scope: preferences global, everything else per project.** What always applies is kept apart from what is project-specific. Exceptions stay explicit in the memory text ("usually prefers X; in Y cases prefers Z") instead of overwriting the general preference.
3. **Who may write: the extraction hook and explicit requests only.** Never automatic writes from tool output. A user command forces an extraction at the current point of the session.
4. **Sensitive projects.** A project can be marked sensitive. Its project memories are then never extracted through a non-local LLM. Global preferences may still be extracted. Project memories in a sensitive project come only from a local LLM route.

5. **Global preferences from agents wait for the user's review (19 November 2026, #207).** A global preference saved over MCP is stored as `pending`: no recall, pack or search returns it, and what it supersedes stays live, until the user approves it (`footnote memory-restore <id>`) or rejects it (`footnote forget <id>`). `footnote memory-review` lists the pending ones first, and from 5 pending the session start asks the agent to tell the user. Project memories, global facts and fixes, and anything written at the command line are live at once. There is no exemption for the hook's own save request, because an injected agent can answer that request too.

   *Why, for the product presentation:* a global preference is recalled into every later prompt, in every project, by every agent. Some grant permissions ("may run benchmarks without asking"). An agent that read a hostile page or README could otherwise turn one prompt injection into a standing rule that steers all future work (security audit of 18 November 2026, `docs/security/AUDIT-2026-11-18.md`). The cost is small: about 3 to 5 global preferences a day in the first week (64 global memories out of 297, 45 of them preferences), a minute of review, against the one memory kind whose forgery would do the most harm.

After agent memory: index build time (section "Where Footnote loses" in `bench/experiments/FINAL-RANKING.md`).

## 7b. One layer: the agent writes its own memories (14 November 2026)

Supersedes decisions 1 and 4 above. Footnote Memory is for agents and by agents: the agent people already use is the LLM that writes the memories, with `footnote_remember`, in its own context. No second model reads the conversation again.

- **When:** every 10 prompts, the Stop hook asks the agent once, at the end of a turn, to save what it learned (the hook's `decision: block` with the request as reason; it never asks twice in a row). `/remember` is a command whose text is the same request. Session start still injects the memory pack and each prompt still gets its relevant memories.
- **Why:** no second LLM to configure or pay, the best model available writes the memories with the full context instead of a weaker model reading an excerpt, the conversation is already in the agent's cached context so the cost is mostly the few hundred tokens of memories, and nothing is sent anywhere new. Agents that forget to save on their own are the known weakness of tool-based memory; here the hook asks at a fixed moment.
- **Removed:** the separate extractor (Claude CLI with Haiku or an OpenAI-compatible endpoint), transcript parsing, the prompt journal, catch-up, and the keyword fallback (measured at 0.50 any@5 against 0.98 for whole sessions, `bench/experiments/MEMORY-BASELINE.md`).
- **Decision 4 (sensitive projects)** existed so a project's text never reached a second, non-local LLM. With one layer, nothing new leaves the agent, so it is resolved by design and its setting is gone.
- **Vectors:** agents call the tool without model paths, so the hook computes vectors for new memories in the background after a save and at session start.
- **Not verified yet:** whether Codex honors a Stop hook's request to continue; Claude Code, Grok and DeepSeek's bridge do.

## 8. Built so far (14 November 2026)

- **Rust core** (`crates/memory`, schema 5 in `crates/search/src/store.rs`):
  - `remember` (JSON array on stdin), `forget`, `memory-pack` and `memory-recall` commands.
  - `footnote_remember` and `footnote_forget` agent tools.
  - Memory databases and corpus databases refuse each other's writes. Agent writes to a corpus are refused through a read-only connection, before any migration.
  - Tests in `tests/memory.rs`.
- **Hook** (`footnote hook`, [src/hook.rs](../src/hook.rs), Rust since 15 Nov 2026): session-start pack, per-prompt recall, and every `every_turns` prompts a Stop request that has the agent save its memories; vectors for new memories in the background. Tests in `tests/hook.rs`. Recall never repeats a memory already shown in the session, in lines cut at 200 characters (#249). Past `handoff_tokens` of context (200k by default, 0 off) Stop asks once for a handoff memory tagged `handoff`; the next session's pack, or the one after a compaction, starts with the project's newest handoff of the last 3 days (#250). To make this hands-free, set Claude Code's `autoCompactWindow` (in `~/.claude/settings.json`, at least 100k) a little above `handoff_tokens`, for example 200k with `handoff_tokens` at 150k: the handoff is saved first, then Claude Code compacts on its own.
- **Live check:** the Claude CLI route (Haiku) extracted a global preference and a project fix from a synthetic conversation in 5 seconds.

Not done yet:
- Per-prompt recall is lexical only, with no relevance threshold.
- The benchmarks in section 5 have not been run on the new path.
- The DeepSeek Harness bridge has not been seen running.

### Installing for every agent

`footnote install --memory` (add `--dry-run` to preview) sets up one shared memory for every detected agent:

| Agent | MCP server `footnote-memory` | Hook events | `/remember` |
|---|---|---|---|
| Claude Code | `~/.claude.json` | `~/.claude/settings.json`: SessionStart, UserPromptSubmit, Stop, PreCompact, SessionEnd | `~/.claude/commands/remember.md` |
| Grok | `~/.grok/config.toml` | runs the hooks in `~/.claude/settings.json` (asks once to trust a folder) | |
| Codex | `~/.codex/config.toml` | `~/.codex/hooks.json`: SessionStart, UserPromptSubmit, Stop (no compaction or end event: the rest is extracted at the next session start) | `~/.codex/prompts/remember.md` |
| DeepSeek Harness | `mcp-client` entry in `~/.dsh/profiles/desktop/cordis.patch.yml` | its Claude Code hooks bridge, pointed at `~/.dsh/footnote-memory-hooks.json` | |

Per tool, the hook reads the conversation from:
- Claude Code and Codex: their transcript files.
- Grok, and DeepSeek if its transcript is not one of these formats: the hook's own journal of prompts and final answers.
- Grok also discards context added on each prompt, so it gets memories only at session start.

Settings are in `%LOCALAPPDATA%/footnote/memory.json`.

**Installed local binary (14 November 2026).** Binary `%LOCALAPPDATA%/Programs/footnote/footnote.exe`, a release build kept apart from `target/release`, which the benchmark receipts use. Checked:
- the hook command runs under bash and PowerShell;
- Codex and Grok list the server;
- the server exposes `footnote_remember` and `footnote_forget`.

Not yet checked: the DeepSeek Harness bridge, until the app is restarted.
