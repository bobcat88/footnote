# Stack ingest timing B: the keyword-first, batched, import-ordered flow on the same 8 libraries (issue #307)

Not a quality benchmark. Question for the last box of #307: does a project get keyword-searchable docs within seconds while the embeddings finish in the background, and what does the flow do to the total time recorded in [STACK-INGEST-TIMING-2026-11-20.md](STACK-INGEST-TIMING-2026-11-20.md) (548 s, embedding 91.7%)?
Harness: `stack_ingest_timing.py <run-dir> <footnote.exe> --flow <project-folder>` ([script](stack_ingest_timing.py)); raw timestamps in [STACK-INGEST-TIMING-2026-11-20-B.log.txt](STACK-INGEST-TIMING-2026-11-20-B.log.txt).

## Setup

- Machine as in the first report (Intel Core Ultra 7 270K Plus, Windows 11), 10:33 to 10:42 on 2026-11-20. Live CPU load sampled right before the run (3 samples over 3 s: 1.5 to 5.4%); no other heavy job ran. Isolated state dir (`<run-dir>/state`), so the real cache was untouched; the shipped pair (bekko-a8m + gte-small, CPU) was copied into it and selected with `models use`.
- Binary: `cargo build --release` of main at `d495122` (includes #317 and #323), sha256 `477bbd96dea4b8d0...`. The installed `footnote.exe` is not on this machine's PATH, so it was built.
- One command, run once: `footnote docs ingest <8 start URLs> --project <the Footnote repo>`. It builds each library's keyword index in turn in the foreground, then starts one detached `docs embed` worker that embeds the libraries (project-imported first, then smallest first). The harness records, in seconds from the start of the command: when each library's database is renamed into place (keyword-searchable; the build renames it only once indexed), when `docs list` first reports it `ready` (every chunk has a vector), and the first `footnote find` that returns a docs hit.
- Same start URLs as the first report, except Pydantic starts at `pydantic.dev/docs/validation/latest/` (the old `docs.pydantic.dev` start narrows the crawl scope, see the first report) and Vite at `vite.dev/guide/` (the redirect target). Pydantic has 93 pages here, 94 before.
- Failed first attempt, not counted: I started the same command in a state dir that had the model files but no model selected, so no embedding worker started and every library stayed `keyword-only` (`docs ingest` printed "no embedding models selected"). The keyword times of that attempt (serde 5.0 s, last library 44.2 s, command returned at 44.2 s) match the run below; the run below is the only one with embeddings. The harness now copies and selects the models itself.

## Results (one run)

| Library | Pages | Keyword-ready s | Fully embedded s | Embedding of that library after it was keyword-ready, s |
|---|---|---|---|---|
| Serde (imported by the project, first) | 38 | 5.5 | 50.2 | 44.7 |
| Flask | 77 | 9.7 | 117.0 | 107.3 |
| SQLAlchemy 2.0 | 222 | 25.5 | 567.4 | 541.9 |
| Requests | 26 | 27.8 | 86.6 | 58.8 |
| Jinja | 16 | 32.3 | 77.5 | 45.2 |
| Pydantic | 93 | 37.5 | 190.0 | 152.5 |
| Vite guide | 24 | 41.3 | 65.3 | 24.0 |
| Tokio tutorial | 11 | 44.1 | 53.2 | 9.1 |
| **Stack** | **507** | **44.1 (last library)** | **567.5 (last library)** | |

- First search usable: the first `footnote find` returned a docs hit at **5.9 s** (Serde alone; keyword-only, as the corpus had no vectors yet). The `docs ingest` command returned, with all 8 keyword indexes built, at **44.1 s**; the user is blocked for that long, not for the embedding.
- Embedding wall-clock after the command returned: 567.5 - 44.1 = **523.4 s**. Libraries became fully embedded in the order Serde, Tokio, Vite, Jinja, Requests, Flask, Pydantic, SQLAlchemy.

## Compared with the first measurement (sequential crawl, index, embed per library)

| | First (548.2 s) | This flow |
|---|---|---|
| Whole stack keyword-searchable | 45.4 s (summed) | 44.1 s |
| First library searchable | 4.3 s | 5.5 s |
| Whole stack fully embedded | 548.2 s | 567.5 s |
| Embedding alone | 502.8 s | 523.4 s |

## Conclusion

1. **The claim holds: a project can search its docs keyword-first within seconds while embeddings finish in the background.** The first library answered a search at 5.9 s, all 8 were keyword-searchable at 44.1 s, and `docs ingest` returned then; the remaining 523 s of embedding ran in the detached worker without blocking. In the first report's sequence the user waited for the embedded state; here the wait before the first search is seconds.
2. **No total speedup.** The stack was fully embedded at 567.5 s against 548.2 s (3.5% slower), embedding alone 523 s against 503 s (4% slower). One run each, so a difference of this size is within what I would expect from run-to-run variation; I make no claim that the new flow is slower or faster, only that it did not make the total shorter. The reason is structural: the flow changes when the user can search, not the amount of embedding work (same chunks, same models, CPU, one worker).
3. **SQLAlchemy is most of the wait.** It took 25.5 s to be keyword-searchable and, embedded last, finished 377 s after the other seven libraries were all fully embedded (at 190 s). So 7 of 8 libraries were hybrid-ready after 3 minutes, and the 9 minutes of the whole stack is one library. Whether that library should go earlier depends on the project; the ranking used the project's import counts and I did not test a project that imports SQLAlchemy.
4. Not measured: search quality of keyword-only versus hybrid on these libraries (not this report's question), whether the order of embedding changes anything beyond which library is hybrid-ready first, GPU, boilerplate pruning (see the first report's priorities, unchanged by this run), and the `docs ready n/m` launcher line (covered by #323's tests, not timed here).
