# Report, defect memory, calibration

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

## Defect memory
Defect seen twice, or still open in a late pass → append to `qloop-ui/DEFECT-MEMORY.md`:
`- [date] <pattern> · cause: <root cause> · rule: <permanent rule> · seen: <n>`
Seen 3 times → becomes a DESIGN.md rule.

## Calibration
Once per project, and when the rubric or critic model changes: critic scores 3 screens you rate good + 3 you rate poor. Calibrated = good ≥ 8.5 / poor ≤ 6.5; else sharpen the band text. Shots in `qloop-ui/calibration/`.
