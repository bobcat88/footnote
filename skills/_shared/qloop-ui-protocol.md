# UI Quality Loop (/QLoop-UI) Protocol

## Trigger
Invoke whenever the artifact is a **user interface** (desktop, web, or Tauri/Rust app screens, components, layouts, themes, motion) and the user says `/QLoop-UI`, asks for a UI quality loop, asks to "check/QA/polish the UI", or wants a UI result before it is shown to them. Also invoke it at the end of any substantial UI build task (new screen, redesign, theme change).

Do NOT use it for pure backend logic or prose-only artifacts. Use QLoop Protocol for those.

## Relationship to other protocols
- QLoop Protocol: the generic iteration engine. This protocol specialises it for UI.
- ui-ux-qloop: the French UI/UX skill (reference research, task matrix, Playwright evidence, dark-theme-first testing). Use it for **what to design and how to research references**. Use this protocol for **how to score and gate the result**. Where both apply, follow both; on a conflict, the stricter rule wins.
- **Benchmark composition:** this protocol is the UI benchmark. `impeccable` feeds G3 and the critique, ui-slop-patterns (the shared AI-slop catalog) extends G3 and the critique, `unslop` covers all user-visible prose, and the project's own anti-slop rules are its `DESIGN.md` banned list (section 11). Apple HIG / Liquid Glass material is a design guideline cited in `DESIGN.md`, not a scored gate (see the stack contract, "Benchmark composition").

## Core Principle
A UI is not "good" because the builder says so. A UI passes only when:
1. every hard gate is green (deterministic, no opinion),
2. an **independent** critic, looking at **screenshots**, cannot score it below the threshold using **anchored bands and cited evidence**,
3. the result holds across **states, window sizes and themes**, not one lucky screenshot.

Think of it as a PMO stage gate: entry criteria (gates), an independent reviewer (critic), and a RAG status that is only green if **no** category is red.

## Pass condition (release threshold)
All of the following, at the same time:
- All hard gates G1-G8 pass.
- Mean of the 9 category scores ≥ **9.0** (reported as **≥ 90/100**).
- **Every** category ≥ **8.0** (no weak link hidden by averages).
- Evidence captured for the full matrix (see step 3).

Maximum **5 passes**. Stop early on plateau (step 8).

---

## Prerequisites (check before pass 1)
1. **Design context complete (UI Design Contract).** `PRODUCT.md` and `DESIGN.md` must exist and pass the completeness check in AI Coding Rules and the template in design-md-template: users and top tasks, color roles with measured contrast, named type scale, spacing unit, minimum window size, banned list (5+), decision log, and human approval. If anything is missing, complete the files first. Do not start styling, and do not run the gates against an incomplete DESIGN.md: the gates would have nothing to check.
2. **Reference exists.** At least one "golden" screen or a reference app/site the output must match in quality. If none, designate the first screen that passes as the reference for the rest of the project.
3. **Deterministic fixtures.** Every screen must render with realistic data in these states: `empty`, `normal`, `heavy` (long names, many rows, large numbers), `error`, `loading`.
   - Tauri: add a mock entry (for example `?mock=1` or `VITE_MOCK=1`) that calls `mockIPC` from `@tauri-apps/api/mocks` with fixture data, so the browser dev server renders real content without the Rust runtime.
4. **Tooling available.** Browser automation (`playwright-cli` or Playwright MCP), `npx impeccable detect`, `@axe-core/playwright`. If something is missing, say so and downgrade the claim (see "Honesty rules").
5. **Load skills** (as in QLoop): unslop for any prose (microcopy, empty-state text, docs), and TDD bug fix for logic or regression bugs found during the loop.

---

## Execution Steps

### 1. Internal build (silent)
Build or modify the UI. Do NOT present it. Use tokens from `DESIGN.md` only; no ad-hoc colors, sizes, fonts or easings.

### 2. Hard gates (deterministic, run first)
Run every gate and save the output to `qloop-ui/pass-N/gates.json`.

| Gate | Check | Fail means |
|------|-------|-----------|
| G1 | No console errors, unhandled rejections or failed requests on any screen in any state | cap 70 |
| G2 | No text overflow, clipping, overlap or horizontal page scroll at any tested size (check `scrollWidth > clientWidth`, truncated labels without tooltip, overlapping bounding boxes) | cap 70 |
| G3 | `npx impeccable detect <src>` has no unresolved findings (each remaining finding must be justified in writing), **and** the `S` and `R` checks of ui-slop-patterns pass their tier rules (scope `web` only for pages and sites) | cap 75 (impeccable, or any slop **Floor** pattern) · cap 85 (a slop **Reflex** pattern with no valid justification) |
| G4 | Accessibility scan (axe) shows no **serious** or **critical** violations | cap 75 |
| G5 | Every interactive element is reachable and operable by keyboard; visible focus ring on all of them; logical tab order | cap 75 |
| G6 | No placeholder or broken content in any rendered screen: `lorem`, `undefined`, `null`, `NaN`, `[object Object]`, raw i18n keys, broken images | cap 70 |
| G7 | `prefers-reduced-motion` is honoured; no animation longer than the DESIGN.md limit; no animation that blocks interaction | cap 80 |
| G8 | Only token values are used (spot-check computed styles against `DESIGN.md`; flag any off-token color, font size, radius, spacing or easing) | cap 80 |

If a gate cannot be run, record `NOT RUN` with the reason. A `NOT RUN` gate caps the score at 85 and must be listed in the result.

**G3 and the slop catalog.** ui-slop-patterns is the only list of AI-slop patterns; do not copy it into other files. Each pattern has a tier: **Floor** (no waiver), **Reflex** (fix, or justify in writing for this screen in `gates.json` under `slop.waivers`; the critic may reject a boilerplate justification) and **Finish** (advisory, listed in the result). `S` and `R` checks are run by the builder at 1920×1080 and saved in `gates.json`; `J` checks go to the critic. A repeated justification becomes a `DESIGN.md` house rule, not a standing exception.

### 3. Evidence capture (the matrix)
Capture screenshots with the browser tool for every screen across:
- **States:** empty · normal · heavy · error · loading
- **Window size:** 1920×1080 only (Johan's rule: the desktop check is 1080p and nothing more; no 1280×800, no minimum-size, no zoom case, to keep the loop short). Add another size only when the user asks for it.
- **Themes:** dark first (per ui-ux-qloop), then light where the app supports it

Budget rule: **pass 1 and the final pass** use the full matrix. Intermediate passes recapture only the screens and states that changed, plus one full-size regression screenshot of every other screen.

Also capture, when interactive: hover, active, focus, disabled, selected, drag/resize, popovers/modals, and one short recording or frame sequence of each animated transition. Wait for animations to settle before capture, or disable non-essential ones: a transition does not prove a persistent state.

Save to `qloop-ui/pass-N/screens/<screen>__<state>__<size>__<theme>.png`.

### 4. Independent critique
Run the critique as a **separate agent / fresh context** (subagent if available; otherwise a clean prompt that is given ONLY the inputs below). The critic must not have seen the builder's reasoning.

Critic inputs (and nothing else): screenshots, `DESIGN.md`, the rubric in this file, `gates.json`, the reference screenshot, and the defect memory file.

Critic obligations:
- List **at least 5 concrete defects** per pass before scoring (if fewer than 5 real defects exist, list the 5 weakest points and state they are minor).
- Every defect cites: **screen · state · size/theme · element · observed value vs expected value** (for example "Table header, `heavy`, 1920×1080: label wraps to 3 lines; expected single line with ellipsis").
- A finding without evidence is discarded and does not reduce the score, but its absence is noted.
- Scores each category using the **anchored bands** below, citing the evidence for the band chosen.
- Answers: **"What is the single most likely reason a real user would think this looks amateur or broken?"**

Use the critic prompt in Appendix A.

### 5. Score
- Score each of the 9 categories 0-10 using the bands.
- `mean` = average of the 9 scores. `floor` = lowest category.
- `raw = round(mean × 10)`.
- `final = min(raw, lowest gate cap)`.
- Release only if `final ≥ 90` **and** `floor ≥ 8` **and** all gates pass.

### 6. Anti-inflation checks (mandatory)
Apply before accepting any score:
1. **Pass-1 suspicion:** if pass 1 scores ≥ 90, run a second critic with the opposite brief: "find every reason to reject this". Take the lower result.
2. **Jump rule:** if any category rises by ≥ 2 points in one pass, require before/after screenshots of the exact fix. Without them, revert that category to its previous score.
3. **No 10s without proof:** a 10 requires a measurable fact (for example "all 214 measured gaps are multiples of 4px"). Otherwise cap at 9.
4. **Builder cannot grade.** The builder never sets or edits scores.
5. **Rubric blindness (recommended):** give the builder the defect list and category names, not the band text, so it fixes problems instead of optimising for the score.

### 7. Refinement
- Fix defects in this order: (a) failed gates, (b) any category below 8, (c) highest-impact defects, (d) polish.
- Fix **root causes** at the token/component level, not per-screen patches (for example fix the spacing token, not twelve margins).
- Re-run the applicable gates after every change.
- Prose changes: re-run unslop. Logic or regression bugs: red → green → nearby validation via TDD bug fix.
- After each pass, turn recurring critiques into testable rules and regression cases (as in ui-ux-qloop).
- Return to step 2 (gates), then 3-5 for the next pass.

### 8. Stop rules
- **Release** when the pass condition is met.
- **Plateau:** if the final score improves by less than 3 points over two consecutive passes, stop and escalate.
- **Cap reached:** after pass 5 without release, stop and escalate. Do not present the work as finished.
- **Escalation output:** current score, per-category scores, remaining defects ranked by impact, the evidence paths, and a recommended decision (accept with known defects / redesign a component / change the design direction). The user decides.

### 9. Real-environment verification (final pass only)
The browser dev server is not the shipped app. Before release, verify at least the main screens in the **real app webview** (for Tauri, the running app, for example via `tauri-connector` or a native screenshot): fonts load, window chrome and drag regions work, resize behaviour is correct, native menus and shortcuts work, IPC data renders. State which OS was tested; webviews differ (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux). Untested OSes are listed as "not verified".

### 10. Final output
Present the result with a `[QLoop-UI Result]` block:

```
[QLoop-UI Result]
Verdict: RELEASED | ESCALATED
Passes: N/5
Score per pass: 71 → 82 → 91
Final: 91/100 (lowest category: <name> <score>)
Gates: G1 ✅ G2 ✅ G3 ✅ G4 ✅ G5 ✅ G6 ✅ G7 ✅ G8 ✅ (NOT RUN: none)
Slop: Floor 0 · Reflex <n> (<n> justified) · Finish <n> · ids: <SL-xx, ...>
Biggest flaw caught: <one line with evidence>
Remaining known defects: <list or "none">
Verified on: <OS/webview> · Not verified: <OS list>
Evidence: qloop-ui/pass-N/
```

Then update the defect memory (Appendix C).

---

## Rubric: 9 categories, anchored bands (0-10)

Scores use the closest band; evidence must support the band. In between = ±1.

### C1. Layout and spacing
- **10:** every gap and padding is a multiple of the base unit (measured); consistent alignment axes; no orphan elements; margins identical for equivalent components.
- **8:** ≤ 2 off-grid values, no visible misalignment at normal viewing.
- **6:** several off-grid values or uneven padding between similar components.
- **≤ 4:** clear misalignment, cramped or floating elements, inconsistent gutters.

### C2. Typography
- **10:** one type scale from tokens, ≤ 2 families, clear weight hierarchy, comfortable line length and line height, numbers aligned (tabular) where tables/data appear.
- **8:** tokens respected, one or two minor inconsistencies.
- **6:** mixed sizes outside the scale, weak hierarchy, awkward wrapping.
- **≤ 4:** random sizes/weights, poor readability, widows/orphans in key labels.

### C3. Color and contrast
- **10:** only token colors, accent used with restraint, all text and UI contrast meets WCAG AA (measured), semantic colors (success/warn/error) consistent, dark and light both balanced.
- **8:** minor contrast or tint inconsistencies, no AA failures.
- **6:** one or two AA failures or off-token colors; dark theme less polished than light.
- **≤ 4:** muddy palette, many contrast failures, decorative color without meaning.

### C4. Hierarchy and density
- **10:** the primary action and key information are identifiable within 3 seconds; density suits the context (dense for expert tools, airy for marketing); grouping and separation are clear without heavy borders.
- **8:** clear hierarchy with minor competing elements.
- **6:** several elements compete; density feels either wasteful or overloaded.
- **≤ 4:** flat or confusing hierarchy; user cannot tell what matters.

### C5. States and edge cases
- **10:** empty, loading, error, partial, heavy-data and disabled states all designed (helpful copy, clear recovery action); no layout jump between states; long strings handled (ellipsis, wrap, tooltip).
- **8:** all states exist, one or two are plain.
- **6:** some states missing or generic ("No data").
- **≤ 4:** blank screens, raw errors, broken layout on heavy data.

### C6. Interaction and motion
- **10:** hover/active/focus/disabled feedback on all controls; transitions short (typically 120-250 ms), consistent easing from tokens, purposeful (state change or continuity), no bounce, glow or gratuitous effects; reduced-motion honoured; no jank.
- **8:** purposeful and consistent with one or two rough transitions.
- **6:** inconsistent timings/easings or decorative motion.
- **≤ 4:** missing feedback, janky, distracting or blocking animation.

### C7. Keyboard, focus and accessibility
- **10:** full keyboard operation, logical tab order, visible focus, escape closes overlays and returns focus, shortcuts documented and conflict-free, accessible names and roles on all controls, axe clean.
- **8:** minor gaps (for example one missing shortcut hint).
- **6:** some controls unreachable or focus lost after actions.
- **≤ 4:** mouse-only, no visible focus, serious axe violations.

### C8. Desktop fit
- **10:** window chrome and drag regions correct; sensible minimum size and graceful resize (layout reflows, no clipping); respects OS theme; native-feeling menus, context menus, selection and scroll behaviour; no web-page tells (accidental text selection on buttons, rubber-band scroll, default blue links, browser focus outlines).
- **8:** one or two web-page tells.
- **6:** feels like a web page in a window; resize issues at minimum size.
- **≤ 4:** broken resize, missing chrome behaviour, unusable at minimum size.

### C9. Consistency and reference parity
- **10:** every screen could be mistaken for the same product; components behave identically everywhere; quality level equals the golden reference screen; no one-off styles.
- **8:** consistent with one or two small deviations from the reference.
- **6:** visible variation between screens or components.
- **≤ 4:** screens look like different products.

---

## Honesty rules
- Never claim a gate passed if it was not run. Write `NOT RUN` and why.
- Never claim a screen was verified if no screenshot of it exists.
- If the browser tool is unavailable, say so, run only the static checks (G3 `S` checks, G8), cap the score at 70, and do not release.
- Report uncertainty: "Not verified on macOS/Linux" is a valid and required statement.
- Do not hide a failed gate by lowering test coverage (for example removing a state from the matrix).

## Calibration (do once per project, repeat when the rubric changes)
1. Pick 3 screens you consider clearly good and 3 clearly poor.
2. Run the critic on them with this rubric.
3. The rubric is calibrated only if good screens score ≥ 8.5 and poor screens ≤ 6.5. Otherwise tighten the band text until they separate.
4. Store the calibration screenshots in `qloop-ui/calibration/` and re-run the check whenever the model, rubric or design direction changes.

## Artefacts layout
```
qloop-ui/
  DEFECT-MEMORY.md
  calibration/
  pass-1/
    gates.json
    critique.json
    score.json
    screens/
  pass-2/ ...
```

---

## Appendix A: Critic prompt (use verbatim as the critic's instruction)

```
You are an independent UI reviewer. You did not build this interface and you
have no loyalty to it. Your job is to find what is wrong, with evidence.

Inputs: screenshots (screen x state x size x theme), DESIGN.md, the rubric,
gates.json, the reference screenshot, and the defect memory.

Rules:
1. List at least 5 concrete defects BEFORE scoring. Each defect must cite:
   screen, state, size, theme, element, observed value, expected value.
2. Do not praise. Do not summarise. Do not soften.
3. Score each of the 9 categories 0-10 using ONLY the anchored bands.
   Quote the evidence for the band you chose.
4. A 10 needs a measurable fact. Otherwise the maximum is 9.
5. Check every item in the defect memory against the screenshots and flag repeats.
6. Compare against the reference screenshot and state where quality drops.
7. Answer: what is the single most likely reason a real user would find this
   amateur or broken?
8. Judge every `J` pattern in ui-slop-patterns.md that applies (scope `web`
   only for pages and sites). Cite screen, element and pattern id (SL-xx) for
   each tell you find. Also answer the specificity test (SL-19): could this
   screen belong to any other product, and what does only this product show?
9. Read the builder's `slop.waivers` in gates.json. Reject any justification
   that is boilerplate or does not name this screen or element; a rejected one
   counts as unjustified.

Output JSON:
{
  "defects": [{"screen":"","state":"","size":"","theme":"","element":"",
               "observed":"","expected":"","category":"","severity":"high|med|low"}],
  "scores": {"C1":0,"C2":0,"C3":0,"C4":0,"C5":0,"C6":0,"C7":0,"C8":0,"C9":0},
  "evidence_for_scores": {"C1":"","C2":""},
  "most_likely_failure": "",
  "slop_tells": [{"id":"SL-00","screen":"","element":"","evidence":""}],
  "rejected_waivers": [],
  "repeats_from_memory": []
}
```

## Appendix B: Capture script skeleton (adapt paths, routes and sizes)

```ts
// qloop-ui/capture.spec.ts  (Playwright Test)
import { test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import fs from 'node:fs';

const screens = ['dashboard', 'projects', 'settings'];           // routes
const states  = ['empty', 'normal', 'heavy', 'error', 'loading']; // mock fixtures
const sizes   = [
  { name: '1920x1080', width: 1920, height: 1080 },               // the only desktop size checked
];
const themes  = ['dark', 'light'] as const;                      // dark first
const PASS    = process.env.QLOOP_PASS ?? '1';

for (const screen of screens)
  for (const state of states)
    for (const size of sizes)
      for (const theme of themes)
        test(`${screen} ${state} ${size.name} ${theme}`, async ({ page }) => {
          const errors: string[] = [];
          page.on('console', m => m.type() === 'error' && errors.push(m.text()));
          page.on('pageerror', e => errors.push(String(e)));

          await page.setViewportSize({ width: size.width, height: size.height });
          await page.emulateMedia({ colorScheme: theme, reducedMotion: 'no-preference' });
          await page.goto(`/#/${screen}?mock=1&state=${state}`);
          await page.waitForLoadState('networkidle');

          const dir = `qloop-ui/pass-${PASS}/screens`;
          fs.mkdirSync(dir, { recursive: true });
          await page.screenshot({
            path: `${dir}/${screen}__${state}__${size.name}__${theme}.png`,
            fullPage: false,
          });

          // G2: horizontal overflow and clipped text
          const overflow = await page.evaluate(() => ({
            pageScroll: document.documentElement.scrollWidth > document.documentElement.clientWidth,
            clipped: [...document.querySelectorAll<HTMLElement>('*')].filter(el =>
              el.scrollWidth > el.clientWidth + 1 &&
              getComputedStyle(el).overflow === 'visible' &&
              el.children.length === 0 && el.textContent?.trim()).length,
          }));

          // G6: placeholder / broken content
          const bad = await page.evaluate(() =>
            /(lorem|undefined|\bnull\b|NaN|\[object Object\])/i.test(document.body.innerText));

          // G4: axe
          const axe = await new AxeBuilder({ page }).analyze();
          const serious = axe.violations.filter(v => ['serious','critical'].includes(v.impact ?? ''));

          fs.appendFileSync(`qloop-ui/pass-${PASS}/gates.jsonl`, JSON.stringify({
            screen, state, size: size.name, theme, errors, overflow, bad,
            axeSerious: serious.map(v => v.id),
          }) + '\n');
        });
```

Run `npx impeccable detect src/` separately for G3, plus the slop catalog: `node AI-Agents/skills/ui-ux-qloop/scripts/slop-scan/slop-scan.mjs <url> --scope app|web --steps steps.json --out qloop-ui/pass-N/slop.json` for SL-01 to SL-05 and SL-31 to SL-33 (see ui-slop-patterns, "Automated scan"), and the remaining `R` checks by hand with `page.evaluate` (accent footprint from the screenshot pixels), and a keyboard traversal script (Tab through every focusable element, record focus visibility and order) for G5.

## Appendix C: Defect memory (`qloop-ui/DEFECT-MEMORY.md`)

Append one line per defect that appeared twice or more, or any defect that reached a late pass. Format:

```
- [2026-10-23] <pattern> · cause: <root cause> · rule: <permanent rule to prevent it> · seen: <count>
```

Examples of rules to promote: "Never set font sizes outside the token scale", "Every list needs an empty and a heavy-data state", "Every modal returns focus to its trigger".

The critic reads this file on every pass and flags repeats. Repeated defects (seen ≥ 3) become entries in `DESIGN.md` as hard rules.

---

## Quick reference
- Pass = gates green + mean ≥ 9.0 + every category ≥ 8.0 + full-matrix evidence.
- Max 5 passes; stop on plateau (< 3 points over two passes).
- Independent critic, ≥ 5 evidenced defects, anchored bands, no 10 without a measurement.
- Fix root causes in tokens and components, not individual screens.
- Final pass in the real app webview; state which OS was verified.
- Escalate instead of pretending: honest "not verified" beats a false 92.
