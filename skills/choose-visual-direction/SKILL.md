---
name: choose-visual-direction
description: Use when a new app, screen, product page or site has no design yet (no DESIGN.md, brand system or finished mockup), when the user wants to pick a look before building, or wants an existing site's design captured as tokens and references. Captures references from leading products, builds four genuinely different mock directions, lets the user pick and tune one in an interactive studio, then writes DESIGN.md. Use it before inventing a look from scratch.
license: MIT
---

# Choose a visual direction

References → four different directions → the user picks and tunes → DESIGN.md → build → review. Screenshots of good products lift first-shot quality more than written rules or token dumps.

- **Skip** when DESIGN.md exists or a finished design is handed over → build in it, then `review-ui-quality`
- **New screen in an app with a design:** steps 2–3 for ideas only → user names what they like in each → build the blend **in the app's own DESIGN.md**, never a new style → approval in chat / no studio, no lock
- Work files: `.design-studio/` / scripts: `<skills>/choose-visual-direction/scripts/studio/` + `<skills>/review-ui-quality/scripts/`

## 1. Brief
Max 3 questions: product + users / the one screen that matters most / platform. Infer the rest, say what you inferred. Name the kind:

| Kind | Leads with | Effects |
|---|---|---|
| Software, AI, dev tool, fintech | the product's own UI doing its real job | ≤ 1, where the subject is technical |
| Game | its world: key art, name set large | allowed, never replacing the world |
| Hardware | the device photographed, whole and big | render only if 3D asked |
| Physical product, brand | product in use, photographed | none |
| Service | people / rooms / the work | none |
| Place | photos of the place / visit details early | none |

## 2. References
- User's sites, or 3–5 category leaders whose public pages show the real product (one-line reason each)
- `node <skills>/review-ui-quality/scripts/ref-capture/ref-capture.mjs <urls> --out .design-studio/refs --shots 6 --wait 8000 [--hide "<selector>"]` → screenshots + `tokens.json` (colours / type / radii / motion). Look at the frames yourself
- Same command = capture an existing site's design as tokens
- `.design-studio/references.md`: per reference, what to take + what it shows working

## 3. Four directions
`.design-studio/directions/D<n>/index.html` (key screen, realistic content, no lorem) + `tokens.json` ([schema](references/studio.md)).
- D1, D2 = two references' feel / D3 = user's known taste or blend of the two strongest / D4 = the category's credible opposite
- Divergence matrix first (rows: idea / theme / accent / palette / type voice / shape / density / motion / how key content shows; one column per direction), every cell in a row different. Locked brand: ≥ 1 direction keeps it, others may break it (say which). Two mocks read as variants → rebuild one
- Product working on the first screen / motion means something
- Four fresh parallel agents when subagents are allowed, each with only brief + its references + these rules
- Check: `node <skills>/choose-visual-direction/scripts/studio/shot.mjs <index.html> <out-prefix>` + `slop-scan --scope web|app`; fix every Floor failure before showing

## 4. Studio
`.design-studio/studio.json` ([schema](references/studio.md)) → `python -I <skills>/choose-visual-direction/scripts/studio/build.py .design-studio/studio.json .design-studio/studio.html`. User picks / tunes (theme, accent, warmth, tint, corners, density, type, fonts, weight, motion) / sees live contrast / writes notes / **Lock**. Reading the lock: [studio.md](references/studio.md). No lock and told to go on → latest choice, say so.

## 5. DESIGN.md
`PRODUCT.md` + `DESIGN.md` from [the template](references/design-md-template.md): colour roles + measured contrast / type scale / spacing from density / radius / fonts / motion (subtle 160 ms, lively 260 ms, ease-out, transform + opacity) / notes / decision log (chosen, tuned, rejected + why). The lock = human approval.

## 6. Build → review
DESIGN.md + chosen mock (composition) + reference shots (feel) + what each screen shows working. No token dumps. Then `review-ui-quality` with the mock + references as the critic's reference.

## Rules
- ≤ 4 directions / never show one failing a Floor check / only the user locks
- References for feel only: never a brand's name / logo / copy / imagery / code / proprietary fonts
- Invented content realistic and clearly fictional: no fake reviews, customers or figures
- Report inferred / chosen / skipped, one line each
