# Skills

Agent skills in the [Agent Skills](https://agentskills.io/specification) format: one folder per job, a short `SKILL.md`, and scripts the agent runs instead of reasoning through the same checks every time. Scripts need Node 20+ or Python 3.10+ and nothing else.

## Install
Copy a skill folder into your agent's skills directory (for Claude Code, `~/.claude/skills/<name>/`). Leave out files and folders whose names start with `_`: `_sources.md` holds the citations behind the rules, for maintainers only.

## Skills
| Skill | Use it for | Scripts |
|---|---|---|
| `analyze-data` | answering a question from data, reviewing an analysis or backtest | |
| `apple-style-ui` | Apple-inspired interface design or review, from the public HIG | |
| `automate-and-scrape` | scraping, crawling, browser automation, scheduled sync jobs | robots-check |
| `build-ai-features` | RAG, semantic search, LLM extraction or agents; choosing a model | ir-score |
| `choose-visual-direction` | a new screen or site with no design yet | studio |
| `configure-nixos-hyprland` | NixOS, Home Manager, flakes, Hyprland desktops | |
| `debug-and-test` | bugs, failing or flaky tests, proving a fix | red-green, leftover-check |
| `draw-architecture-diagrams` | architecture and workflow diagrams as code (LikeC4) | |
| `fix-ci-and-releases` | failing or slow CI, GitHub Actions security, releases | workflow-check |
| `launch-website` | pre-launch and live-site audits | launch-check |
| `make-code-faster` | profiling and proving a speed-up | bench-gate |
| `make-slides-and-reports` | decks, reports and spreadsheets for other people | office_check |
| `manage-databases` | schemas, queries, indexes, migrations, backups | sqlite_health |
| `review-code` | reviewing a diff or PR, acting on review feedback | sarif-diff |
| `review-ui-quality` | UI QA with hard gates, screenshots and an independent critic | slop-scan, design-check, ref-capture |
| `run-benchmarks` | deciding whether B beats A on quality | paired-bootstrap |
| `run-project-governance` | specs, tickets, triage, priorities, RAID, status | |
| `script-windows` | PowerShell and Windows automation | |
| `ship-desktop-app` | Tauri desktop apps: testing, installers, signing, updates | |
| `ship-rust-cli` | Rust CLIs, daemons and libraries, through release | |
| `write-a-skill` | writing or reviewing a skill | skill-check |
| `write-modern-css` | CSS for any UI: controls, states, motion, tokens | |
| `write-plainly` | prose for people and for agents | prose-lint |

## Maintaining
- `node write-a-skill/scripts/skill-check/skill-check.mjs .` checks names, sizes, links and source freshness; `--stamp` records the skill hash after a source review.
- Tests: `node --test $(find . -name '*.test.mjs')`, `python -m unittest` in each folder that holds a `test_*.py`.
