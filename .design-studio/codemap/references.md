# Code map redraw: references (17 Nov 2026)

| Reference | What to take | What it shows working |
|---|---|---|
| LikeC4 playground (refs/playground-likec4-dev) | Big titled boxes with a one-line description, dashed relation with its own label chip, quiet dotted canvas, zoom controls bottom-left, breadcrumb "Landscape view" with back/forward | A two-level model drawn from code, drill-down by view |
| LikeC4 site (refs/likec4-dev) | Nested containers: a system box that holds component boxes; the model is code, the picture follows | Model-as-code to diagram |
| IcePanel (refs/icepanel-io) | Many small dark cards with an icon, title, type line and tech line; group frames (Google Cloud, IcePanel.io); every edge labelled with its purpose; "Flows" switch | A living, dense architecture model on a dark canvas |
| repowise (refs/repowise-dev) | One accent colour for "what matters" (orange = what breaks); label beside the node, not inside a legend; restraint | Impact of a change as a picture |
| Structurizr (refs/structurizr-com) | C4 discipline: every box says what it is and its technology; every arrow says what it does | Plain, label-first C4 |

Not taken: the IcePanel mascot, any brand colours, logos or copy.

## Data used in every mock (real, this repository, `footnote code-map --depth 2`)
Blocks: src 14 files 453 symbols; crates/search 7/181; tests 29/122; crates/core 6/79; crates/code 2/49; crates/alias 2/46;
crates/library 3/41; crates/memory 2/27.
Links (references): src→core 86, src→search 55, search→core 41, src→memory 20, library→core 14, memory→search 12,
src→library 8, library→search 4, src→code 4, code→core 2, alias→core 1, memory→core 1.
Outside: serde_json 1005, assert_cmd 597 (tests), anyhow 487, rusqlite 223 (database), tempfile 122, tantivy 82 (database),
url 55 (network), ureq 47 (network), time 33, ort 19 (model).
Memory anchors (Footnote's advantage): e.g. 3 memories on crates/code (scip-python Windows fix, parser-base decision,
scip-java blocked), 2 on src (usage limits, test speed).

## Divergence matrix

| Row | D1 Model view | D2 Living canvas | D3 Flow and memory | D4 Strata |
|---|---|---|---|---|
| Idea | LikeC4-faithful C4: system box holding component boxes, drill-down by view | IcePanel-like working canvas: many small cards, groups, every edge says what it does | Footnote's own: references flow as moving dots, memories pinned to blocks | Not a node graph at all: layers stacked by dependency depth, links as bands between strata |
| Theme | Dark slate, large filled cards | Near-black, card outlines only | Locked launcher green-black (brand kept) | Warm dark paper, ink-like |
| Accent | Cobalt blue | Amber | Teal (locked) | Vermilion |
| Palette strategy | One saturated fill for project blocks, grey for outside | Monochrome cards, accent only on the selected path | Brand neutrals, accent as light along links | Two-tone: ink + one warning colour for cycles and heavy links |
| Type voice | Geometric sans, centred titles | Small UI sans, dense labels, mono tech line | Plus Jakarta Sans titles, mono counts | Serif titles (Newsreader), mono numbers, editorial |
| Shape | Large 12 px rounded rectangles, dashed links | 6 px cards, curved orthogonal edges, group frames | Soft 20 px tiles, pill pins | Full-width bands, no boxes, hairline rules |
| Density | Sparse: 8 blocks, room to breathe | Dense: blocks + outside services + labels all at once | Medium | Medium-high, table-like |
| Motion | Zoom transition on drill-down only | Hover lights a path, nothing moves at rest | Dots flow along links at a speed set by references; pins pulse once when a memory lands | None at rest; a cycle or heavy link draws in on load |
| How content is shown | Each box: name, one-line purpose, files and symbols; relation chip "86 refs · config, errors" | Cards with icon, kind, tech; outside services in their own group frame; flows panel | Map left, inspector right with the block's memories, docs and top links | Rows top to bottom: entry (src, tests), features (search, library, memory, code, alias), base (core), outside (rusqlite, tantivy, ort…) |
