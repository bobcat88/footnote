# Launcher design: initial analysis (P1, M2 step 1)

15 November 2026. Initial design analysis inputs before locking `DESIGN.md`; no visual decisions are taken here. Plan:
[P1-PLAN.md](../roadmap/P1-PLAN.md). Brand contract: [DESIGN.md](../../DESIGN.md).

## 1. What is already locked

The product site's direction (DESIGN.md, D2 "Afterglow", 14 Nov) is the family's identity, and the launcher inherits it:
dark first (ground #09120B, surface #121C15, teal accent #2B9185, glow #5FD3C3, mint blocks), Plus Jakarta Sans 600 for display,
Hanken Grotesk for text, JetBrains Mono for paths, hashes and numbers, radius 21 px for blocks and cards, slight liquid glass on
chosen elements only, a slow field of flowing dots as the data metaphor, self-hosted fonts. Logo: concept A, the lens numeral;
the Library and Memory marks are not chosen yet.

A site is read once; a launcher is used daily. What changes is density, motion budget and controls, not the identity.

## 2. T3 Code study (MIT; source read on GitHub, nothing installed or copied)

Stack: a React web app (Tailwind v4 with `@theme` tokens, Base UI, Lucide icons) inside Electron. Files read:
`apps/web/src/index.css`, `appearanceFonts.ts`, `components/Sidebar.motion.ts`, `components/chat/MarkdownFileLink.tsx`,
`MarkdownExternalLink.tsx`, `packages/shared/src/themePalettes.ts` and `themeCss.ts`.

| Area | What T3 Code does | Take for Footnote |
|---|---|---|
| Geometry | Semantic tokens so controls cannot drift apart: `--control-radius` 0.5 rem, sidebar insets 0.5 rem, row inset 0.625 rem, top bar 52 px, chat column 46 rem, details panel 17.5 rem. One base radius (0.5 rem) and a scale from it (sm -4 px to 3xl +12 px) | The same discipline: one radius base, a scale, named insets. Ours keeps 21 px for cards and needs a smaller control radius (to choose) |
| Type | System stack (-apple-system, Inter Variable, Segoe UI); extra dense sizes for metadata (11 px, 10 px); interface, code and prompt sizes adjustable in Settings > Appearance | Dense metadata sizes and user-adjustable sizes (accessibility). Keep our families |
| Depth | Glass only on chrome: blur 12 px (16 px dark), 80 % opacity, saturation 1.14 (1.08 dark), with a no-blur fallback; soft composer shadows; a 1 px inset highlight on controls | Matches DESIGN.md "glass on chosen elements only": chrome and the lens, never content |
| Motion | 150 ms ease-out for list rows (positions animated with the Web Animations API), travel capped at 40 px, nothing under reduced motion; a decelerating curve `cubic-bezier(0.32, 0.72, 0, 1)` for drawers; view transitions | Short, functional motion for the app; the dot field stays decorative and optional |
| Links in chat | No underline, information colour; hover or focus reveals a dotted underline (dots every 4 px); external links show the site's favicon inline at 14 px (globe fallback); file links are inline chips with the file or folder icon, ellipsis, selectable text | The Activity and Memory screens: each memory links to its project, file or source with these inline chips |
| Themes | Palettes as data compiled to CSS variables, plus contrast settings | The future light theme as a second palette, not a second stylesheet |

Not taken: their fonts, colours, icons, assets or copy. We did not check which surfaces use which radius step, so the "rounded
look" preference should be judged on screenshots during the design pass, not inferred from tokens.

## 3. Screens from the engine commands (M1)

| Screen | Data it shows | Command |
|---|---|---|
| Overview | One card per product (on or off, health); this week: memories saved, prompts given memories; agents connected; models in use | `footnote status`, `config set memory.enabled` |
| Memory | Its options with gain and cost; locked ones with the company's reason; uninstall | `footnote options`, `config`, `uninstall --memory` |
| Models | This machine (CPU threads, RAM, every GPU with its real VRAM), the recommendation with its reasons and sources, installed models | `footnote models recommend/list/install/use/remove` |
| Activity | What memory saved and recalled, linked to projects (the visibility principle) | **missing** |
| Settings | Every option, policy banner, metrics on or off | `footnote options` |

Gaps found for the engine, before the window:
1. **Browse memories:** a paged list by project and kind, with forget. Recall and forget exist; a plain list does not.
2. **Activity feed:** saved memories have dates; recalls are only counted from the metrics file, and the hook keeps no prompt
   text (zero retention). An activity feed can show counts and which memories were injected, never what was asked.
3. **Progress for long steps:** model downloads and indexing need progress lines the window can follow.
4. **Library:** corpus status (sources, freshness) for its card; later documentation ingestion (backlog item 14).

## 4. Open questions settled for design

**Answered 15 November:** (1) a compact tray panel plus the full window; (2) T3 Code's design language first, adjusted only where
it does not suit Footnote; (3) the dot field in the **header only**;
(4) dark only for now; (5) marks for every product and a rule for
future members of the family, so the question is never asked again.


1. **Window shape:** a compact panel from the tray (quick switch and this week's activity) plus a full window, or the full
   window only.
2. **Density:** compact like T3 Code (small control radius, dense rows) or closer to the site (21 px everywhere, more air).
3. **Dot field in the app:** off, on the Overview header only, or everywhere (a constant animation costs GPU and attention).
4. **Light theme:** now, as a second palette, or later as DESIGN.md says.
5. **Marks:** Library and Memory marks (concept A lenses) to settle in the same session as the icons.

## 5. One design language for the whole family (15 Nov)

The launcher's language is the family's, not the launcher's: the coming Footnote Code graph visualizer (and later the project
node map and the visual roadmap) must read as the same product. So the studio locks a shared system, not one screen:

- **Shared tokens and components** (colours, type, radius scale, insets, glass, motion curves, link chips), used by
  the launcher, the site and every later app; each app adds only what its content needs.
- **Graph primitives decided once:** node (a card at the card radius, the product's lens mark for its kind), edge (hairline in
  the line colour, glow colour when it carries the current path), focus (the glass lens magnifies the selected node and its
  neighbours), clusters (mint blocks), labels in JetBrains Mono for symbols and paths.
- **Motion shared with the data metaphor:** the flowing dots of the site become what moves along edges (a call path, a recall
  from memory to answer), short layout transitions (the 150 ms rule) and nothing under reduced motion.
- **Same link chips everywhere:** a symbol, file or memory opens the same way in the chat, the launcher and the graph.

The studio session therefore mocks one launcher screen and one graph view per direction, so a direction is judged on both.

## 6. Four directions to mock

| Direction | Idea |
|---|---|
| Console | T3-like: sidebar of products, dense content, glass top bar; the graph sits in the content pane; the most familiar |
| Lens | The Overview is the logo: one glass lens per product, activity orbiting it; the graph is navigated by lens focus; the most on-brand |
| Palette | Raycast-like: a command palette first, panels for each product; the fastest daily use |
| Ledger | Activity first: a timeline of what memory did, filtered by product and project; visibility as the home screen |

Next: the design studio run (references, four mocks, locked DESIGN.md section for the launcher), then
the engine gaps of section 3, then M3.
