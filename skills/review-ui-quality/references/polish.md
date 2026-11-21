# Polish: the last pass

Run after the gates are green and no category is under 8. Each line is a check; fix at the token or component, not per screen.

## Spacing and shape
- Every gap is on the spacing scale (4 or 8 px base unless `DESIGN.md` says otherwise).
- Nested corners are concentric: inner radius = outer radius − padding.
- Asymmetric glyphs (play triangles, chevrons, badges beside text) are aligned by eye, not by bounding box.
- Depth comes from one shadow scale and surface steps, not a different shadow per component.

## Type
- Prose runs 45–75 characters per line; multi-line headings use `text-wrap: balance`, short lead paragraphs `text-wrap: pretty`.
- Tabular figures (`font-variant-numeric: tabular-nums`) on tables, prices, counters, clocks and anything that updates.
- App chrome tops out at weight 600; hierarchy comes from size and colour steps first.
- Monospace only for code, commands, hashes and machine data.

## Colour
- Define the palette in OKLCH so equal steps look equal; derive hover and pressed states from the token, not a new hex.
- No hard-coded colours in components: dark mode breaks first where they hide.
- One filled primary action per view; the others are secondary or quiet.

## Motion
- Every animation answers "what changed?"; decorative motion goes.
- Animate `transform` and `opacity`; layout properties only for a deliberate layout morph, profiled.
- Durations, as a starting scale: hover and press about 100 ms, tooltip and menu about 120 ms, panel about 150 ms, dialog about 200 ms, page transitions 300 ms at most.
- Easing: decelerate on enter, accelerate on exit, a standard curve for on-screen changes (Material's `cubic-bezier(0.4, 0, 0.2, 1)` family is a sound default). No overshoot on interface motion.
- Any input interrupts an animation; nothing makes the user wait for one to finish.
- Under `prefers-reduced-motion: reduce`, replace movement with a fade or an instant change.

## States and copy
- Each state tells the user what happened and what to do next: empty states offer the first action, errors say what failed and how to recover.
- Buttons name the action ("Save changes", "Delete project"), never "OK" or "Submit".
- Dialog titles are statements ("Delete project"), not "Are you sure?"; irreversible actions say "This can't be undone."
- A disabled control explains why, in a popover or inline text; a tooltip on a disabled element is unreachable for many users.

## Browser and platform surfaces
- Set `color-scheme`, a `theme-color`, styled scrollbars and `::selection` so browser-drawn parts match the theme.
- Form controls get the brand through `accent-color` before custom widgets.
- Favicon, page title and social preview exist and match the product (sites).
