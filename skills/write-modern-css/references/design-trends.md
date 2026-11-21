# Design and CSS trends, last 12 months

Window of this file: 2025-11-20 to 2026-11-20 (researched 2026-11-20).
**Expiry rule:** this file is dated. When the newest source is older than 3 months, or a task starts more than 12 months after the window end, refresh it before using it as "current" (sources: web.dev monthly posts and Baseline digests, developer.chrome.com, webkit.org blog, Firefox release notes, CSS-Tricks, Smashing, Awwwards, Codrops, Dribbble). Replace rows, keep the dates.
**Use:** native CSS first. Check Baseline status in Table B before shipping a feature without a fallback. Rows marked "Limited" need `@supports` and a plain fallback.

---


## Table A. Visual and interaction trends

| Trend | What it is | CSS technique, native first | Best for | Avoid when | Source URL | Source date |
|---|---|---|---|---|---|---|
| Scroll-driven reveals and staggers | Element animation progress is tied to scroll or view position instead of time. | `animation-timeline: view()`; stagger with `animation-range-start: contain calc(10% + 2.5% * sibling-index())` | Editorial, storytelling, product tours | No fallback for non-supporting browsers; no `prefers-reduced-motion` gate | https://css-tricks.com/staggering-scroll-driven-animations-with-animation-range-start/ | 2026-11-16 |
| Scroll-triggered time-based animation | A time-based animation plays once a scroll boundary is crossed (not scrubbed). | `timeline-trigger` + `animation-trigger` (Chrome 146 only) | Entrance effects that should not scrub | Cross-browser requirement (Chrome-only at last check) | https://developer.chrome.com/blog/new-in-web-ui-io26 | 2026-07-01 |
| Index-driven stagger without JS | Delay or offset computed from an element's sibling position. | `animation-delay: calc(sibling-index() * 0.05s + 0.2s)` | Menus, dialog contents, card grids | Lists that reorder often and must not re-animate | https://developer.chrome.com/blog/new-in-web-ui-io26 | 2026-07-01 |
| View-transition morphs between states/pages | Matching `view-transition-name` elements animate between DOM states or documents. | `::view-transition-*`, `view-transition-name`, `@view-transition`; element-scoped `element.startViewTransition()` (Chrome 147) | Galleries, list-to-detail, multi-page sites | Large DOM snapshots on low-end devices; transitions longer than the task needs | https://developer.chrome.com/blog/new-in-web-ui-io26 | 2026-07-01 |
| Native styled form controls | `<select>` and its picker fully restyled without JS widgets. | `select, ::picker(select) { appearance: base-select }` inside `@supports (appearance: base-select)` | Forms, filters, settings | Safari/Firefox parity needed without a fallback (see Table B) | https://smashingmagazine.com/2025/12/state-logic-native-power-css-wrapped-2025/ | 2025-12-09 |
| Native carousel controls | Scroll containers get generated navigation dots and arrows. | `scroll-marker-group: after`, `::scroll-marker`, `::scroll-button(inline-end)` | Galleries, tab-like strips | Content that must work in Safari/Firefox today (not verified there) | https://smashingmagazine.com/2025/12/state-logic-native-power-css-wrapped-2025/ | 2025-12-09 |
| Corner shapes beyond rounding | Squircle, bevel, scoop, notch corners on boxes. | `border-radius: 25px; corner-shape: squircle` (or `superellipse(n)`) | Buttons, cards, brand-forward UI | Need identical rendering in Safari/Firefox (article: Chromium only) | https://www.smashingmagazine.com/2026/03/beyond-border-radius-css-corner-shape-property-ui/ | 2026-03-12 |
| Non-rectangular borders | Borders and shadows follow a custom shape. | `border-shape` (Chrome 147), `shape()` (Baseline Newly, Feb 2026) | Badges, callouts, decorative frames | Interactive targets with clipped focus rings | https://web.dev/blog/web-platform-04-2026 | 2026-04-24 |
| Bento grids | Asymmetric tile layouts for dense information. | CSS Grid with `grid-template-areas`, `subgrid`; collapse to one column on small screens | Feature overviews, dashboards, SaaS landing | Mobile order puts the hero tile after minor ones; screen readers follow DOM order, not visual order | https://www.netguru.com/blog/top-web-design-trends | updated 2026-10-16 |
| Layouts off the 12-column default | Varying column proportions, modular and compound grids for hierarchy. | Grid with custom track ratios, stacked grids with different column counts | Brand and editorial sites | Dense data UIs that need regular rhythm | https://css-tricks.com/beyond-12-columns/ | 2026-11-12 |
| Masonry via grid lanes | Native masonry-style layout. | `display: grid-lanes` (Safari 26.4; Firefox removed old `masonry` syntax in 159) | Image walls, card feeds | Chrome/Firefox support is absent or unverified (see Table B) | https://web.dev/blog/web-platform-03-2026 | 2026-03-31 |
| Oversized type as hero | Large headline replaces stock hero image. | `clamp()` fluid type, variable fonts, `text-box-trim`, `text-fit` (Chrome 150 only) | Brand, editorial, portfolios | Line25 rates it "Normalizing" in 2026, so it no longer differentiates alone | https://line25.com/articles/web-design-trends-2026/ | 2026-04 (page shows 04-16 and 04-23) |
| Restrained kinetic type | Moving text limited to one line. | Scroll-driven or `@keyframes` on `transform`/`opacity`; avoid animating variable-font axes | Hero lines, section openers | More than one hero line; text moving >5 s without a pause control (WCAG 2.2.2); axis animation causes reflow/CLS | https://www.netguru.com/blog/top-web-design-trends | updated 2026-10-16 |
| Neo-brutalism / anti-design | Thick outlines, hard shadows, raw layout as a human-made signal. | `border: 3px solid`, hard `box-shadow` with zero blur, system or mono fonts | Creative studios, indie products | Conversion/checkout pages; saturated pairs can fail contrast (pure yellow on white well under 2:1 per source) | https://www.designrush.com/agency/website-design-development/trends/web-design-trends | published 2025-05-26, updated 2026-10-20 |
| Restrained glass ("glassmorphism 2.0") | Translucency and blur used to show depth hierarchy, not decoration. | `backdrop-filter: blur()` on chrome only; solid fallback under `@supports not` | Navigation, overlays, toolbars | Over text-heavy content; performance-limited devices; SVG-filter refraction variants are Chromium-only per secondary sources | https://line25.com/articles/web-design-trends-2026/ | 2026-04 |
| Playful imperfection | Random tilt/offset on decorative elements. | `rotate: random(-2.5deg, 2.5deg)` inside `@supports (rotate: random(-2deg, 2deg))` | Decorative badges, stickers, ambient motion | Core layout, body text, interactive targets; reduced-motion users | https://www.smashingmagazine.com/2026/10/embracing-playful-imperfection-css-random-function/ | 2026-10-20 |
| Theme-aware color systems | Colors react to scheme and contrast automatically. | `light-dark()` (images too), `contrast-color()`, `color-mix()`, relative `alpha()` | Any site with dark mode | Contrast checks skipped: `contrast-color()` returns only black or white | https://web.dev/blog/web-platform-04-2026 | 2026-04-24 |
| Component state via style queries | Descendants restyle from a parent's custom property. | `@container style(--theme: primary) { ... }` | Design-token themes, card variants | Treating it as free: container queries change formatting context (margin collapse, zero-width collapse) | https://www.smashingmagazine.com/2026/11/modern-css-might-make-your-front-end-more-complex/ | 2026-11-03 |
| Declarative light-dismiss overlays | Dialogs close on outside click or Esc without JS. | `<dialog closedby="any">`, popover API | Modals, menus, tooltips | Destructive confirmations that must not dismiss accidentally | https://developer.chrome.com/blog/new-in-web-ui-io26 | 2026-07-01 |
| WebGL layer synced to DOM | Canvas draws images/effects; real text stays in the DOM. | Real text in DOM at `opacity: 0` while canvas renders; Lenis smooth scroll plus a shared clock (per Codrops search snippet) | Luxury, automotive, architecture, culture, games/tech | SaaS/content sites, conversion pages, no-GPU devices (see WebGL section) | https://digitalstrategyforce.com/journal/why-are-immersive-experiences-dominating-the-2026-awwwards/ | no explicit date; refers to H1 2026 |
| Gutter and divider styling | Rules drawn between grid/flex items natively. | `column-rule` / `row-rule` gap decorations (Chrome 149) | Card grids, tables of tiles | Firefox/Safari need a pseudo-element fallback | https://web.dev/blog/web-platform-06-2026 | 2026-06-30 |

## Table B. CSS and platform features shipped in the last 12 months

Status labels are taken from the cited page. "Not stated" means the page gave no Baseline label; "Limited (inferred)" is my reading of the cited per-browser support, not a label from the source.

| Feature | Browsers and Baseline status | One-line example | Source URL |
|---|---|---|---|
| Scroll-driven animations | Firefox 159 enabled by default (2026-10-27); Chrome/Edge 115, Safari 26.x. Baseline Newly available (web.dev, 2026-10-29) | `animation-timeline: view();` | https://web.dev/blog/web-platform-10-2026 |
| Scroll-triggered animations | Chrome 146 (stable 2026-03); Firefox/Safari not stated | `timeline-trigger: --t view() contain 25% contain 75%` | https://web.dev/blog/web-platform-03-2026 |
| `:active-view-transition` | Baseline Newly available, Jan 2026 digest (pub. 2026-03-02) | `:root:active-view-transition { ... }` | https://web.dev/blog/baseline-digest-jan-2026 |
| View transition types | Firefox 147 (same-document only); Baseline status not stated | `html:active-view-transition-type(slide)` | https://web.dev/blog/web-platform-01-2026 |
| Element-scoped view transitions | Chrome/Edge 147; Firefox, Safari not supported; status not stated | `el.startViewTransition(() => update())` | https://web.dev/blog/web-platform-04-2026 |
| Anchor positioning | Firefox 147 shipped (2026-01-30); Baseline Newly available. Safari 27.0 adds transform-aware anchoring and `position-anchor: normal/none` | `position-anchor: --btn; position-area: block-end;` | https://web.dev/blog/web-platform-01-2026 |
| `field-sizing` | Firefox 152 completes engines; Baseline Newly available (digest pub. 2026-07-17) | `textarea { field-sizing: content; }` | https://web.dev/blog/baseline-digest-jun-2026 |
| Customizable `<select>` | Chrome (listbox mode Chrome 145), Safari 27.0 (2026-09-14), Safari 27.2 refinements (2026-10-26); Firefox 153 only parses nested elements. Limited (inferred) | `select { appearance: base-select; }` | https://webkit.org/blog/18515/webkit-features-for-safari-27-2/ and https://web.dev/blog/web-platform-07-2026 |
| `sibling-index()` / `sibling-count()` | Firefox 154 (post pub. 2026-08-28) says Baseline Newly available | `animation-delay: calc(sibling-index() * 0.1s)` | https://web.dev/blog/web-platform-08-2026 |
| `corner-shape` | Chrome 139+/Chromium only per Smashing (2026-03-12); Chrome IO post calls it experimental. Limited (inferred) | `corner-shape: squircle;` | https://www.smashingmagazine.com/2026/03/beyond-border-radius-css-corner-shape-property-ui/ |
| `if()` | Chrome (version not stated in opened page); Firefox unsupported per search snippet only. Limited (inferred) | `color: if(style(--c: white): black; else: white);` | https://developer.chrome.com/blog/new-in-web-ui-io26 |
| `@function` | Chrome; version not stated | `@function --double(--x) { result: calc(var(--x) * 2); }` | https://developer.chrome.com/blog/new-in-web-ui-io26 |
| Container style queries (custom properties) | Firefox 151 completes engines; Baseline Newly available (digest pub. 2026-06-03) | `@container style(--theme: dark) { ... }` | https://web.dev/blog/baseline-digest-may-2026 |
| Name-only container queries | Chrome 148, Firefox 149, Safari 26.4; Baseline Newly available | `@container card { ... }` | https://web.dev/blog/web-platform-05-2026 |
| `:open` pseudo-class | Safari 26.5; Baseline Newly available (2026-05-29) | `dialog:open { ... }` | https://web.dev/blog/web-platform-05-2026 |
| Popover `hint` | Firefox 149; Chrome/Safari status not stated in opened page | `<div popover="hint">` | https://web.dev/blog/web-platform-03-2026 |
| `closedby` on dialog | Chrome (per IO26 post); Firefox/Safari not verified | `<dialog closedby="any">` | https://developer.chrome.com/blog/new-in-web-ui-io26 |
| Grid lanes (`display: grid-lanes`) | Safari 26.4; Chrome/Edge/Firefox not supported at 2026-03-31; Firefox 159 removed legacy `masonry`. Limited | `display: grid-lanes;` | https://web.dev/blog/web-platform-03-2026 |
| `shape()` | Baseline Newly available, Feb 2026 digest (pub. 2026-03-30); Firefox 148 | `clip-path: shape(from 0 0, line to 100% 0, ...)` | https://web.dev/blog/baseline-digest-feb-2026 |
| `rect()` / `xywh()` | `rect()` Baseline Widely available per Aug digest; `shape-outside` variants Chrome 149 | `clip-path: rect(0 100% 100% 0 round 8px)` | https://web.dev/blog/baseline-digest-aug-2026 (see Sep) and https://web.dev/blog/web-platform-06-2026 |
| `contrast-color()` | Baseline Newly available (digest pub. 2026-05-27); Chrome 147 | `color: contrast-color(var(--bg));` | https://web.dev/blog/baseline-digest-apr-2026 |
| `alpha()` relative color | Chrome 151, Firefox 155, Safari 27: Baseline Newly available | `color: alpha(from var(--c) / 0.5)` | https://web.dev/blog/web-platform-09-2026 |
| `light-dark()` with images | Chrome 150, Firefox 150, Safari 27: Baseline Newly available | `background: light-dark(url(a.png), url(b.png))` | https://web.dev/blog/web-platform-09-2026 |
| `progress()` | Chrome 138, Firefox 155, Safari 27: Baseline Newly available | `opacity: progress(100vw, 400px, 1200px)` | https://web.dev/blog/web-platform-09-2026 |
| `revert-rule` | Chrome 148, Firefox 150, Safari 27: Baseline Newly available | `color: revert-rule;` | https://web.dev/blog/web-platform-09-2026 |
| CSS typed arithmetic in `calc()` | Chrome 140, Firefox 158, Safari 26.x: Baseline Newly available | `width: calc(100vw / 16px)` returns a number | https://web.dev/blog/web-platform-10-2026 |
| `random()` | Chrome 156/Edge 156, Safari first; Firefox Technology Preview; status not stated | `rotate: random(-3deg, 3deg);` | https://www.smashingmagazine.com/2026/10/embracing-playful-imperfection-css-random-function/ |
| Gap decorations | Chrome 149 only | `column-rule: 1px solid;` on grid/flex | https://web.dev/blog/web-platform-06-2026 |
| `text-box-trim` / `text-box` | Firefox 154: Baseline Newly available | `text-box: trim-both cap alphabetic;` | https://web.dev/blog/web-platform-08-2026 |
| `text-fit` | Chrome 150 only | `text-fit: grow;` (syntax not verified) | https://web.dev/blog/web-platform-06-2026 |
| `background-clip: border-area` | Chrome 150 only | gradient borders | https://web.dev/blog/web-platform-06-2026 |
| `@scope` article, popover API guides | Authoring guides, not new shipping events | `@scope (.card) { ... }` | https://www.smashingmagazine.com/2026/02/css-scope-alternative-naming-conventions/ (listing only; body not opened) |
| `:has()` | Baseline Widely available (digest 2026-07-17) | `.card:has(img)` | https://web.dev/blog/baseline-digest-jun-2026 |
| `animation-composition` | Baseline Widely available (digest 2026-03-02) | `animation-composition: add;` | https://web.dev/blog/baseline-digest-jan-2026 |
| `linear()` easing / `<easing-function>` | Baseline Widely available (digest 2026-07-17) | `transition-timing-function: linear(0, 0.5, 1)` | https://web.dev/blog/baseline-digest-jun-2026 |
| Subgrid, `contain-intrinsic-size`, `hyphens` | Baseline Widely available (digest 2026-04-14) | `grid-template-rows: subgrid;` | https://web.dev/blog/baseline-digest-mar-2026 |
| Scroll anchoring (`overflow-anchor`) | Safari 27 completes engines: Baseline Newly available | `overflow-anchor: none;` | https://web.dev/blog/web-platform-09-2026 |
| Scroll-state queries | Chrome 133; Smashing (2025-12-09) | `@container scroll-state(stuck: top) { ... }` | https://smashingmagazine.com/2025/12/state-logic-native-power-css-wrapped-2025/ |
| `sizes="auto"` on lazy images | Safari 27, Chrome 126, Firefox 150; Newly available | `<img loading="lazy" sizes="auto">` | https://web.dev/blog/web-platform-09-2026 |

## Table C. Animation and transition guidance

| Topic | Guidance with number | Source URL | Source date |
|---|---|---|---|
| Popover/overlay reveal | `transition-duration: 0.2s`, `ease-out`, on `opacity`, with `transition-behavior: allow-discrete` and `overlay` for top-layer elements | https://developer.chrome.com/docs/modern-web-guidance/guides/ui-behaviors/motion | date not shown on page |
| Continuous loaders | Spinner `1s linear infinite`; indeterminate bar `1s infinite linear`, slowed to `20s` under reduced motion | same | not shown |
| Safe properties | Animate `opacity` and `transform` (incl. individual `translate`, `rotate`, `scale`); avoid `left/right/top/bottom` | same | not shown |
| Entry/exit | `@starting-style` + `transition-behavior: allow-discrete` for `display` changes | https://developer.chrome.com/blog/new-in-web-ui-io26 | 2026-07-01 |
| Stagger step | `calc(sibling-index() * 0.05s + 0.2s)` (Chrome IO26); `* 0.1s` (Smashing); scroll stagger step 2.5% of range | https://developer.chrome.com/blog/new-in-web-ui-io26 ; https://css-tricks.com/staggering-scroll-driven-animations-with-animation-range-start/ | 2026-07-01 ; 2026-11-16 |
| Scroll stagger caveat | `animation-delay` has no effect on scroll-driven animations; use `animation-range-start`; `animation-timing-function` applies to the whole timeline | https://css-tricks.com/staggering-scroll-driven-animations-with-animation-range-start/ | 2026-11-16 |
| Custom easing | `linear()` for bounce/spring curves; Baseline Widely available | https://web.dev/blog/baseline-digest-jun-2026 | 2026-07-17 |
| Reduced motion, strategy | Remove: wrap in `@media (prefers-reduced-motion: no-preference)`. Keep gentler: override in `reduce` | https://developer.chrome.com/docs/modern-web-guidance/guides/ui-behaviors/motion | not shown |
| Reduced motion, anti-pattern | Do not use global `animation-duration: 0.01ms`; keep `color`/`background-color` transitions; dampen or disable spatial movement, scaling, rotation, persistent loops | same | not shown |
| Reduced motion, JS | If animation does not run, `animationend`/`transitionend` never fire; check `element.getAnimations()` is empty and continue | same | not shown |
| Final state | Ending state must be visible (not stuck at `opacity: 0`) when motion is off | same | not shown |
| Flash limit | No more than 3 flashes per second (WCAG 2.3.1) | same | not shown |
| Pause control | Auto-starting motion lasting over 5 s needs a visible, keyboard-operable pause (WCAG 2.2.2) | same; https://www.netguru.com/blog/top-web-design-trends | not shown ; updated 2026-10-16 |
| Focus rings | `clip-path` and `mask-image` clip outlines; draw the indicator on an unclipped parent | Chrome modern web guidance (motion) | not shown |
| Scroll performance | Prefer CSS scroll-driven animation to JS scroll listeners; JS scroll handlers can raise INP | https://www.netguru.com/blog/top-web-design-trends | updated 2026-10-16 |
| Frame budget | Target 60 fps; keep scroll loop under 4 ms per frame (publisher's own figure, no method) | https://digitalstrategyforce.com/journal/why-are-immersive-experiences-dominating-the-2026-awwwards/ | undated |
| Page weight context | Median mobile page 2,559 KB (HTTP Archive Web Almanac 2025, cited by Netguru); a 3D hero spends it fast | https://www.netguru.com/blog/top-web-design-trends | updated 2026-10-16 |
| Contrast with motion UIs | 4.5:1 body text; 3:1 large text and UI boundaries | https://www.netguru.com/blog/top-web-design-trends | updated 2026-10-16 |
| Transitions vs JS libraries | Smashing newsletter 565 raised CSS transitions vs JS animation libraries for performance (title only; not opened) | https://www.smashingmagazine.com/the-smashing-newsletter/smashing-newsletter-issue-565/ | 2026-06-30 (search snippet) |
| Event API | `event.animation` on `AnimationEvent`/`TransitionEvent` (Chrome 151) | https://web.dev/blog/web-platform-07-2026 | 2026-07-31 |

Duration ranges for page transitions or micro-interactions (for example 150 to 400 ms) were not found in any opened 2026 source other than the 0.2 s overlay example. Do not treat any broader range as sourced.

## WebGL and shaders

Fits (per sources):
- Luxury fashion, automotive, architecture/real estate, cultural institutions: 28/22/18/14% of immersive Site-of-the-Day winners per Digital Strategy Force. Same source puts technology/SaaS under 5%. It claims immersive 3D was 61% of H1 2026 winners (29 of 47 used Three.js, 8 custom WebGL, 4 Babylon.js, 6 none). Its own caveats: the cited link is caniuse, not award data; no sample method. Treat as unverified. https://digitalstrategyforce.com/journal/why-are-immersive-experiences-dominating-the-2026-awwwards/ (undated, H1 2026)
- DesignRush: 3D/AR is powerful but heavy and needs fallbacks. https://www.designrush.com/agency/website-design-development/trends/web-design-trends (updated 2026-10-20)
- Awwwards Sites of the Day list (2026-11-11 to 2026-11-20) shows studio/portfolio/brand sites, some on Webflow hosting; the page lists no technology per site, so it neither confirms nor refutes the WebGL share. https://www.awwwards.com/websites/sites_of_the_day/ (opened 2026-11-20)

Does not fit:
- Conversion pages, checkout, content-heavy or SaaS-docs sites; devices without GPU acceleration. Netguru: a 3D hero uses up a 2,559 KB median mobile budget quickly. https://www.netguru.com/blog/top-web-design-trends (updated 2026-10-16)

Guardrails (sourced):
- Load a poster image with explicit width/height first so it is the LCP element with zero CLS (Netguru).
- Functional HTML fallback for low-power or no-GPU devices (Digital Strategy Force).
- Keep real text in the DOM so it stays accessible and indexable; the WebGL layer draws a copy. The Codrops Atelier UI tutorial (2026-10-21) is cited for this only from a search snippet because the page returned 403; see Not verified.
- Gate motion behind `prefers-reduced-motion`; pause control for motion over 5 s (Netguru, Chrome guidance).
- A new native alternative to track: HTML-in-Canvas (origin trial) places real DOM in `<canvas>` via `layoutsubtree`. https://developer.chrome.com/blog/new-in-web-ui-io26 (2026-07-01)

## Fading trends

| Trend | Evidence | Source URL | Date |
|---|---|---|---|
| Original heavy glassmorphism | "Peaked around 2021", "heavy-handed"; only the restrained version persists | https://line25.com/articles/web-design-trends-2026/ | 2026-04 |
| Neumorphism | Largely gone from production after 2020 accessibility criticism; limited return | https://www.designrush.com/agency/website-design-development/trends/web-design-trends | updated 2026-10-20 |
| Experimental navigation for its own sake | Weight and accessibility cost without user value | same | updated 2026-10-20 |
| Stock hero photos, generic AI illustrations | Called overused and a credibility problem | same | updated 2026-10-20 |
| Purple-to-magenta "AI startup" gradients; beige minimalism | Called generic | same | updated 2026-10-20 |
| Retro/Y2K without brand link; neo-brutalism and dopamine design | Expected to look dated in 1 to 3 years (author's forecast) | same | updated 2026-10-20 |
| Figma-template/homogenized look; 12-column default | Called homogenized; layouts should start from content | https://line25.com/articles/web-design-trends-2026/ ; https://css-tricks.com/beyond-12-columns/ | 2026-04 ; 2026-11-12 |
| Autoplay hero video, auto-rotating carousels, parallax ignoring reduced motion, light-gray body text | Listed as outdated | https://www.netguru.com/blog/top-web-design-trends | updated 2026-10-16 |
| Oversized display type as a differentiator | Moves from "Peak" (2025) to "Normalizing" (2026) | https://line25.com/articles/web-design-trends-2026/ | 2026-04 |
| JS-for-everything UI plumbing | Natively replaced by popover, `closedby`, `scroll-target-group`, anchor positioning; but Smashing warns migrations add hidden complexity | https://developer.chrome.com/blog/new-in-web-ui-io26 ; https://www.smashingmagazine.com/2026/11/modern-css-might-make-your-front-end-more-complex/ | 2026-07-01 ; 2026-11-03 |

## Not verified

- Awwwards: no official trends report found; per-site technology and "collections" pages were not opened. Only the Sites of the Day list (page, opened 2026-11-20) was read. Site of the Month/Year not checked.
- Dribbble: no Dribbble-authored 2026 trends page found; Dribbble Select roundup not opened.
- Codrops: tympanus.net returned 403 to fetch. Titles and dates (2026-08-25 lens effect, 2026-10-21 Atelier UI, 2026-08-15 HAOQI.DESIGN, 2026-02-19 DOM-to-WebGL gallery, 2026-01 ASCII/dithering shaders) come from search snippets only.
- Josh W. Comeau: blog index opened but showed no dates and no 2026 CSS posts; titles (scroll-driven animations, squash and stretch, sprites) are from a daily.dev search snippet only. No Comeau entry is used as a source.
- Chrome release notes (developer.chrome.com/release-notes) page held no per-version content; Chrome 143 to 158 coverage comes from web.dev monthly posts and the IO26 post, which often omit versions. Chrome 158 items (CSS-Tricks Bytes 207/208, 2026-11-17) were titles only.
- Mozilla Firefox: MDN notes opened only for 159; others via web.dev.
- Safari 27.0 notes: first 100,000 of 117,251 characters read; scroll-driven animations and view transitions status in 27.0 not confirmed. WebKit blog items "spatial CSS", "Adaptive Dark Mode", Safari 27.3 had no dates in the listing.
- Smashing newsletter 549/565 and CSS-Tricks `scroll-state()` almanac: not opened.
- `if()` and `closedby` support outside Chrome; `text-fit` syntax; Liquid Glass web recreations (search results undated); view-transition cross-document Baseline status; popover/dialog Baseline dates (older than 12 months).
- Figma trends page has no date; not used as a dated source.
- Digital Strategy Force statistics and Line25 adoption percentages are unsourced publisher claims.
