# Material 3 values

Run the generic profile first, then `--profile material` (values in `review-ui-quality/scripts/design-check/profiles/material.json`). It uses 48 px touch targets and adds WARN-level token checks:

| Id | Rule |
|---|---|
| M1 | Control and card radii on the shape scale 0, 4, 8, 12, 16, 20, 28, 32, 48 px, or full pill |
| M2 | Font sizes on the type scale, 11–57 px |
| M3 | Durations in the token set (50–1000 ms); easings from the M3 curves (emphasized `cubic-bezier(0.2, 0, 0, 1)`) |
| M4 | Disabled content at 0.38 opacity |

- State layers: hover 0.08, focus and pressed 0.10 (Compose) or 0.12 (web components), dragged 0.16.
- The Material web components have been in maintenance mode since June 2024 and don't include the Expressive update (shape morphing, spring motion), which is Compose-first. On the web, carry the tokens into native CSS.
- Not scripted: colour roles and tonal palettes, layout margins and gutters, canonical layouts, spring motion.
- One design language per product: never mix Material and Apple conventions on one screen.

Sources: [M3 shape](https://m3.material.io/styles/shape/corner-radius-scale) · [M3 type scale](https://m3.material.io/styles/typography/type-scale-tokens) · [M3 motion tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs) · [M3 states](https://m3.material.io/foundations/interaction/states/state-layers) · [material-web status](https://github.com/material-components/material-web#readme)
