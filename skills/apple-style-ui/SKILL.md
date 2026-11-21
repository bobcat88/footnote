---
name: apple-style-ui
description: Use for Apple-inspired interface design or review on any platform, from Apple's public Human Interface Guidelines: hierarchy, Liquid Glass and materials, layout, typography, SF Symbols, motion, accessibility, privacy and widgets, plus an Apple HIG audit of an existing screen. Native Apple apps use system components; web and Windows products get the craft without pretending to be Apple.
license: MIT
---

# Apple-style UI

Work from Apple's public HIG and developer documentation, re-checked live for each substantive project ([references/hig-sources.md](references/hig-sources.md)). This is an evidence-based method: no claim of Apple endorsement or internal rules, and no Apple branding, artwork or SF Symbols in a product's identity.

## Keep these apart
- **Documented guidance**, **observed product behaviour**, **design judgement** and **project decisions**. Cite each fact; label hypotheses.
- "Apple-like" isn't blur, rounded cards and springs. The craft is hierarchy, restraint, adaptive system behaviour, coherent transitions, clear states and legible content.
- **Liquid Glass is a system material**, not a CSS recipe. On Apple platforms use the native components. A web `backdrop-filter` is an approximation: say so, keep an opaque readable default, and never call it Liquid Glass.
- Don't invent Apple spacing scales, easing curves or "secret" thresholds. Use the HIG's platform-specific values and measure the project's own tokens.

## Before designing
1. Inventory the project: instructions, tokens, real screens and flows, supported platforms and versions, prior decisions. Look at the actual target surface, not a marketing image.
2. Read the HIG pages for the surface (Materials, Liquid Glass adoption, Layout, Typography, Color, Motion, Accessibility, Widgets, SF Symbols) and the API docs for native work.
3. Keep a small evidence table: source and date → principle → how this project adapts it → acceptance test.
4. Define the states before building: empty, loading, error, selected, disabled, hover and pressed, keyboard focus, reduced motion, long content.

## Principles
- **Hierarchy.** The primary task and next action are obvious at first view. Content leads; chrome is quiet but clearly interactive. Accent, tint, blur, shadow and motion are reserved for priority and state.
- **Materials.** Glass belongs to the floating navigation and control layer, not to content cards. Dense text gets a regular or solid surface. Check contrast after compositing over light, dark, bright and busy backgrounds; blur is not a contrast strategy. Every glass rule has a `prefers-reduced-transparency` fallback.
- **Layout.** Design from the available window or container, not device names. Respect safe areas, resizing, text scaling, localisation and RTL. Prefer intrinsic sizing to fixed heights that clip.
- **Type and icons.** System text styles and Dynamic Type on Apple platforms. SF fonts are licensed for Apple platforms and mockups only: never bundle them; elsewhere use `system-ui` stacks or a licensed face. Platform minimums differ (the HIG currently lists macOS 13 pt default and 10 pt minimum, iOS 17 pt and 11 pt); check the live table. Icons match the adjacent text's weight; icon-only controls have accessible names.
- **Motion.** It explains continuity, state and cause; it never makes people wait and is always interruptible. Prefer system transitions and springs natively; in CSS animate opacity and transform. Honour Reduce Motion with a non-motion cue for the state.
- **Accessibility.** WCAG 2.2 AA: contrast 4.5:1 (1.4.3) and 3:1 non-text (1.4.11), focus visible (2.4.7) and not obscured (2.4.11), target size (2.5.8). Apple's target sizes are separate and in points (iOS 44×44 default, 28×28 minimum; macOS 28×28 default, 20×20 minimum); don't mix units. Test with the platform's accessibility inspector and a screen reader; a DOM scan is a filter, not sign-off.
- **Privacy.** Map where sensitive data can show: notifications, app switcher, Lock Screen widgets and Live Activities, screenshots, shared screens. Translucency doesn't hide data.
- **Widgets.** A glanceable extension of the app with one hierarchy and few focused interactions. Test each supported family, appearance and rendering mode (HIG appearances and WidgetKit rendering modes are different things), stale and placeholder states, and tap destinations, on real devices.

## Check
```
node <skills>/review-ui-quality/scripts/design-check/design-check.mjs <url|file> --profile apple [--input pointer]
```
The craft rules with Apple target sizes (`--input pointer` for macOS-style UI), plus **AP1**: every `backdrop-filter` rule has a `prefers-reduced-transparency: reduce` override (FAIL). Apple publishes no universal spacing scale, so `--grid` is the project's own step. Glass fidelity, largest text sizes, screen readers and dark mode stay manual; run `review-ui-quality` for the full loop.

## Report
Functional, accessibility and visual results separately, with what wasn't tested. Never claim Liquid Glass parity from CSS, widget readiness from a static mock, or Apple quality from a dark palette and rounded corners.

## Web starter (an approximation, not native material)
```css
:root { color-scheme: light dark; --surface: Canvas; --text: CanvasText;
  --separator: color-mix(in srgb, CanvasText 16%, transparent); --focus: Highlight; }
.surface { color: var(--text); background: var(--surface); border: 1px solid var(--separator); }
@supports (backdrop-filter: blur(1px)) {
  .floating-controls { background: color-mix(in srgb, var(--surface) 78%, transparent); backdrop-filter: blur(18px) saturate(1.25); }
}
@media (prefers-reduced-transparency: reduce) { .floating-controls { background: var(--surface); backdrop-filter: none; } }
:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
```
Starter values, not Apple specifications: measure contrast over real backgrounds.
