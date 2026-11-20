# PRODUCT
Last reviewed: 2026-11-12 · Owner: Footnote maintainers

## 1. Users and context
- Primary user: a developer or technical lead who works daily with AI coding agents (Claude Code, Codex, Hermes, Cursor, Gemini CLI, any other MCP host), experienced, careful about what leaves their machine.
- Context of use: web (landing page, README on GitHub) read on a laptop or a phone; minimum supported viewport 390 px wide, design reviewed at 1440 x 900. The product itself is a CLI plus an MCP server on stdio. English first; French documents are a first-class case, not an afterthought.

## 2. Top three tasks
1. Understand in one screen what Footnote is and whether it beats the memory they have now — success looks like a visitor who can say "local memory with cited, checkable answers, measured on public benchmarks" after 20 seconds.
2. Install it and connect it to their agent — success looks like one command, a working recall in the agent within two minutes, and the exact line for their harness on screen.
3. Trust it — success looks like finding, without asking anyone, where the data lives, what the agent may and may not do (read-only), and the benchmark receipts behind every number.

## 3. Tone
- Is: plain, exact, quietly confident, documentary.
- Is not: hyped, cute, "magical", vague about numbers.

## 4. Success metric
A visitor reaches the GitHub repository from the landing page (click on "View on GitHub"), and a first-time user connects one harness in under two minutes.

## 5. Non-goals
- Not a cloud service, not a vector database product, not a chat UI.
- No telemetry, no accounts, no network calls from the page or the tool.
- No claim without a receipt: a number appears only with its dataset, split and source.

## 6. Accessibility floor
WCAG AA. Keyboard-complete. Reduced motion honoured (everything rests, nothing stays hidden). Text contrast measured in DESIGN.md. Exceptions: none.
