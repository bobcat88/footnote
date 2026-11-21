# UI slop patterns

Tells that make an interface look machine-made, each with a check. This is the only list; project bans stay in the project's `DESIGN.md`.

## How a pattern binds
| Tier | Meaning | Effect |
|---|---|---|
| **Floor** | Correctness, honesty or access. No waiver. | Fails G3, caps the score at 75 |
| **Reflex** | A default models fall into. Fix it or justify it. | Unjustified: caps at 85. Justified: listed in the result |
| **Finish** | Craft detail. | Never blocks; listed in the result |

**Justifying a Reflex pattern.** The builder writes it into `gates.json` under `slop.waivers`: the id, the evidence the exemption holds, and a reason that names this screen or element (at least 24 characters, never reused word for word). The critic may reject boilerplate. A pattern justified on three builds in a row is house style: move it into `DESIGN.md`.

**Check class.** `S` static (source, computed styles) · `R` measured on the render · `J` judged by the critic from screenshots. The builder runs `S` and `R`; the critic judges `J`. Measure at the review size (1920×1080 by default) and calibrate thresholds on the project's calibration screens before they gate a release.

**Scope.** `all` = apps and sites. `web` = pages and sites only.

## All UI
| Id | The tell | Class | Tier |
|---|---|---|---|
| SL-01 | **Fake success.** "Saved", "Sent", "Done" with no request or IPC call behind it, including a mock shipped as real. Perform the action and assert the call happened. | S+R | Floor |
| SL-02 | `transition: all`, alone or in a list. | S | Floor |
| SL-03 | Broken input states: border width changes between states; focus ring drawn with `border`; input height differs from the button beside it; "disabled" by class without `disabled` or `aria-disabled`. | S | Floor |
| SL-04 | Auto-advancing content (carousel, ticker) that doesn't pause on hover and focus ([WCAG 2.2.2](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html)). | S | Floor |
| SL-05 | Stand-in names in rendered text (Jane Doe, John Doe, Acme, Nexus). Fixtures use realistic names. | S | Floor |
| SL-06 | Purple-to-blue, purple-to-pink or cyan-on-dark gradients. | S | Reflex |
| SL-07 | A framework's default palette used as the brand (indigo-500, violet-600, slate-900…) when `DESIGN.md` didn't choose it. | S | Reflex |
| SL-08 | Accent fills over 5% of the viewport (warn) or 8% (fail), unless `DESIGN.md` declares it. Measure screenshot pixels. | R | Reflex |
| SL-09 | More than 3 font families or 3 weights in visible text. | R | Reflex |
| SL-10 | One word set apart (italic, colour, weight, highlight) inside an otherwise plain heading. | S+J | Reflex |
| SL-11 | Straining text: running text under 12 px, labels under 11 px, multi-line text leaded under 1.3, unless `DESIGN.md` sets it on purpose. | R | Reflex |
| SL-12 | A heading closer to the content above it than to the content it heads. | R | Reflex |
| SL-13 | Text in a filled or bordered box padded less than max(8 px, 0.5 em) at the sides or max(4 px, 0.3 em) top and bottom (boxes over 100×30 px). | R | Reflex |
| SL-14 | A control with no `:active` or `:disabled` style; a focus ring that fades in instead of appearing at once. | S | Reflex |
| SL-15 | Two icon sets on one screen. | S | Reflex |
| SL-16 | Two `position: sticky; top: 0` elements stacking on each other. | S+R | Reflex |
| SL-17 | A celebration toast for an effect the user can already see. | J | Reflex |
| SL-18 | Animating `width`, `height`, `top`, `left`, `margin` or `padding` instead of `transform` and `opacity`. A deliberate layout morph (sidebar collapse, accordion) can be justified. | S | Reflex |
| SL-19 | **Specificity test**: could this screen belong to any other product, and what does only this product show? | J | Reflex |
| SL-20 | More than one hover effect on an element, or hover on something not interactive. | S+J | Finish |
| SL-21 | Two filled primary buttons side by side, or two primary-button styles in one view. | S | Finish |
| SL-22 | A decorative `svg` or `canvas` with neither `aria-hidden` nor an accessible name. | S | Finish |
| SL-23 | A small tracked-caps label above every heading (the "eyebrow" reflex). List headers in native apps are not this. | S+J | Reflex |
| SL-24 | A grid of identical cards (icon, title, two lines) as the default layout, or cards nested in cards. | J | Reflex |
| SL-25 | Gradient-filled text (`background-clip: text`). | S | Reflex |
| SL-26 | Blur and translucency on content surfaces rather than floating chrome; glass that lowers text contrast. | S+J | Reflex |
| SL-27 | A coloured stripe on one side of cards or callouts as decoration. | S | Reflex |
| SL-28 | A hard offset shadow (zero blur) or a coloured glow around boxes. | S | Reflex |
| SL-29 | Emoji standing in for icons. | S | Reflex |
| SL-44 | Monospace type on text that isn't code, data or input. | R | Reflex |
| SL-45 | Bounce or elastic easing on interface motion (a curve that overshoots). | S | Reflex |
| SL-46 | Prose lines over 75 characters, or under 45, at the review size. | R | Finish |

## Pages and sites only (scope `web`)
| Id | The tell | Class | Tier |
|---|---|---|---|
| SL-30 | Invented proof: metrics, quotes, logo walls or case counts nobody supplied. | S+J | Floor |
| SL-31 | Content hidden at rest: a reveal starts at `opacity: 0` and needs script to show it. Hide only under a `.js` class, or animate from visible. | S | Floor |
| SL-32 | At 1920×1080: container without `max-width`, gutters under 120 px, a grid that gains columns, or a hero that no longer fits. | R | Floor |
| SL-33 | A drawn browser bar (traffic lights, URL pill) or OS title bar. Use a real screenshot or no chrome. | S | Floor |
| SL-34 | The template page: hero, three features, CTA, footer; a reflex email sign-up closing a page whose job isn't a mailing list. | J | Reflex |
| SL-35 | The stock "editorial" look on an unthemed build: cream paper, high-contrast serif display, terracotta accent. | J | Reflex |
| SL-36 | A page of small things: sections with no mass, section heads under 40 px, or 160 px+ of dead space between the hero and the fold. | R | Reflex |
| SL-37 | A drawing where a photo or the product belonged; empty picture blocks; 24+ identical marks standing for a number. Real charts, maps, diagrams and the user's logo are exempt. | R+J | Reflex |
| SL-38 | A built mock of a product that exists and could be captured. | J | Reflex |
| SL-39 | A device frame with nothing real inside, or a "product UI" of styled boxes standing in for a capture. | S+J | Reflex |
| SL-40 | Decoration text: rotated labels, scroll cues, a version pill in the hero, `01 / 04` section counters. | J | Reflex |
| SL-41 | One fact (price, date, count) repeated in four or more sections. | S+J | Reflex |
| SL-42 | More than about 80 words in any 900 px window; a hero with more than a headline, one sentence and its actions. | R | Reflex |
| SL-43 | Every section separated the same way, or gaps under 120 px between sections. | R+J | Finish |

## Not on the list, on purpose
- Valid on native platforms: pure black and white, neutral greys, tracked-caps list headers, middle-dot separators, centred product heroes, photo-led pages, 28–40 px desktop control heights.
- Already gated elsewhere: overflow, overlap, contrast, focus, placeholders, reduced motion and tokens (G1, G2, G4–G8).
- Noisy: an orphan item on a grid's last row, one-word last lines in headings.

## What the scripts check
`scripts/slop-scan` (Playwright, 1920×1080): SL-01 to SL-05, plus SL-31 to SL-33 with `--scope web`. Exit 0 clean, 1 on a fail, 2 on a tooling error.
- **SL-01** needs `--steps` (the actions that should save or send); without them it's `NOT RUN`. It fails when success text appears and no network call (fetch, XHR, ping, form post) or `window.__slopIpcCalls` moved. Tauri mocks: have the `mockIPC` handler increment `window.__slopIpcCalls`.
- **SL-03** checks input height against the adjacent button, box or border change on focus, and disabled-by-class. Not checked: a focus ring made only of a border-colour change.
- **SL-04** hovers each infinite text animation and each carousel or ticker; content must stop changing. Not checked: pause on keyboard focus.
- **SL-31** compares text opacity with scripts on and off. App shells hide nav text without scripts too, so it's `web` scope only.
- **SL-32** measures text gutters per top-level block, ignoring screen-reader text and real horizontal scrollers. Not checked: grids gaining columns, hero fit.
- **SL-33** looks for red, yellow and green dots in one parent.
- **SL-30** stays with the critic: whether a number was invented can't be seen from the page.

`scripts/design-check` measures the craft rules: hit targets, motion under reduced motion, visible focus, contrast, card and list alignment, icon sizing, default-looking controls, hover and pressed states, state transitions. Profiles: `generic`, `apple`, `material`. Exit 1 on any fail.

Everything else marked `R`: read computed styles and boxes with `page.evaluate` at the review size and record the numbers in `gates.json`. A new automated check lands only with a failing fixture, a clean fixture and a test.
