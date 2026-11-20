# Agent signatures on GitHub

Every change in this repository is tracked by a GitHub issue, and every issue, comment, or pull request filed by an AI agent ends with a signature naming the **agent + model** and its logo.

## Logos

| Family | File | Preview |
|---|---|---|
| Antigravity / Gemini | `.github/assets/agents/gemini.svg` | <img src="assets/agents/gemini.svg" width="20" height="20" alt="Gemini" /> |
| Claude Code / Claude | `.github/assets/agents/claude.svg` | <img src="assets/agents/claude.svg" width="20" height="20" alt="Claude" /> |
| Codex / OpenAI GPT | `.github/assets/agents/gpt.svg` | <img src="assets/agents/gpt.svg" width="20" height="20" alt="GPT" /> |

## Signature snippets

Append one of these blocks at the bottom of every GitHub issue, PR description, or issue/PR comment (replace `<Model>` with the exact human-readable model name used for that run):

### Antigravity · Gemini

```markdown
---
<img src="https://github.com/bobcat88/footnote/blob/main/.github/assets/agents/gemini.svg?raw=true" width="16" height="16" align="absmiddle" alt="Gemini" /> **Antigravity · Gemini 4 Neon**
```

### Claude Code · Claude

```markdown
---
<img src="https://github.com/bobcat88/footnote/blob/main/.github/assets/agents/claude.svg?raw=true" width="16" height="16" align="absmiddle" alt="Claude" /> **Claude Code · Claude Sonnet 5.5**
```

### Codex · GPT

```markdown
---
<img src="https://github.com/bobcat88/footnote/blob/main/.github/assets/agents/gpt.svg?raw=true" width="16" height="16" align="absmiddle" alt="GPT" /> **Codex · GPT-6.2 Sol**
```

## Ownership & metrics

Combine the footer above with an `agent:gemini`, `agent:claude`, or `agent:gpt` label and follow the checklist in [docs/ISSUE-RULES.md](../docs/ISSUE-RULES.md). Maintainers audit issue hygiene and model-level throughput with:

```bash
python scripts/agent_metrics.py --limit 100
```
