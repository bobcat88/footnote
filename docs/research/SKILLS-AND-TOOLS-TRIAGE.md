# Skills and tools triage, 19 November 2026

Johan sent 20 screenshots of tools and skills that are trending. This note checks each one against what Footnote already has and against the work planned before the v0.9 alpha. It also lists which skills each planned step needs. It feeds the curated-skills epic (#229) and does not change it: nothing on this list is installed or listed until it passes the source review (#229, security paragraph) and the with-versus-without test (#232).

Verdicts: **adopt** (review it, then add it to the catalogue), **cherry-pick** (take a few parts, not the whole pack), **compete** (a rival to benchmark against), **study** (borrow ideas only) and **skip**.

## The finds that change plans

1. **TokenEconomizer (Microsoft Research, UC Berkeley; MIT) competes directly with Lean.** It is a hook plugin for Claude Code, Codex, Copilot CLI and others, built on three mechanisms: tool pruning (removes tools unused after 10 prompts across 2 sessions; on by default), a loop guard (a judge model sends one redirect note when a turn repeats the same failures; on by default) and micro-skills (repository facts confirmed by 3 sessions and injected at start; off by default because it adds model calls). On 277 SWE-Bench tasks it reports 18.4–30.9% lower mean cost, lower in all 30 cells, and Pass@1 up in 4 of 5 agent and model pairs (down 1.0 with Opus 4.8). These are vendor numbers on older models.
   - Micro-skills overlap Footnote Memory, tool pruning automates what we did by hand when we removed codebase-memory-mcp from normal runs, and the loop guard is something Lean does not have.
   - Action: add it as an arm in Lean E (#244). Lean must beat it on cost per accepted delivery, or we cannot claim the lead.
2. **Indra OCR (Datalab) is a strong new candidate for the OCR benchmark (#136).** It runs on CPU at about 210 ms a page on 4 cores and outputs Markdown, text or JSON with boxes and reading order. It reports 66.7 on OmniParseBench against 58.4 for PaddleOCR PP-StructureV3 (vendor numbers; French is not benchmarked separately). The models are ONNX files with no PyTorch, so Footnote could run them from Rust through ONNX Runtime and stay within the all-Rust rule. Licences: the code is Apache-2.0, but the weights are under a modified OpenRAIL-M licence, free for personal use, research and startups under $20M, and not for use that competes with Datalab's API. That fits Johan's personal corpus. Shipping the weights inside Footnote for companies would need a licence check first.
3. **SkillEvaluator (NVIDIA; Apache-2.0)** does two things #229 needs. Its static tier checks a skill's schema, PII, licence, hidden Unicode and scripts, runs Gitleaks, Bandit and SkillSpector, and tests whether each agent actually loads the skill. Its live tier compares runs with and without the skill in a sandbox. Action: use its static tier as the automated first pass of the source review (lab tooling, so Python is fine). Keep our Flow matrix for the live comparison, because ours measures cost per accepted delivery; theirs records tokens but scores quality.

## Every screenshot

| Find | What it is | Verdict | Why |
|---|---|---|---|
| TokenEconomizer | Cost-control hooks for coding agents | **compete** | See above; arm in #244 |
| Indra OCR | CPU OCR to Markdown, ONNX | **adopt for the #136 test** | See above; personal-use licence fits E7 |
| SkillEvaluator | Skill linting and with/without runs | **adopt (static tier)** | First pass of the #229 review |
| marketingskills (coreyhaines31, MIT, 52 skills) | Marketing, SEO and growth skills that share one product-context file | **cherry-pick** | Fills the SEO and launch gap (below). Its shared context file is the same idea as our PRODUCT.md |
| Modern Web Guidance (Chrome team, with Edge and Firefox) | Skill and CLI that fetch current web-platform guidance and Baseline compatibility data | **adopt** | Official source, small, fills a real gap for the website: agents write dated front-end code |
| OpenSEO (MIT) | Self-hosted Semrush or Ahrefs alternative with an MCP server | **skip for now** | Needs paid DataForSEO credits. Google Search Console and Bing Webmaster Tools are free and enough for an alpha site |
| Graphify | Code and docs knowledge graph | **done** | Already benchmarked against Footnote Code (#246, docs/CODE-GRAPH-DESIGN.md) |
| Dashboard Design skill (abdiraimov-otabek, ~64 stars) | Dashboard UI skill with screenshot checks | **adopt** (after due diligence) | Its "done when" steps, known-total fixtures and honesty-first critique are worth taking whole; remove its promo sign-off |
| ppt-master (MIT, ~75k stars) | Agent-built native PowerPoint (SVG to DrawingML) | **study** | We have `premium-editable-presentations` and `pptx`. Its native-chart path is the part to compare. It is token-heavy, and its PDF step pulls in PyMuPDF (AGPL) |
| Papermorph | PDF to animated web book skill | **skip** | No planned use |
| MotionSites skill | Cinematic landing pages | **have it** | Installed as `motionsites-web-builder` |
| Humanizer, ADHD, Taste, UI UX Pro Max, Archify | Writing and design skills from the same video lists | **skip** | `unslop`, `impeccable`, `design-taste-frontend` and `design-studio` cover them; more overlapping design skills make output less consistent |
| Cap | Open-source screen recorder | **adopt as a tool** | Demo videos for the alpha launch, free |
| OpenWhispr, VoiceStudio, Open Higgsfield, Chatwoot, AppFlowy | Dictation, voice, AI video, support desk, notes | **skip** | Off the plan. Chatwoot only if alpha support outgrows GitHub issues |
| AG-UI (MIT) | Event protocol between agents and front ends | **catalogue candidate** | Not needed for Footnote today; for any project that streams agent runs into a UI, use its event model and review its `ag-ui-a2ui-integration` skill |
| gitcraft | Rust + egui git client | **reference** | The Launcher stays Tauri; as a cross-project reference for an egui desktop skill |
| Sniffnet, Pulse | Network monitor, Windows event viewer | **skip** | Off-topic. Pulse's timeline is a decent reference for a health view |
| Skiper UI, Vengeance UI, Animaster | React animation libraries | **skip (no source)** | Paid packs with no public source to review; if a project needs one, a free library (Magic UI) is the candidate |
| "20 ways to unslop AI designs" | Checklist | **study** | Mostly in `impeccable` and `unslop` already. Check DESIGN.md for the three often missed: tabular figures, inner radius = outer radius − padding, loading states |
| "22 backgrounds" | Texture list | **study** | The blueprint grid is already in the Code map direction. Use one texture at most, on marketing pages only |
| Comment on plugin safety ("what can it read, run, reach; is it pinned") | Review questions | **have it** | Same rule as the #229 review: pinned, hashed, read before listing |

## Skills each planned step needs

| Planned step | Have | Gap, and the proposed fill |
|---|---|---|
| Website design and build | `design-studio`, `impeccable`, `design-taste-frontend`, `motionsites-web-builder`, `ui-ux-qloop`, `playwright-cli`, `website-launch-checklist` | Modern Web Guidance |
| SEO | `website-launch-checklist` (sitemaps, structured-data policy) | marketingskills: `seo-audit`, `ai-seo` (being cited by AI answers, which matters most for a developer tool), `schema`, `site-architecture` |
| SEA (paid ads) | — | Not needed for the alpha. A minimal-spend plan relies on organic reach. Take `ads` only if a budget appears |
| Marketing and launch plan | — | marketingskills: `launch`, `marketing-plan`, `directory-submissions`, `community-marketing`, `competitors` (comparison pages, backed by our benchmarks), `social` |
| Product descriptions | `unslop`, PRODUCT.md | marketingskills `copywriting` and `product-marketing`, adapted to read our PRODUCT.md instead of their own context file |
| Per-product presentations | `premium-editable-presentations`, `pptx` | None; compare against ppt-master's native charts if decks become a channel |
| Benchmark charts | `dataviz`, Johan's chart format | None |
| Demo videos | — | Cap (tool) |
| OCR personal corpus (E7, #136) | DOCUMENT-INTELLIGENCE-RESEARCH.md (Docling, PaddleOCR, Tesseract) | Add Indra to the #136 test set |
| Skill curation itself (#229, #232) | Source review, Flow matrix | SkillEvaluator's static tier |
| Lean (#238–#244) | Flow matrix | TokenEconomizer as the rival arm |

## How to adapt a pack to Footnote's design

Marketing packs are written for SaaS growth teams and push volume: popups, urgency, programmatic pages. Footnote's line is data first and plain words. When we cherry-pick:

- **One context source.** Point each skill at PRODUCT.md, DESIGN.md and the benchmark reports instead of a second marketing context file that drifts.
- **Claims come from our benchmarks.** Every comparative claim links to a dated benchmark report. The `competitors` skill becomes "comparison pages built from our measured results", not adjectives.
- **Run `unslop` on every output.** No urgency tricks, popups or invented social proof (drop `popups`, `paywalls`, `offers`, `cold-email`, `sms`).
- **Vendor and pin.** Copy the chosen `SKILL.md` files into `skills/skills/` with their source commit and hash in the manifest, as the existing pack does, and record the review in the PR.
- **Then measure.** Each kept skill runs with and without on 3 real tasks (#232). Marketing tasks need an acceptance rubric (for example: every claim sourced, no banned patterns, structured data valid), since there is no test suite to pass.

## Next steps

1. Lean E (#244): add TokenEconomizer as an arm in the Flow matrix once the current ladder run finishes.
2. #136: add Indra to the OCR test with French accented scans; record the weights licence in the result.
3. #229: review and vendor Modern Web Guidance and the 12 marketingskills listed above. Run SkillEvaluator's static tier on all of them and on our existing 18 skills.

## Sources

- TokenEconomizer: https://github.com/microsoft/TokenEconomizer (arXiv 2610.22732)
- Indra OCR: https://github.com/datalab-to/indra
- SkillEvaluator: https://github.com/NVIDIA/SkillEvaluator (arXiv 2608.20614)
- marketingskills: https://github.com/coreyhaines31/marketingskills
- OpenSEO: https://github.com/every-app/open-seo
- Modern Web Guidance: https://developer.chrome.com/docs/modern-web-guidance
- ppt-master: https://github.com/hugohe3/ppt-master
- Dashboard Design skill: https://skillsllm.com/skill/dashboard-design-skill
- AG-UI: https://github.com/ag-ui-protocol/ag-ui

Due diligence on the cloned repos: [THIRD-PARTY-DUE-DILIGENCE.md](THIRD-PARTY-DUE-DILIGENCE.md). Verdicts above are for the cross-project catalogue, not only the Footnote plan.

All retrieved 19 November 2026. Figures are the projects' own and have not been reproduced.
