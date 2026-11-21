---
name: review-ui-quality
description: Use to review, QA, audit, critique or polish a user interface (web page, site, web app, Tauri or WebView2 app, Win32 window, component, theme, motion), when the user says /QLoop-UI or asks for a UI quality loop, or wants to know whether a screen is good enough to ship. Replaces "looks good to me" with hard gates, screenshot evidence, one or two independent critics and an anchored 9-category score. Also run it at the end of any substantial UI build, even unasked.
license: MIT
---

# Review UI quality (QLoop)

A screen passes only when, at the same time:
- every hard gate G1–G8 passes,
- the 9 category scores average ≥ 9.0 (reported as ≥ 90/100),
- every category is ≥ 8.0,
- the evidence matrix is captured.

At most 5 passes. The builder never scores its own work.

## Before pass 1
1. **Design contract.** The project has a `DESIGN.md` (colour roles with measured contrast, type scale, spacing unit, motion limits, a banned list). Missing? Write it first (see `choose-visual-direction`); gates G7 and G8 need it.
2. **Reference.** At least one golden screen or real product to match. None? The first screen that passes becomes it.
3. **Fixtures.** Every screen renders realistic data in `empty`, `normal`, `heavy`, `error` and `loading` states. For Tauri, add a mock entry that calls `mockIPC` so the dev server renders without the Rust side.
4. **Tooling.** Node with `playwright-core` (the scripts find it in the project or a global Playwright install) and `@axe-core/playwright`. Anything missing: say so and apply the honesty rules.
5. **Safe capture.** Synthetic data or verified redaction before any screenshot of a real app; record provenance per capture. Rules in [references/capture.md](references/capture.md).

## The loop
1. **Build** with `DESIGN.md` tokens only. Don't present it yet.
2. **Gates**, saved to `qloop-ui/pass-N/gates.json`:

| Gate | Check | Fail caps the score at |
|---|---|---|
| G1 | No console errors, unhandled rejections or failed requests, any screen, any state | 70 |
| G2 | No overflow, clipping, overlap or horizontal page scroll | 70 |
| G3 | `slop-scan` and `design-check` exit 0, and the `S`/`R` checks in [slop-patterns](references/slop-patterns.md) meet their tier | 75 (Floor) · 85 (unjustified Reflex) |
| G4 | axe: no serious or critical violations | 75 |
| G5 | Every control reachable and operable by keyboard, visible focus, logical order | 75 |
| G6 | No `lorem`, `undefined`, `null`, `NaN`, `[object Object]`, raw i18n keys, broken images, stand-in names | 70 |
| G7 | `prefers-reduced-motion` honoured; no animation over the `DESIGN.md` limit or blocking input | 80 |
| G8 | Computed colours, sizes, radii, spacing and easings match the tokens | 80 |

   A gate that can't run is `NOT RUN` with the reason, and caps the score at 85.
   ```
   node <skill>/scripts/slop-scan/slop-scan.mjs <url|file> --scope app|web [--steps steps.json] --out qloop-ui/pass-N/slop.json
   node <skill>/scripts/design-check/design-check.mjs <url|file> [--profile generic|apple|material] [--out qloop-ui/pass-N/design.json]
   ```
3. **Evidence matrix**: each screen × state × theme (dark first) at 1920×1080, 100% scale, unless the project's acceptance plan names other sizes. Add hover, focus, pressed, disabled, open overlays and frame sequences of transitions for interactive screens. Pass 1 and the final pass capture everything; passes in between recapture what changed plus one regression shot per other screen. Files: `qloop-ui/pass-N/screens/<screen>__<state>__<size>__<theme>.png`.
4. **Critique** by a separate agent with fresh context that never sees the builder's reasoning. Inputs, and nothing else: screenshots, `DESIGN.md`, [the rubric](references/rubric.md), `gates.json`, the reference, `qloop-ui/DEFECT-MEMORY.md`. Prompt and JSON output: [references/critic-prompt.md](references/critic-prompt.md).
   - **Dual critic** when the user asks for it, for a client delivery or release, or when pass 1 scores ≥ 90. Run two critics on different model families, same inputs, blind to each other. Merge: each category takes the **lower** score; defects are the union (same screen and element = one defect); a category where they differ by 2 or more is listed as a disagreement with both evidence lines, and the lower score stands.
5. **Score.** `raw = round(mean × 10)`, `final = min(raw, lowest gate cap)`.
6. **Anti-inflation.** A category that rises 2+ points in one pass needs before/after screenshots of the fix, or it reverts. A 10 needs a measured fact ("all 214 gaps are multiples of 4 px"); otherwise 9 is the ceiling.
7. **Fix**, in order: failed gates, categories under 8, highest-impact defects, polish ([references/polish.md](references/polish.md)). Fix the token or component, not twelve margins. Re-run the gates touched. Bugs found on the way: red test, then green.
8. **Stop.** Release when the pass rule holds. Escalate when the score gains < 3 points over two passes, or after pass 5: current scores, defects ranked by impact, evidence paths, and a recommendation (accept with known defects, redesign a component, change direction). The user decides.

## Final pass in the real environment
The dev server isn't the shipped app. Check the main screens in the real target: the deployed site, the Tauri or WebView2 app, or the native window. Fonts load, chrome and drag regions work, resize, menus and shortcuts behave, IPC data renders. Name the OS and webview tested; list the rest as not verified. Win32 and WebView2 specifics: [references/windows-native.md](references/windows-native.md).

## Honesty
- Never mark a gate passed that didn't run, or a screen verified without its screenshot.
- No browser automation available: run only the static checks, cap at 70, don't release.
- Never shrink the matrix to hide a failure.

## Result block
```
[QLoop-UI Result]
Verdict: RELEASED | ESCALATED
Passes: N/5 · Score per pass: 71 → 82 → 91
Final: 91/100 (lowest: <category> <score>)
Critics: single | dual (<model A>, <model B>; disagreements: <categories or none>)
Gates: G1 ✅ G2 ✅ G3 ✅ G4 ✅ G5 ✅ G6 ✅ G7 ✅ G8 ✅ (NOT RUN: none)
Slop: Floor 0 · Reflex <n> (<n> justified) · Finish <n> · ids: <SL-xx, ...>
Biggest flaw caught: <one line with evidence>
Remaining defects: <list or none>
Verified on: <OS / browser / webview> · Not verified: <list>
Evidence: qloop-ui/pass-N/
```
Then add each defect seen twice, or still open in a late pass, to `qloop-ui/DEFECT-MEMORY.md`: `- [date] <pattern> · cause: <root cause> · rule: <permanent rule> · seen: <n>`. Seen 3 times: it becomes a `DESIGN.md` rule.

## Calibration
Once per project, and when the rubric or the critic model changes: the critic scores 3 screens you rate good and 3 you rate poor. Calibrated when good ≥ 8.5 and poor ≤ 6.5; otherwise sharpen the band text. Keep the shots in `qloop-ui/calibration/`.
