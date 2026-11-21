---
name: write-modern-css
description: Use when writing or reviewing CSS for any UI, whatever its design language: styling every control past the browser default, tokens, layout-shift hygiene, transitions and micro-interactions (dropdowns, dialogs, toasts, accordions, tabs, skeletons, number changes), scroll and timeline animation, WebGL fit, Material 3 values, and current CSS features with their browser support. Runs design-check's CSS rules.
license: MIT
---

# Write modern CSS

Every input, button, link and call to action gets a deliberate design with rest, hover, pressed, focus-visible and disabled states; nothing ships looking like the browser default. Native CSS first; a framework or animation library only if the project already has one or the effect needs it.

## Check
```
node <skills>/review-ui-quality/scripts/design-check/design-check.mjs <url|file> [--profile generic|material|apple] [--site-type <type>] [--perf] [--out report.json]
```
It runs the craft rules (targets, focus, contrast, alignment, default-looking controls, hover and pressed, transitions) plus these CSS rules:

| Id | Rule | Level |
|---|---|---|
| K0 | Trends file older than 90 days | WARN |
| K1 | Animating `width`, `height`, `top`, `left`, `margin`, `padding` or `font-size`; use `transform` and `opacity` | WARN |
| K3 | More than 12 literal colours, or 6+ with no custom properties | WARN |
| K4 | `100vh` without `dvh` or `svh` | WARN |
| K5 | `@font-face` without `font-display`; images without `width`/`height` or `aspect-ratio` (layout shift) | WARN |
| K6 | p95 frame time over 33 ms (`--perf`) | WARN |
| K7 | WebGL on a site type it doesn't fit, or without fallback, reduced-motion handling and an offscreen pause | FAIL / WARN |

Exit 1 on any FAIL. Each rule has a failing and a clean fixture and a test (`node --test scripts/design-check/tests`). A new rule lands the same way.

## Rules
- **Tokens.** Colours, sizes, radii, durations and easings are custom properties from `DESIGN.md`; components never hold raw values. Build palettes in OKLCH.
- **One easing family, short durations.** Micro-interactions 100–250 ms, overlays about 200 ms, nothing over 300 ms on repeat paths. Enter decelerates, exit accelerates. No overshoot on interface motion.
- **Animate `transform` and `opacity`.** Layout properties only for a deliberate layout change, and then profile it.
- **Reduced motion.** Every animation has a `prefers-reduced-motion: reduce` branch: a fade or an instant change. Nothing auto-plays without a pause.
- **No layout shift.** Reserve space for images, embeds and fonts (`aspect-ratio`, `font-display`, size-adjusted fallbacks).
- **Use the platform.** `<dialog>`, `popover`, `:focus-visible`, `accent-color`, `:has()`, container queries, `@starting-style` for entry animations, view transitions for state and page changes. Check support in the trends file's Baseline table; a "Limited" feature needs `@supports` and a plain fallback.
- **Choose the tool by the motion.** One state change: CSS transition. Several steps, scroll-linked storytelling, SVG morphing or layout-change (FLIP) animation: a timeline library such as GSAP, scoped and cleaned up with the component, with reduced motion honoured in every timeline. Ready-made animated components: name the exact component and variant so it's copied, not invented, and keep a product to 5–8 motion components so it has one motion language.
- **WebGL fits** tech, gaming, creative studios, luxury, automotive, culture and product showcases; it doesn't fit content sites, checkout, docs, government, SaaS conversion pages, accessibility-critical flows or audiences without a GPU. Where it fits: a poster or HTML fallback, real text in the DOM, reduced motion, and a render loop that pauses offscreen and on `visibilitychange`.

## References
- [references/motion-recipes.md](references/motion-recipes.md): CSS for the common transitions (dropdown, dialog, toast, tooltip, accordion, tabs, skeleton, number change, page morph).
- [references/design-trends.md](references/design-trends.md): dated trends and a Baseline support table. Refresh it when its newest source is over 3 months old.
- [references/material.md](references/material.md): Material 3 values the `material` profile checks.
- Apple-style UI: the `apple-style-ui` skill and the `apple` profile.

What stays human: taste, hierarchy, whether a trend fits the brand. That is the critic's job in `review-ui-quality`.
