# Third-party due diligence, 19 November 2026

Follow-up skills & tools triage (19 Nov). Sourced upstream repositories were downloaded into a separate temporary review folder outside this repository and checked for structure, execution behavior, network calls, and licences. Candidates were evaluated for Footnote's agent workflow needs and ecosystem alignment.

Method: shallow clone of each real upstream repository into an external temporary folder with git hooks disabled, no submodules, and no LFS. Nothing was installed or run. Manifests, build scripts, install hooks, network calls, and process launches were audited directly.

| Repo | Commit | Licence | Size |
|---|---|---|---|
| microsoft/TokenEconomizer | `4a04fe4` | MIT | 7 MB |
| datalab-to/indra | `ca81e18` | Apache-2.0 code; weights modified OpenRAIL-M | 4 MB |
| NVIDIA/SkillEvaluator | `ac9fc0e` | Apache-2.0 | 31 MB |
| coreyhaines31/marketingskills | `73573e9` | MIT | 9 MB |
| GoogleChrome/modern-web-guidance | `64a6d70` | Apache-2.0 | 3 MB |
| abdiraimov-otabek/dashboard-design-skill | `463b8fd` | MIT | 2 MB |
| hugohe3/ppt-master | `d8f704b` | MIT (PyMuPDF dependency is AGPL) | 94 MB |
| every-app/open-seo | `672a66c` | MIT | 44 MB |
| ag-ui-protocol/ag-ui | `3416b86` | MIT | 84 MB |
| Vancete/gitcraft | `a067383` | MIT or Apache-2.0 | 3 MB |

Not downloaded: Animaster and Vengeance UI are paid packs with no public source (Vengeance's npm package was unpublished). Skiper UI is shadcn-based with free and $129 premium parts. The GitHub repo that appears in searches (Sandeep-Hipparagi/Skipper-UI) calls itself a reverse-engineered copy of the paid set, so we leave it alone.

## Verdicts for the catalogue

| Repo | Verdict | One line |
|---|---|---|
| TokenEconomizer | **compete** (Lean arm in #244) | Strong ideas, slow mechanics: one Python process per hook event |
| Indra | **adopt for the #136 test** | About 2,400 lines of post-processing around ONNX models; portable to Rust |
| SkillEvaluator | **adopt `quality-check` and Tier 2** | Keyless quality score; Tier 2 finds overlap between skills in a catalogue |
| Dashboard Design skill | **adopt** (was "study") | Best-written skill of the set; remove its promo sign-off |
| Modern Web Guidance | **adopt the guides, not the CLI** | 237 good local guides, but the skill runs `npx …@latest` on every query |
| marketingskills | **cherry-pick 12** | Useful content; heavy, and 24 skills point at a partner tool registry |
| ag-ui | **catalogue candidate (agent-UI projects)** | Frozen 1.0 protocol with a Claude Agent SDK integration and its own skill |
| gitcraft | **reference, not a skill** | Clean egui code; sends commit-author email hashes to Gravatar |
| ppt-master | **study `svg_to_pptx` only** | 19 dependencies, a self-updater and a Gemini watermark remover |
| OpenSEO | **skip** | Needs paid DataForSEO; the hosted version adds 28% to every request |

## Per repo

### TokenEconomizer

What: a hook plugin for Claude Code, Codex, Copilot CLI, Cursor, Pi, opencode, Grok, Hermes, Droid and Antigravity. Three mechanisms: micro-skills (repo facts learned over sessions), tool pruning, and a loop guard.

How:
- Python standard library only. On Claude Code it registers one launcher (`plugin/scripts/te-hook`) on six events: SessionStart, UserPromptSubmit, PostToolUse, PostToolUseFailure, Stop and SessionEnd. Each event starts a Python interpreter, and on Windows it first searches PATH for one.
- Tool pruning writes bare-name `permissions.deny` rules into the repo's `.claude/settings.local.json`. That removes tools from the session's list, not just their calls. On Codex it writes `.codex/config.toml`, on Copilot `disabledMcpServers`. In `auto` mode (the default) it writes these files by itself once sessions have been idle for a while. It adds the file to `.git/info/exclude` and only removes entries it added.
- Changes apply from the next session, because swapping tools mid-session would rewrite the prompt cache. That is a sound finding worth reusing.
- Model "side calls" (proposer, stuck judge, nudge writer) run through the CLI the user is already signed in to: `claude -p --tools "" --max-turns 1 --strict-mcp-config` on Haiku by default. There is no direct network code and no API key. It sets `TE_SIDECALL=1` so its own hooks don't recurse.
- Data stays in `~/.tokeneconomizer`. Folding a session runs in a detached background process.

Risks: by default it changes agent settings in the user's repos on its own, and side-call prompts contain repo-controlled text (it does deny tools to those calls). The SWE-Bench numbers are the vendor's own, on older models.

What Footnote can do better:
1. **Speed.** A Rust hook binary instead of an interpreter on every tool call. On Windows, starting Python costs tens of milliseconds per event, and PostToolUse fires on every tool call. Measure it in the #244 arm.
2. **Consent.** Propose pruning and show the diff; don't write settings silently. TokenEconomizer's own `propose` mode shows this is cheap to do.
3. **What Lean is missing:** a loop guard. Copy the design (rules first, a judge only when the rules see trouble, one redirect note) and measure it.
4. **Memory.** Micro-skills is a weaker Footnote Memory (repo facts confirmed by 3 sessions). Footnote Memory already has review and scope; show the difference with numbers, not adjectives.

### Indra OCR

What: CPU OCR, PDF or image to Markdown, text or JSON with boxes.

How: `indra_ocr/` is about 5,100 lines of Python: detection, line masks, CTC recognition, layout, reading order, right-to-left text, math, and a back-off language model stored as `lm.npz` (no pickle). Models are ONNX files downloaded from Hugging Face at first run through `huggingface_hub`, pinned to a revision tag and cached, with no network after that. It runs no other processes. The optional viewer is a local HTTP server, and its page loads Google Fonts (a network call from a page that says your files stay local) and an upsell to Datalab's paid API.

What Footnote can do better: load the ONNX files from Rust with ONNX Runtime and pin them by SHA-256 instead of using a Hugging Face client. The post-processing (about 2,400 lines across `det_post`, `order_post`, `layout_post`, `pipeline` and `document`) is the part to port. Run the #136 test first to see whether it is worth porting. The weights licence allows personal use, which fits E7. Shipping the weights to companies needs a licence check.

### SkillEvaluator

What: three tiers. Tier 1 validates (schema, PII, licence, hidden Unicode, Bandit, Gitleaks, SkillSpector, and a harness check of whether each agent loads the skill). Tier 2 deduplicates. Tier 3 runs agents with and without the skill in Docker through Harbor.

How: a large Python project (877 .py files, a 3,287-line `uv.lock`). It runs SkillSpector as a pinned wheel through `uv tool run`. Tier 2 and Tier 3 need a model provider (a free NVIDIA Build key works). Its "telemetry" code reads token counters from run traces; I found no phone-home code.

Correction to the triage note: Tier 3 does record tokens per arm (prompt, cached and completion). It is a measurement only and does not change the verdict.

What we take:
- `skillevaluator quality-check ./skill` needs no key, scanner, GPU or Docker. Make it the automated first step of the #229 review.
- Tier 2 with `--catalog` reports overlapping guidance between skills. That is the overlap problem we already have with our design skills; run it on the whole catalogue.
- Keep our Flow matrix for live runs: it measures time and cost per accepted delivery, and theirs scores quality.

### Dashboard Design skill

What: one 8.8 KB skill and 10 reference files (layout, tokens, perception, components, snippets in React and Tailwind, SwiftUI, QA, and two case studies).

Why adopt: every build step ends with a "done when" test. It asks for a fixture with known totals before any UI, makes honesty checks come first in a critique, and requires a "compared to what?" baseline for every number. Those are the habits our benchmark charts need. It also covers SwiftUI, which helps Apple-platform projects.

Before vendoring: delete the "Sign-off" section, which tells the agent to end finished work with an advert for the author's Mac apps. Then check it against `dataviz` for overlap (SkillEvaluator Tier 2).

### Modern Web Guidance

What: 237 Markdown guides from the Chrome team (UI behaviours, performance, forms, CSS, security, accessibility…) and a Chrome extensions skill.

How: the SKILL.md tells the agent to run `npx -y modern-web-guidance@latest search …`, then `retrieve`, on every web task. That fetches and runs the newest npm release each time. The CLI's source is in a separate repo (`modern-web-guidance-src`) that I have not reviewed.

What we do better: vendor the guides at a pinned commit, search them locally with grep, and drop the `npx …@latest` step. That avoids network calls and running unpinned remote code, which is our pinning rule. Re-sync the guides on purpose, as a reviewed update.

### marketingskills

What: 52 skills, plus 142 Node CLI wrappers for paid marketing APIs (Ahrefs, Amplitude, Apollo, …) that read keys from environment variables, plus a partner registry (`partners.json`) in which paid partners are marked ◆.

Findings: the skills are large (`ai-seo` alone is 36 KB, about 9,000 tokens). 24 SKILL.md files point at the tool registry or partner integrations, so vendoring them as-is would put sponsored tools into our agents' advice. Every skill reads `.agents/product-marketing.md`.

What we do better: take the 12 listed in the triage note, cut each to what we use, point them at PRODUCT.md, remove the partner and tool-registry references, and don't vendor the CLIs.

### ag-ui (cross-project)

What: an MIT event protocol between an agent backend and a UI. Version 1.0 is frozen and backed by a JSON Schema, with 31 events in eight families (runs, text, tool calls, state snapshots and JSON Patch deltas, reasoning, activity, subagents, passthrough). There are SDKs in TypeScript, Python and .NET, and integrations for 23 frameworks, including the Claude Agent SDK and Claude Managed Agents.

Risks: a large monorepo (6,000 files). The only install hook is `postinstall: playwright install chromium` in its e2e test app, not in the SDKs.

For the catalogue: any project that streams an agent run into a UI (a run viewer, `footnote run` progress for #243, a chat front end) should consider this event model instead of inventing a custom schema. Its own skill, `ag-ui-a2ui-integration`, is a candidate for an "agent UI" category.

### gitcraft (cross-project)

What: a Rust and egui 0.36 git client, about 6,600 lines. It runs the installed `git`, `gh` and `glab` rather than libgit2, so the user's config, credentials and hooks apply.

Good craft: `unsafe_code = "forbid"`, clippy denies `unwrap`, `expect` and `panic`, a PTY terminal through `portable-pty`, and file watching through `notify`. `build.rs` only embeds the Windows icon.

Privacy issue: `avatars.rs` sends the MD5 hash of every commit author's email to Gravatar (and asks GitHub or GitLab for avatars), with no setting to turn it off. We would make that opt-in.

For the catalogue: a reference repo for a future "egui desktop app" skill next to `ship-rust-tool`, not a skill itself.

### ppt-master

What: an agent pipeline that builds SVG slides and converts them to native PowerPoint shapes (`svg_to_pptx`, `pptx_shapes`, `pptx_ooxml`).

Red flags: 19 Python dependencies, including PyMuPDF (AGPL), `curl_cffi` (fakes browser TLS fingerprints to get past bot checks), `google-genai`, and image backends for eight paid APIs. `update_repo.py` runs git pull and pip install by itself. `gemini_watermark_remover.py` removes Gemini's visible watermark from generated images; we won't vendor anything that removes AI-provenance marks. 12,196 of its 13,182 files are icons.

What we take: only the SVG to DrawingML converter, studied for `premium-editable-presentations` if native charts in decks become a need.

### OpenSEO

A Cloudflare Workers app (TanStack, Drizzle) with an MCP server and agent skills. All data comes from DataForSEO, which is paid, and the README's DataForSEO link is an affiliate link. The hosted version adds 28% to every request. Skip. Google Search Console and Bing Webmaster Tools cover the alpha site for free.

## Next steps

1. #244: add TokenEconomizer as an arm and record its hook latency next to Lean's.
2. #136: run Indra on the French scans; if it wins, open a story to port the post-processing to Rust with ONNX Runtime.
3. #229: run `skillevaluator quality-check` (keyless) on the existing skills and candidates. Review any candidates before adoption.
4. Catalogue backlog: the ag-ui skill, an egui desktop skill using gitcraft as its reference, and a reviewed animation library if needed.

Keep external clones outside the repository and clean up temporary review clones once done.
