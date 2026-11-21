---
name: make-slides-and-reports
description: Use when producing a deck, report, memo or spreadsheet for other people (PPTX, DOCX, XLSX, ODP, PDF), above all from source documents or contracts: purpose and audience first, one message per slide, claims traceable and labelled, content kept editable, every page rendered and checked, nothing private shipped. Runs office_check.
license: MIT
---

# Make slides and reports

## Run
`python <skills>/make-slides-and-reports/scripts/office_check.py <files...> [--json]` → reads the file, never edits it. Exit 1 on FAIL. Tests: `python -m unittest test_office_check` in `scripts/`.

| Id | Rule | Level |
|---|---|---|
| O1 | placeholder: `{{` / TODO / TBD / XXX / lorem ipsum | FAIL |
| O2 | personal local path: `C:\Users\…` / `/home/…` / `/Users/…` | FAIL |
| O3 | broken encoding: `Ã©` / `â€™` / U+FFFD | FAIL |
| O4 | review comments still in the file | FAIL |
| O5 | hidden slide / O8 picture without alt text | WARN |
| O6 | speaker notes / O7 author metadata: shipped, read them | INFO |

## Before any slide
- Purpose: inform / align / decide; a meeting without a decision mandate doesn't get a decision slide
- Audience, owner of the decision, who validates which claims, delivery format: spoken deck / reading document / appendix
- Storyboard as sentences: each slide title is its message; read the titles alone, they tell the story
- Reuse the client's template, fonts, colours, logos; missing font = a labelled draft, never a silent substitute

## Each slide or page
- One message + its evidence / detail to notes or appendix / one visual operation: compare / zoom / reveal / sequence
- Text, tables, charts native and editable; a diagram image only if regenerable from its source, said so
- Charts: labelled axes, units, source line; colour is never the only cue
- Numbers: one format and unit throughout; local currency and decimal conventions; totals add up

## Claims from sources (contracts, specs, minutes)
- Two passes: extract facts (actor / obligation / object / deadline / condition / reference) → then interpret (consequence / owner / open question)
- Label every working claim: source fact / dated project fact / hypothesis / open question / needs expert validation
- Never infer a breach, approval, compliance or readiness from circumstantial evidence
- Actions: owner / date or next decision / forum; unknown stays "unassigned", never invented
- Expert gates by claim type (project, IT, legal); until all required gates close it's a "working version", not "final"

## Before delivery
- Render every page of the actual file at normal size: clipping / overlap / density / contrast / brand
- `office_check` clean / spell-check the source and the exported text (accents, diacritics) / reading order and alt text
- Show only shareable references (article, page, ticket, dated minutes); internal file paths and hashes stay in a private ledger
- Version compare: align by meaning, not slide number; report added / corrected / removed claims and why
- Say which copy owns future edits: the generator source or the office file
