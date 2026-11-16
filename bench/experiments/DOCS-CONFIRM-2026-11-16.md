# Documentation search: confirmation on fresh questions

Written 16 November 2026, before any system ran on these libraries. Confirms (or not) the diagnostics of
[DOCS-FOLLOWUPS-RESULTS-2026-11-15.md](DOCS-FOLLOWUPS-RESULTS-2026-11-15.md), which reused Grounded Docs' 59 already-seen questions.

## Data

- `bench/datasets/docs-confirm/dataset.yaml` (sha256 `88dad3e8b3d405c743b200e54d7b1008aef624fa99998b643a926491ead5e9e7`):
  40 questions, Pydantic 20 and Svelte 20, intents api-lookup, conceptual, comparison, troubleshooting; graded page URLs
  (3 primary, 2 related), 53 distinct URLs, each checked to answer HTTP 200. Written by Claude from the sites' page lists, without
  running either system. Footnote's side wrote them, the reverse of the earlier set (written by Grounded Docs' author).
- Crawl recipe `bench/datasets/docs-confirm/recipe.json`: pydantic.dev/docs/validation/latest/ and svelte.dev/docs/svelte/,
  200 pages each, no exclusion, same for both systems.

## Systems, frozen

- **Grounded Docs 3.4.1**, hybrid with EmbeddingGemma 300M Q8_0 through llama.cpp, telemetry off (as in DOCS-VS-GROUNDED.md).
- **Footnote**, shipped model pair, hybrid, `--per-source 1` (one result per page, chosen by the earlier diagnostics), its own
  crawler (`footnote crawl`, with this branch's fixes: one file per page, redirect scope).

## Rule

- **Primary, end to end:** Footnote (own crawl, hybrid, `--per-source 1`) minus Grounded Docs (own crawl, hybrid), nDCG@5 with
  Grounded Docs' metric code, paired bootstrap over the 40 questions (10,000 resamples, seed 20261116).
  **Equivalent:** 95 % lower bound above −0.05. **Better:** above 0. Otherwise **worse**.
- **Secondary, same text:** Footnote `--per-source 1` on Grounded Docs' exported pages minus Grounded Docs; same rule.
- **Decision:** if the primary is at least equivalent, the crawler ships and documentation searches default to one result per
  page. Otherwise neither, and the per-library results say what to fix.
- Reported, not judged: Footnote as shipped (no cap), keyword only, EmbeddingGemma same weights with the cap, crawl coverage
  of the judged URLs, per library and intent. Query latency not measured.
- 40 questions is small: an interval crossing zero means not distinguishable, never better.

## Deviation, before any score (16 November, 05:30)

First run: Grounded Docs' Pydantic crawl failed and stopped the job: it fetched `llms-full.txt` (the whole documentation in one
file) and one of its chunks reached 2,118 tokens, above the 2,048 a llama.cpp slot holds (same server settings as run 1). Both
systems now skip `llms-<name>.txt` (pattern `/llms-[a-z]+.txt$/`: `llms-full`, `llms-medium`, `llms-small`), which are copies of the pages, not
pages. Footnote's first crawl is discarded and redone with the same recipe. No search had run.

## Second deviation, after the coverage count and before any Footnote end-to-end score (05:50)

Seen before this note: the same-text results (Footnote as shipped against Grounded Docs) and the crawl coverage. Footnote's crawl
covered 0 of Pydantic's 24 judged URLs: it fetched the Markdown copies listed by llms.txt (`.../concepts/config/index.md`) and
recorded those addresses, which the metric's URL normalisation does not map to the page (`.../concepts/config/`). The pages
were fetched; their recorded address was wrong for citing. Fix in the crawler (recorded address = the HTML page a reader would
cite), then Footnote's crawl is redone. The end-to-end step had not produced a score (it failed to start the binary).
The same-text capped and same-weights steps had also failed before scoring (a missing input, a hard-coded library list).

## Results (16 November, 05:43 to 05:48)

| System | nDCG@5 | Recall@5 | Hit@3 | vs Grounded Docs hybrid, paired 95 % interval |
|---|---|---|---|---|
| Grounded Docs hybrid, own crawl (184 + 86 pages) | **0.634** | 0.704 | **0.750** | reference |
| **Primary:** Footnote, own crawl, hybrid, `--per-source 1` | 0.515 | 0.671 | 0.600 | **−0.119 [−0.251, +0.023] → worse** |
| **Secondary:** Footnote, Grounded Docs' pages, `--per-source 1` | 0.557 | **0.742** | 0.750 | −0.076 [−0.169, +0.020] → worse |
| Footnote, Grounded Docs' pages, as shipped (no cap) | 0.543 | 0.717 | 0.750 | −0.091 [−0.184, +0.005] |
| Footnote, one global database (no cap) | 0.536 | 0.692 | 0.750 | global − per library −0.007 [−0.034, +0.018] |
| Grounded Docs full-text | 0.491 | 0.629 | 0.675 | |
| Footnote keyword, `--per-source 1` | 0.479 | 0.654 | 0.575 | −0.155 [−0.241, −0.068] |

Per library (same pages, capped): Pydantic 0.444 against 0.479, Svelte 0.670 against 0.789. Crawl coverage of the judged URLs
after the address fix: Footnote 53/53, Grounded Docs 53/53.

- **Verdict: worse on both judged comparisons; the rule's decision applies: the crawler does not ship yet and documentation
  searches keep their current default.** Both intervals cross zero, so the gap is not proven either; equivalence is not shown.
- The earlier diagnostics did not hold on fresh questions: the page cap gained +0.014 here (+0.044 on the reused set).
- Footnote finds the judged pages (higher recall on the same pages) but ranks them lower. Reading Grounded Docs' code
  (3.4.1, `addDocuments`, `SemanticMarkdownSplitter`): each embedded chunk is prefixed with the page title, URL and heading path,
  chunks follow the heading tree (500 to 5,000 characters, preferred 1,500), and results are assembled per page with parent
  and sibling chunks. Footnote embeds only the last heading and the chunk text. These are the next things to test, tuned on
  the two seen sets and decided on a third, unseen one.
- Both question sets are now seen: any further tuning must be judged on a new set.

Reported, not judged (06:01): Footnote with the same EmbeddingGemma-300M weights and `--per-source 1` on Grounded Docs' pages:
nDCG@5 0.545, recall@5 0.729 (shipped pair, same pages: 0.557). The diagnostic gain on the reused set (+0.019 over Grounded
Docs) did not hold either: on fresh questions the larger model did not beat the shipped pair inside Footnote.
