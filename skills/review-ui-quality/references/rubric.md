# Rubric: 9 categories, anchored bands (0–10)

Pick the closest band and quote the evidence for it. Between two bands = ±1. A 10 needs a measured fact.

**C1. Layout and spacing**
- 10: every gap and padding is a multiple of the base unit (measured); shared alignment axes; equivalent components have identical margins.
- 8: ≤ 2 off-grid values; no visible misalignment at normal viewing.
- 6: several off-grid values, or uneven padding between similar components.
- ≤ 4: clear misalignment, cramped or floating elements, inconsistent gutters.

**C2. Typography**
- 10: one type scale from tokens, ≤ 2 families, clear weight hierarchy, prose 45–75 characters per line, tabular figures in data.
- 8: tokens respected, one or two small slips.
- 6: sizes off the scale, weak hierarchy, awkward wrapping.
- ≤ 4: random sizes and weights, hard to read, widows in key labels.

**C3. Colour and contrast**
- 10: token colours only; accent used sparingly; text ≥ 4.5:1 (≥ 3:1 large) and UI parts ≥ 3:1, measured (WCAG 2.2 SC 1.4.3, 1.4.11); semantic colours consistent; dark and light both balanced.
- 8: small tint drift; no contrast failure.
- 6: one or two contrast failures or off-token colours; one theme visibly weaker.
- ≤ 4: muddy palette, many failures, decorative colour with no meaning.

**C4. Hierarchy and density**
- 10: the primary action and key information are found within 3 seconds; density fits the job (dense for expert tools, open for marketing); grouping is clear without heavy borders.
- 8: clear hierarchy, minor competition.
- 6: several elements compete; density wasteful or overloaded.
- ≤ 4: flat or confusing; the user can't tell what matters.

**C5. States and edge cases**
- 10: empty, loading, error, partial, heavy and disabled states all designed, each with useful copy and a recovery action; no layout jump between states; long strings handled (ellipsis with tooltip, or wrap).
- 8: every state exists; one or two are plain.
- 6: some states missing or generic ("No data").
- ≤ 4: blank screens, raw errors, layout breaks on heavy data.

**C6. Interaction and motion**
- 10: hover, pressed, focus and disabled feedback on every control; transitions short (about 120–250 ms) on token easings; motion has a purpose; reduced motion honoured (SC 2.3.3); no jank.
- 8: purposeful and consistent, one or two rough transitions.
- 6: mixed timings or easings, or decorative motion.
- ≤ 4: missing feedback, janky, distracting or blocking animation.

**C7. Keyboard, focus and accessibility**
- 10: full keyboard use; logical order; visible focus never hidden behind sticky content (SC 2.4.7, 2.4.11); Escape closes overlays and returns focus; shortcuts documented and conflict-free; names and roles exposed; targets ≥ 24 px or spaced (SC 2.5.8); axe clean.
- 8: small gaps (one missing shortcut hint).
- 6: some controls unreachable, or focus lost after actions.
- ≤ 4: mouse-only, no visible focus, serious axe violations.

**C8. Platform fit**
- 10: behaves like its platform. Desktop app: correct chrome and drag regions, graceful resize, follows the OS theme and scale, native-feeling menus, selection and scrolling, no web tells (selectable button text, rubber-band scroll, default blue links, browser focus outlines). Website: real links, working back button, reflows without loss (SC 1.4.10), fast first paint.
- 8: one or two tells.
- 6: feels like a web page in a window, or a site fighting the browser.
- ≤ 4: broken resize or navigation, unusable at the tested size.

**C9. Consistency and reference parity**
- 10: every screen could be mistaken for the same product; components behave identically; quality matches the reference; no one-off styles.
- 8: one or two small deviations.
- 6: visible variation between screens.
- ≤ 4: screens look like different products.

## Sources
- WCAG 2.2: [1.4.3 Contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html), [1.4.11 Non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html), [1.4.10 Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html), [2.3.3 Animation from interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html), [2.4.7 Focus visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html), [2.4.11 Focus not obscured](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html), [2.5.8 Target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html).
- Line length: [WCAG 1.4.8 Visual presentation](https://www.w3.org/WAI/WCAG22/Understanding/visual-presentation.html) (80 characters at most); 45–75 is the usual typographic range.
