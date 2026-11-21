---
name: review-ui-quality
description: Use to review, QA, audit, critique or polish a user interface (web page, site, web app, Tauri or WebView2 app, Win32 window, component, theme, motion), when the user says /QLoop-UI or asks for a UI quality loop, or wants to know whether a screen is good enough to ship. Replaces "looks good to me" with hard gates, screenshot evidence, one or two independent critics and an anchored 9-category score. Also run it at the end of any substantial UI build, even unasked.
license: MIT
---

# Review UI quality (QLoop)

Pass = gates G1–G8 all pass / mean of 9 categories ≥ 9.0 (≥ 90/100) / every category ≥ 8 / evidence matrix captured. Max 5 passes. The builder never scores its own work.

## Before pass 1
- `DESIGN.md` exists (colour roles + measured contrast / type scale / spacing unit / motion limits / banned list); missing → `choose-visual-direction`
- A reference screen or product to match; none → the first passing screen becomes it
- Fixtures per screen: empty / normal / heavy / error / loading (Tauri: `mockIPC` entry so the dev server renders)
- Node + `playwright-core` + `@axe-core/playwright`; missing → say so, honesty rules apply
- Synthetic data or verified redaction before any capture of a real app → [references/capture.md](references/capture.md)

## Run
```
node <skills>/review-ui-quality/scripts/slop-scan/slop-scan.mjs <url|file> --scope app|web [--steps steps.json] --out qloop-ui/pass-N/slop.json
node <skills>/review-ui-quality/scripts/design-check/design-check.mjs <url|file> [--profile generic|apple|material] --out qloop-ui/pass-N/design.json
node <skills>/review-ui-quality/scripts/ref-capture/ref-capture.mjs <urls> --out refs   # reference screenshots + tokens
```

| Gate | Check | Fail caps at |
|---|---|---|
| G1 | no console errors / unhandled rejections / failed requests | 70 |
| G2 | no overflow / clipping / overlap / horizontal scroll | 70 |
| G3 | slop-scan + design-check exit 0 / [slop tiers](references/slop-patterns.md) met | 75 Floor · 85 Reflex |
| G4 | axe: no serious or critical | 75 |
| G5 | keyboard: every control reachable + operable / visible focus / logical order | 75 |
| G6 | no lorem / undefined / null / NaN / `[object Object]` / raw i18n keys / broken images | 70 |
| G7 | reduced motion honoured / no animation over the DESIGN.md limit or blocking input | 80 |
| G8 | computed colours, sizes, radii, spacing, easings = tokens | 80 |

Gate that can't run = `NOT RUN` + reason, caps at 85.

## Loop
1. Build with DESIGN.md tokens only
2. Gates → `qloop-ui/pass-N/gates.json`
3. Evidence: screen × state × theme (dark first) at 1920×1080, 100% scale + hover / focus / pressed / disabled / overlays / transition frames → `qloop-ui/pass-N/screens/<screen>__<state>__<size>__<theme>.png`. Pass 1 + final = everything; between = what changed + one regression shot per screen
4. Critic: separate agent, fresh context, only screenshots + DESIGN.md + [rubric](references/rubric.md) + gates.json + reference + `qloop-ui/DEFECT-MEMORY.md` → [prompt + JSON](references/critic-prompt.md)
   - Dual critic when asked / client delivery or release / pass 1 ≥ 90: two model families, blind to each other. Merge: lower score per category / union of defects / gap ≥ 2 = disagreement, lower stands
5. Score: `final = min(round(mean × 10), lowest gate cap)`
6. Anti-inflation: +2 in one pass needs before/after shots or reverts / a 10 needs a measured fact, else 9 max
7. Fix: failed gates → categories < 8 → biggest defects → [polish](references/polish.md). Fix the token or component / re-run touched gates / bug → red test then green
8. Stop: release when the pass rule holds / escalate when < 3 points gained over two passes or after pass 5 → scores, ranked defects, evidence, recommendation; the user decides

## Final pass in the real target
Deployed site / Tauri or WebView2 app / native window: fonts / chrome + drag regions / resize / menus + shortcuts / IPC data. Name OS + webview tested, list the rest as not verified → [windows-native](references/windows-native.md).

## Honesty
- Never pass a gate that didn't run, or a screen without its screenshot / never shrink the matrix to hide a failure
- No browser automation → static checks only, cap 70, no release

Result block, defect memory, calibration → [references/report.md](references/report.md).
