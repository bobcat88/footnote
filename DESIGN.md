# DESIGN
Last reviewed: 2026-11-14 · Direction locked by Johan in the Design Studio on 2026-11-14 · **Status: WIP**

> **Work in progress.** The direction, palette, type and shape below are locked choices. The logo and product marks, the glass treatment and the motion are a decent first pass that will need deeper work: treat them as provisional, not final artwork.

Footnote is transparent for your team: set it up, and what comes into focus is your company's data and knowledge. The page shows that literally: data flowing in the background, liquid glass that brings part of it into focus, and the product answering with its sources.

## 1. Tokens source
Hand-written, then tuned and locked in the Design Studio on 2026-11-14 (base direction D2 "Afterglow"). Contrast values measured by the studio at the lock. Studio page: https://claude.ai/artifact/9nnqR2T6Z78NUwADRSXWxB; work files in `.design-studio/` (local only, not in git).

## 2. References
| # | Product or source | URL | Date viewed | Evidence type | Borrowed | Rejected and why |
|---|---|---|---|---|---|---|
| 1 | Raycast | https://www.raycast.com | 2026-11-14 | live page | dark ground, short centred headline, the product shown as a command palette | the shader sculpture standing in for the product |
| 2 | LangChain | https://www.langchain.com | 2026-11-14 | live page | a field of flowing dots as the background (data in motion) | blue accent, logo wall |
| 3 | Mode | https://mode.com | 2026-11-14 | live page | large rounded blocks that interlock around content | stock photography, lime and forest palette |
| 4 | Fabric | https://www.fabric.vc/portfolio | 2026-11-14 | live page | card grid with category tags and card motion | orange line-field header |
| 5 | Zed | https://zed.dev | 2026-11-14 | live page | the real product filling the hero | (direction D1, not chosen) |
| 6 | Warp | https://www.warp.dev | 2026-11-14 | live page | captioned live figure with a legend | (not used) |
| 7 | mem0 | https://mem0.ai | 2026-11-14 | live page | category baseline to differ from | centred SaaS hero, purple, code block as the product |
Structure, layout and motion patterns only: no brand names, logos, copy, imagery or proprietary fonts are copied. Blocked access: none.

## 3. Color roles
Dark-first. A light theme is not designed yet (WIP); until it is, the page commits to dark and sets every color explicitly.

| Role | Dark | Contrast |
|---|---|---|
| background | #09120B | - |
| surface (blocks, cards) | #121C15 | - |
| line (hairlines, borders) | #29332C | - |
| text | #EAF0EB | 16.47:1 on background, 15.1:1 on surface |
| muted text | #A2AFA5 | 8.35:1 on background |
| accent (teal) | #2B9185 | 4.98:1 on background (UI) |
| button text on accent | #101114 | 4.94:1 |
| glow (data dots, marks in lenses) | #5FD3C3 | decorative and marks only |
| mint block (statements, results) | #D7EFE8 with ink #0B1A12, muted #3D5A4D | pale contrast blocks, added 2026-11-14 |
| success / warning (freshness: current, stale) | #7FD39A / #F2B84B | from D2, to re-measure |

Rule: the accent marks actions, Footnote's own results in charts, and sources. No violet, no blue-to-purple gradient.

## 4. Typography
- Families: **Plus Jakarta Sans** 600 (display, chosen by Johan) and **Hanken Grotesk** (body); JetBrains Mono for paths, hashes, commands and numbers in tables. Self-hosted in the final build (local-first: no font request leaves the page).
- Scale (ratio 1.335, density 1.05):

| Name | Size | Use |
|---|---|---|
| small | 12 px | captions, meta rows (11 px floor for functional text) |
| body | 16 to 18 px | prose, 60 to 70 characters per line |
| h3 | 29 px | block titles |
| h2 | 38 px | section titles |
| h1 | 68 px | hero statement |

- Tabular numerals wherever numbers are compared.

## 5. Space and shape
- Spacing steps (from the lock): 4, 8, 13, 17, 25, 34, 50, 67 px.
- Radius: 21 px for blocks and cards; full round for pills and lenses.
- Elevation: **slight liquid glass** on chosen elements only (the logo lens, the product palette, one or two hero panels): translucent fill, specular rim, refraction of the moving data behind. Everything else is flat surface + hairline. Never glass on every card.
- Icons: thin outline, 1.6 stroke on a 24-unit grid, round joins.

## 6. Layout and density
- Max content width 1120 to 1200 px; 16 px minimum side gutter; works at 390 px.
- Rounded blocks that interlock (reference 3) for the sections after the hero.
- Marketing density: one idea per screen.

## 7. Logo and product marks (WIP)
Concept A, "the lens numeral": the word **Footnote** is the family; a small glass lens takes the place of the footnote number and carries the product's mark. Everyone reads Footnote as the family and the mark as the product.

| Product | Mark in the lens | Status |
|---|---|---|
| Footnote (umbrella) | the numeral 1 | provisional |
| Footnote Library | database cylinder: L1 plain, or L2 with lines of text | open, not chosen |
| Footnote Memory | wireframe brain: M1 plain, or M2 with connected nodes | open, not chosen |
| Footnote Code | `{x}` (Johan) | chosen, drawing provisional |

`.design-studio/` is a local working folder, ignored by git: its files are on the designer's machine, not in the repository. Sketches: `.design-studio/logo/index.html` (concepts A, B, C), `.design-studio/logo/v2.html` (marks). Still to do: final SVG geometry, a still version for places without motion, an app icon (concept B, the focus tile, is the candidate), and a pass by someone with real logo skills.

## 8. Motion (WIP)
- Level: lively, with meaning. Durations 160 to 320 ms, ease-out, transform and opacity only.
- Background: a slow field of dots flowing in bands (the company's data), paused under reduced motion.
- The lens drifts slightly over the data, as if reading it.
- Cards rise and reveal on entry and hover (reference 4); content is visible at rest and never waits at opacity 0.
- Never: numbers counting up, parallax on text, motion that implies a result the tool did not measure.

## 9. Copy rules
- Every number carries its dataset and split; every claim links to a receipt.
- Footnote Code is shown as "soon" until it ships.
- Banned words: "supercharge", "unlock", "seamless", "next-generation", "revolutionize"; no fake testimonials, logo walls or invented customer counts.

## 11. Launcher (app), locked 16 November 2026
Studio run with four divergent directions (`.design-studio/launcher/`, studio page https://claude.ai/artifact/GQrHYr68hMgGzo3GXGe4rx).
Johan chose D1's structure for the full window and D2's lenses for the mini mode, with the family's colours and fonts on both,
and asked for these settings to be applied for him. Composition reference: `.design-studio/launcher/chosen/index.html`.

- **Full window (D1 "Console" structure):** sidebar 232 px (products, workspace, not installed, Footnote Enterprise group, quiet
  support link at the foot), top bar 44 px, content, activity column 320 px. Dense like T3 Code: tables for projects and agents,
  three product cards with one big number each.
- **Mini mode (D2 "Lens"):** the tray panel and the floating bubble: one glass lens per product, live data moving between them, a
  few numbers. A switch moves between mini mode and the full window.
- **Tokens:** the site's palette (section 3), Plus Jakarta Sans for titles and big numbers, Hanken Grotesk 14 px for UI text,
  JetBrains Mono for paths, ids and timings. App radius 12 px for cards, 8 px for controls, full round for lenses and pills (the
  site's 21 px stays for marketing blocks). Functional text never below 11 px. Dark only.
- **Motion:** full window subtle (160 ms, transform and opacity only); mini mode lively (the lenses and data flow); nothing moves
  under reduced motion.
- **Code views, by zoom level:** architecture map (blocks with import counts and outside services as dashed nodes: the inputs and
  outputs), then a block's files, then one symbol's callers and callees. Never the whole graph at once; the full "hairball" only
  as ambient decoration in mini mode.
- **Architecture map, blueprint layout (Johan, 17 Nov 2026):** chosen as a blend of four studio directions (`.design-studio/codemap`).
  Three columns: Flows | map | detail. The map sits on a 24 px blueprint grid, with the code in a solid zone and "Calls out" in a
  dashed zone grouped by kind (databases, model runtimes, network and APIs, files and system). `server` and `mcp` packages are
  listed as "Calls in". Blocks carry yellow (`--warn`) memory pills from anchored memories. A flow starts at an entry block and
  follows the heaviest references; the lit path animates with dots, but not under reduced motion. The detail panel shows the
  selected flow or block: its memories first, then its steps or links. All colours, type and radii are this section's tokens; the
  directions' own styles were for choosing structure only. The Layers tab (the Strata direction) reads the same map top to bottom: entry, features, base, outside, each with its links down.

## 10. Decision log
| Date | Decision | Reason | Rejected alternative |
|---|---|---|---|
| 2026-11-16 | Launcher: D1 structure (full window) + D2 lenses (mini mode), family palette and fonts | Johan: D2 innovative but poor in data, D1 has all the information he needs; one look for both modes so the switch feels like one app | D3 Ledger (light paper, serif), D4 Palette (command box first) |
| 2026-11-14 | Base direction D2 "Afterglow", tuned: green-black ground, teal accent, Jakarta + Hanken, radius 21, lively motion | Johan's pick in the Design Studio; D1, D3 and D4 looked too similar to each other | D1 Workbench, D3 Living memory, D4 Paper |
| 2026-11-14 | Slight liquid glass over a data-flow background | Johan's note: Footnote is transparent, the company's knowledge comes into focus | Flat dark cards only; glass everywhere |
| 2026-11-14 | Logo concept A (lens numeral), mark becomes the product | The family stays legible (Footnote) and each product is one glyph | B focus tile as the main logo (kept as the app-icon candidate), C mark system |
| 2026-11-14 | Footnote stays the umbrella name: Library, Memory, Code | Johan's decision after the naming brainstorm | Cairn, Gloss |
| 2026-11-14 | Page v2: stream into a glass lens (hero), interlocking mint blocks, staggered product cards, orbit beside a bleeding glass panel | Johan found v1 too plain; v2 takes structure and motion from LangChain, Mode and Fabric. Johan: "more bold, I like it" | v1 centred hero with flat cards |
| 2026-11-14 | Design status WIP | Johan: decent, will need deeper work later | Treating the first pass as final |
| 2026-11-12 | (superseded 2026-11-14) Paper and ink palette with one proof red, Newsreader, radii 0 and 2 px, no glass | Was the first draft; replaced by the studio lock | |
| 2026-11-12 | (kept) Benchmarks as a table first, a chart second | Numbers must be readable and copyable | Chart-only hero |
| 2026-11-12 | (kept) Self-host fonts | A local-first, no-telemetry product must not call font CDNs | Google Fonts at runtime |
