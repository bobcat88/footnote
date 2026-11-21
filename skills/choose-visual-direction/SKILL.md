---
name: choose-visual-direction
description: Use when a new app, screen, product page or site has no design yet (no DESIGN.md, brand system or finished mockup), when the user wants to pick a look before building, or wants an existing site's design captured as tokens and references. Captures references from leading products, builds four genuinely different mock directions, lets the user pick and tune one in an interactive studio, then writes DESIGN.md. Use it before inventing a look from scratch.
license: MIT
---

# Choose a visual direction

References first, then four genuinely different directions; the user steers and you build. Screenshots of good products lift first-shot quality far more than written rules or token dumps do, and the direction users pick is the one that shows the product working.

**Skip it** when a `DESIGN.md` exists or the user hands over a finished design: build in that system, then run `review-ui-quality`.

**New screen inside an app that already has a design:** do steps 2 and 3 for ideas and structure only. Show the four screenshots; the user names what they like in each; build that blend as one more mock **in the app's own `DESIGN.md`** (its tokens, type, components), never a new style. The user approves in chat. Skip the studio and the lock; record the screen's decisions in the existing `DESIGN.md`.

Work files go in the project under `.design-studio/`. Scripts: `<skill>/scripts/studio/` here, browser tools in `review-ui-quality/scripts/`.

## 1. Brief
One round of at most three questions: the product and its users, the one screen that matters most (the one that sells or does the main job), the platform. Infer the rest and say what you inferred.

Then name the **kind**, because it decides the treatment:

| Kind | Leads with | Effects |
|---|---|---|
| Software, AI, developer tool, fintech | the product's own UI, live, doing its real job | one effect or scene at most, where the subject is technical |
| Game | its world: key art, name set large | allowed, never replacing the world |
| Hardware | the device in photographs, whole and big | a render only when asked for 3D |
| Physical product or brand | photographs of the product in use | none; flat grounds, type, photos |
| Service (clinic, agency, firm) | people, rooms, the work | none |
| Place (café, shop, hotel) | photographs of the place, the visit details early | none |

## 2. References
- The user gave sites or screens: capture them. Nothing given: pick 3–5 leaders in the category whose public pages show the real product, and say why in one line each.
- Capture (1920×1080, screenshots, computed tokens, a motion fingerprint):
  `node <skills>/review-ui-quality/scripts/ref-capture/ref-capture.mjs <urls> --out .design-studio/refs --shots 6 --wait 8000`
  (`--hide "<selector>"` for a loader that never clears). Look at the frames yourself.
- This is also how you capture an existing site's design as tokens: each `<site>/tokens.json` holds its colours, type, radii and motion.
- In `.design-studio/references.md`, per reference: what to take (one line) and what it shows working.

## 3. Four directions
Each is a self-contained `.design-studio/directions/D<n>/index.html` of the key screen with realistic content (the product at work, believable data, no lorem), plus `tokens.json` in the studio schema ([references/studio.md](references/studio.md)).
- D1, D2: each anchored on a different reference's feel.
- D3: the user's known taste, if you know it; otherwise a blend of the two strongest references.
- D4: the category's opposite, what nobody in the sector does, still credible.
- **Divergence is the rule.** Four near-identical options are one option. Before building, write a matrix (rows: idea, theme, accent, palette strategy, type voice, shape, density, motion, how the key content is shown; one column per direction), make every cell in a row different, and show it first. A locked brand doesn't excuse sameness: at least one direction keeps it, the others may break it on purpose (say which). If two finished mocks read as variants at a glance, rebuild one from its opposite before showing any.
- Every mock shows the product working on its first screen, and its motion means something for the product.
- Build them with four fresh agents in parallel when the user allows subagents (fresh context keeps them apart), each given only the brief, its references and these rules.
- Check each: `node <skill>/scripts/studio/shot.mjs <index.html> <out-prefix>` and `slop-scan <index.html> --scope web|app`. Fix every Floor failure before showing anything.

## 4. Studio
Write `.design-studio/studio.json` (schema in [references/studio.md](references/studio.md)), then
`python -I <skill>/scripts/studio/build.py .design-studio/studio.json .design-studio/studio.html`.
The user picks a base direction, tunes theme, accent, warmth, tint, corners, density, type contrast, fonts, display weight and motion, sees live contrast checks, writes notes, and presses **Lock this direction**. Reading the lock depends on the host: see [references/studio.md](references/studio.md). Continue only once it's locked; if the user says to go on without locking, use the latest choice and say so.

## 5. DESIGN.md
Write `PRODUCT.md` and `DESIGN.md` from [references/design-md-template.md](references/design-md-template.md) with the locked choice: colour roles with the measured contrast, type scale, spacing from density, radius, fonts, motion level and durations (subtle 160 ms, lively 260 ms, ease-out, `transform` and `opacity` only), the user's notes. Decision log: the direction chosen, what was tuned, the directions rejected and why. The lock is the human approval the template asks for.

## 6. Build, then review
Build with `DESIGN.md`, the chosen mock as the composition reference and the reference screenshots as a feel guide, plus a short brief of what each screen shows working and what its motion expresses. Don't pass a dump of measured reference tokens. Then run `review-ui-quality` with the chosen mock and the references as the critic's reference.

## Rules
- At most four directions; never show one that fails a Floor slop check.
- References are for feel: never copy a brand's name, logo, copy, imagery, code or proprietary fonts.
- Invented content is realistic and clearly fictional: no invented reviews, customers or figures presented as real.
- Only the user locks a direction.
- Report what you inferred, chose and skipped, one line each.
