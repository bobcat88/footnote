---
name: draw-architecture-diagrams
description: Use when drawing or updating an architecture, system context, deployment or workflow diagram, or turning a codebase into one: diagrams as code in LikeC4 (C4 model), real components only, typed elements with technology logos, ownership boundaries, rendered PNG and editable draw.io exports, visual check.
license: MIT
---

# Draw architecture diagrams

## Run
- `likec4 validate <workspace>` → syntax and references
- `likec4 export png <workspace> --flat -o <out>` → images to check and share
- `likec4 export drawio <workspace> --all-in-one --uncompressed -o <out>` → editable hand-off
- `likec4 list-icons --group tech --format json` → exact logo keys
- Pin the CLI version and review it before first use; never `likec4 publish` a private model without permission

## Model
- Read README, AGENTS.md and the code structure first; draw only components that exist
- One model, many views; diagrams are views of the model, never a separate hand-drawn graph
- `specification` (element kinds) / `model` (elements, relations) / `views` in the `.c4` source
- Typed kinds: person / frontend / service / datastore / cache / queue / external
- `technology` on concrete components: framework, runtime, database, broker, format
- Relations say what flows: "reads orders (SQL)", not "uses"
- Several standalone `.c4` files each with a `specification`: validate each in its own temp workspace

## Views
- Context → containers → components; each view answers one question, about 5–15 elements
- Detailed view of a system: `include saas, saas.*` (`include *` alone collapses to top level)
- Ownership or deployment boundary = a nested parent with a soft colour; style the parent (`saas`), not `saas.*`
- External providers stay outside the boundary unless they really belong to it

## Style
- One logo per concrete leaf box; none on conceptual boundaries
- No exact icon → nearest true parent brand, then a neutral icon; keep the `technology` text; never a competitor's logo
- Contrast and readability over brand colours; add styles only when a real need repeats
- D2 for a compact low-level diagram; draw.io only as an export target, not the source

## Check
- Every edit leaves a rendered PNG and one look at it: labels readable / arrows don't cross labels / boundaries visible
- PNG export fails on a missing browser → that's the environment, not the DSL; say so, don't install browsers globally
- Validation passing isn't a visual check
