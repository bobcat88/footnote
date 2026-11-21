---
name: apple-style-ui
description: Use for Apple-inspired interface design or review on any platform, from Apple's public Human Interface Guidelines: hierarchy, Liquid Glass and materials, layout, typography, SF Symbols, motion, accessibility, privacy and widgets, plus an Apple HIG audit of an existing screen. Native Apple apps use system components; web and Windows products get the craft without pretending to be Apple.
license: MIT
---

# Apple-style UI

Re-read the live HIG pages for the surface each project: [Materials](https://developer.apple.com/design/human-interface-guidelines/materials) / [Layout](https://developer.apple.com/design/human-interface-guidelines/layout) / [Typography](https://developer.apple.com/design/human-interface-guidelines/typography) / [Color](https://developer.apple.com/design/human-interface-guidelines/color) / [Motion](https://developer.apple.com/design/human-interface-guidelines/motion) / [Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility) / [Widgets](https://developer.apple.com/design/human-interface-guidelines/widgets) / [SF Symbols](https://developer.apple.com/design/human-interface-guidelines/sf-symbols). No Apple branding, artwork or SF Symbols in a product's identity; no claim of endorsement.

## Keep apart
- Documented guidance / observed behaviour / judgement / project decision: cite facts, label hypotheses
- "Apple-like" ≠ blur + rounded cards + springs. It's hierarchy / restraint / adaptive behaviour / coherent transitions / clear states / legible content
- Liquid Glass = system material. Native → system components. Web `backdrop-filter` = an approximation: opaque readable default, never called Liquid Glass
- No invented spacing scales, easings or "secret" thresholds: HIG values per platform + the project's own tokens

## Before designing
- Inventory: instructions / tokens / real screens + flows / platforms + versions / prior decisions (look at the real surface, not marketing)
- Evidence table: source + date → principle → project adaptation → acceptance test
- States: empty / loading / error / selected / disabled / hover + pressed / focus / reduced motion / long content

## Principles
- **Hierarchy:** primary task + next action obvious / content leads, chrome quiet but clearly interactive / accent, blur, shadow, motion reserved for priority + state
- **Materials:** glass on floating nav + controls, not content cards / dense text on solid / check contrast over light, dark, bright, busy / every glass rule has a `prefers-reduced-transparency` fallback
- **Layout:** from the window or container, not device names / safe areas / resizing / text scaling / RTL / intrinsic sizing over fixed heights
- **Type + icons:** system text styles + Dynamic Type natively / SF fonts never bundled off Apple platforms (`system-ui` or a licensed face) / check the live size table (macOS 13 pt default, 10 pt min; iOS 17 pt, 11 pt) / icons match text weight / icon-only controls named
- **Motion:** explains continuity + cause / never makes people wait / interruptible / CSS: opacity + transform / Reduce Motion gets a non-motion cue
- **Accessibility:** WCAG 2.2 AA (1.4.3 text 4.5:1 / 1.4.11 non-text 3:1 / 2.4.7 + 2.4.11 focus / 2.5.8 targets) / Apple targets in points: iOS 44×44 (min 28), macOS 28×28 (min 20); don't mix units / test with the platform inspector + a screen reader
- **Privacy:** where sensitive data shows: notifications / app switcher / Lock Screen widgets + Live Activities / screenshots / screen sharing. Translucency hides nothing
- **Widgets:** glanceable, one hierarchy, few interactions / test every family, appearance, rendering mode, stale + placeholder states, tap targets on real devices

## Run
`node <skills>/review-ui-quality/scripts/design-check/design-check.mjs <url|file> --profile apple [--input pointer]` → craft rules with Apple targets + **AP1**: every `backdrop-filter` has a `prefers-reduced-transparency: reduce` override (FAIL). `--grid` = the project's own step. Glass fidelity / largest text / screen readers / dark mode stay manual → `review-ui-quality`.

## Report
Functional / accessibility / visual results separately + what wasn't tested. Never claim Liquid Glass parity from CSS, widget readiness from a mock, or Apple quality from dark + rounded.

## Web starter (approximation)
```css
:root { color-scheme: light dark; --surface: Canvas; --text: CanvasText;
  --separator: color-mix(in srgb, CanvasText 16%, transparent); --focus: Highlight; }
.surface { color: var(--text); background: var(--surface); border: 1px solid var(--separator); }
@supports (backdrop-filter: blur(1px)) {
  .floating-controls { background: color-mix(in srgb, var(--surface) 78%, transparent); backdrop-filter: blur(18px) saturate(1.25); } }
@media (prefers-reduced-transparency: reduce) { .floating-controls { background: var(--surface); backdrop-filter: none; } }
:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
```
