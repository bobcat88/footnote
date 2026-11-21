---
name: write-modern-css
description: Use when writing or reviewing CSS for any UI, whatever its design language: styling every control past the browser default, tokens, layout-shift hygiene, transitions and micro-interactions (dropdowns, dialogs, toasts, accordions, tabs, skeletons, number changes), scroll and timeline animation, WebGL fit, Material 3 values, and current CSS features with their browser support. Runs design-check's CSS rules.
license: MIT
---

# Write modern CSS

Every input, button, link and CTA: deliberate rest / hover / pressed / focus-visible / disabled. Nothing ships looking like the browser default. Native CSS first; a library only if the project has one or the effect needs it.

## Run
`node <skills>/review-ui-quality/scripts/design-check/design-check.mjs <url|file> [--profile generic|material|apple] [--site-type <type>] [--perf] [--out report.json]` → craft rules + CSS rules below, exit 1 on FAIL. Tests: `node --test <skills>/review-ui-quality/scripts/design-check/tests`; a new rule lands with a failing + clean fixture + test.

| Id | Rule | Level |
|---|---|---|
| K0 | trends file older than 90 days | WARN |
| K1 | animating width / height / top / left / margin / padding / font-size → transform + opacity | WARN |
| K3 | > 12 literal colours, or 6+ with no custom properties | WARN |
| K4 | `100vh` without `dvh` / `svh` | WARN |
| K5 | `@font-face` without `font-display` / images without size or `aspect-ratio` | WARN |
| K6 | p95 frame time > 33 ms (`--perf`) | WARN |
| K7 | WebGL on a site type it doesn't fit / no fallback, reduced motion or offscreen pause | FAIL / WARN |

## Rules
- **Tokens:** colours / sizes / radii / durations / easings as custom properties from DESIGN.md; no raw values in components; palettes in OKLCH
- **Timing:** one easing family / micro 100–250 ms / overlays ~200 ms / ≤ 300 ms on repeat paths / enter decelerates, exit accelerates / no overshoot
- **Properties:** transform + opacity; layout properties only for a deliberate layout change, profiled
- **Reduced motion:** every animation has a `reduce` branch (fade or instant) / nothing auto-plays without pause
- **No layout shift:** `aspect-ratio` / `font-display` / size-adjusted fallbacks
- **Platform first:** `<dialog>` / `popover` / `:focus-visible` / `accent-color` / `:has()` / container queries / `@starting-style` / view transitions. "Limited" Baseline → `@supports` + plain fallback
- **Tool by motion:** one state change → CSS transition / multi-step, scroll story, SVG morph, FLIP → timeline library (GSAP) scoped + cleaned up with the component, reduced motion in every timeline / ready-made components: name exact component + variant / 5–8 motion components per product
- **WebGL fits** tech / gaming / creative studios / luxury / automotive / culture / product showcases; **not** content sites / checkout / docs / government / SaaS conversion / accessibility-critical / no-GPU audiences. Where it fits: poster or HTML fallback / real DOM text / reduced motion / pause offscreen + on `visibilitychange`

## References
- [motion-recipes.md](references/motion-recipes.md): dropdown / dialog / toast / tooltip / accordion / tabs / skeleton / number change / page morph
- [design-trends.md](references/design-trends.md): dated trends + Baseline support table; refresh when > 3 months old
- [material.md](references/material.md): Material 3 values for `--profile material`
- Apple-style: `apple-style-ui` + `--profile apple`

Taste, hierarchy, trend-vs-brand fit stay with the critic in `review-ui-quality`.
