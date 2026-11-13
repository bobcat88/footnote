# DESIGN
Last reviewed: 2026-11-12 · Approved by: pending human approval · Status: draft

> Footnote looks like a well-set technical document, not like a dashboard. Paper, ink, one proof-reader's red, and a raised numeral
> that means the same thing everywhere: *this claim has a source you can check.*

## 1. Tokens source
Hand-written for Footnote on 2026-11-12. Contrast values measured with the WCAG 2.x formula the same day. Nothing extracted from another site.

## 2. References
| # | Product or source | URL | Date viewed | Evidence type | Borrowed | Rejected and why |
|---|---|---|---|---|---|---|
| 1 | Stripe documentation | https://stripe.com/docs | 2026-11-12 | live pages | restraint, long reading measure, code beside prose | gradient mesh and glossy cards: ornament without information |
| 2 | Tufte CSS (sidenotes, tables as the hero) | https://edwardtufte.github.io/tufte-css/ | 2026-11-12 | published CSS | margin notes, rules instead of boxes | a body column too narrow for a product page |
| 3 | Linear marketing pages | https://linear.app | 2026-11-12 | live pages | dense factual hero, one accent | dark glass cards and glow: the AI-product look we avoid |
Blocked access: none recorded.

## 3. Color roles
| Role | Light | Dark | Contrast vs surface (light / dark) |
|---|---|---|---|
| surface (paper / ink ground) | #F4EFE6 | #14120F | - |
| surface-raised | #FBF8F2 | #1D1A16 | - |
| text-primary | #1B1915 | #EFE8DA | 15.3 / 15.3 |
| text-muted | #625A4E | #A89F90 | 5.9 / 7.2 |
| accent (proof red) | #A8281C | #F2705A | 6.1 / 6.5 |
| success | #2F6B3A | #7FC48B | 5.6 / 9.1 |
| warning | #8A5A00 | #E3B341 | 5.2 / 9.6 |
| danger | #B3261E | #FF8A7A | 5.7 / 8.2 |
| focus ring | #1B1915 | #EFE8DA | 15.3 / 15.3 |

Rule: the accent marks sources, actions, and Footnote results in benchmark charts (footnote numerals, links, the primary button, the focus underline, and Footnote bars and values). It never fills a section. No violet, no blue-to-purple gradient, anywhere. Primary button: paper text on accent, 6.6:1 (light); ink text on accent, 6.5:1 (dark).

## 4. Typography
- Families: Newsreader (variable, optical size; text and display) and IBM Plex Mono (code, numerals, labels). Both SIL OFL, **self-hosted**: no font request leaves the page, consistent with a local-first product.
- Scale:

| Name | Size | Line height | Weight | Use |
|---|---|---|---|---|
| caption | 0.8125rem | 1.45 | 450 | footnotes, table notes, labels (Plex Mono) |
| body | 1.125rem | 1.6 | 400 | prose, 62ch measure |
| title | clamp(1.5rem, 2.4vw, 2rem) | 1.2 | 560 | section headings |
| display | clamp(2.75rem, 7vw, 6rem) | 1.02 | 520 (optical size 72) | hero statement |

- Tabular numerals: yes, wherever numbers are compared (benchmarks, latencies, install steps).
- Footnote numerals are real superscripts (`<sup>`), accent colour, never images.

## 5. Space and shape
- Base unit: 8 px. Allowed steps: 4, 8, 16, 24, 32, 48, 64, 96, 128.
- Radii: 0 and 2 px only. Paper does not have rounded corners.
- Borders and elevation: 1 px hairline rules in muted text at 35% opacity; no shadows, no blurs, no glass. Depth comes from a change of ground (paper to raised paper) and from rules.
- Icon set: none decorative. Real product marks only for the harnesses Footnote installs into (their own colours); UI glyphs are typographic.

## 6. Layout and density
- Grid: 12 columns, max content width 1160 px; prose column 62ch with a marginal column (evidence, notes) at 280 px from 1100 px up.
- Minimum window size: 390 x 844 (phone) for the page; design reviewed at 1440 x 900.
- Resize behaviour: the marginal column folds under its paragraph as an indented note below 1100 px; tables scroll inside their own frame, never the page.
- Density: marketing page, one idea per screen, 60 to 80 words per screen outside tables.

## 7. Components and states
| Component | Default | Hover | Active | Focus | Disabled | Loading | Error |
|---|---|---|---|---|---|---|---|
| Primary button | accent fill, paper text | fill darkens 8%, underline draws | presses 1 px down | 2 px offset ring in focus colour | 50% opacity | n/a (links) | n/a |
| Text link | accent, underline offset 3 px | underline thickens | - | ring | - | - | - |
| Footnote marker | accent superscript | its marginal note highlights, no layout shift | - | ring | - | - | - |
| Code block | raised paper, Plex Mono, copy button | copy button visible | copy confirms with "Copied" | ring | - | - | copy blocked: "Select and copy" |
| Table row | hairline above | raised paper background | - | - | - | - | - |

Anything not in this table needs a decision-log entry first.

## 8. Screen states
| Screen type | Empty copy and action | Loading | Error copy and recovery | Partial | Heavy data |
|---|---|---|---|---|---|
| Landing page | n/a | static HTML, no spinner | n/a (no network use) | chart is an SVG with a text table as fallback | tables scroll inside their frame |
| Code sample | n/a | n/a | clipboard blocked: select-all hint | n/a | long lines scroll horizontally |

Long strings: code never wraps, it scrolls; prose wraps; hashes truncate in the middle with the full value in the title attribute.

## 9. Motion
- Duration range: 120-320 ms; the hero entrance up to 640 ms once.
- Easing tokens: `out` = cubic-bezier(0.2, 0.7, 0.2, 1); `in-out` = cubic-bezier(0.6, 0, 0.2, 1).
- Reduced motion: every animation is replaced by its end state; no content depends on motion to appear.
- May animate: the hero entrance, underline draws, footnote markers settling into place, benchmark bars growing once as they enter the viewport, link hover.
- Must never animate: body text on scroll, numbers counting up, parallax, anything that implies a result the tool did not measure.

## 10. Keyboard and desktop
- Shortcuts: none required; Tab order is reading order; skip link first.
- Focus order and Escape: no modals; install instructions are sections, not a widget.
- OS theme: follows the system.

## 11. Banned list
- Never a gradient mesh, glow, glass card or blurred blob.
- Never violet, lavender or blue-to-purple anywhere.
- Never a stock photo, an AI-generated image or a decorative illustration.
- Never a number without its dataset and split next to it.
- Never "supercharge", "unlock", "seamless", "next-generation", "revolutionize" or a robot emoji.
- Never a fake testimonial, logo wall or invented customer count.
- Never a third-party request (fonts, analytics, CDN) from the page.

## 12. Decision log
| Date | Decision | Reason | Rejected alternative |
|---|---|---|---|
| 2026-11-12 | The raised numeral (footnote marker) is the brand device | It is what the product does: every answer carries a checkable source | A generic brain, chip or spark icon |
| 2026-11-12 | Self-host Newsreader and IBM Plex Mono | A local-first, no-telemetry product must not call Google Fonts | Google Fonts, system serif only |
| 2026-11-12 | Paper and ink palette with one red | Reads as a document of record; high contrast; distinct from the dark-neon AI tool look | Dark mode with a neon accent |
| 2026-11-12 | Benchmarks as a table first, a chart second | Numbers must be readable and copyable; the chart is a summary | Chart-only hero |

| 2026-11-12 | Compare memory products by use, updates and deployment; separate measured search-engine results | Readers can choose between distinct memory models without mistaking component timings for product benchmarks | An unsupported winner or a feature checklist with unverified absences |

| 2026-11-12 | Use the proof-red accent for Footnote bars and values in benchmark charts | Make our results recognizable alongside published alternatives, as requested | Identical colors that make the product difficult to find |
