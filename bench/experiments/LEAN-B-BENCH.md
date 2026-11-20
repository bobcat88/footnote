# Lean B bench plan (#239)

Question: does `rules/lean/LEAN-B.md` lower **cost per accepted task** against Ponytail and Caveman? First pilot run: see LEAN-B-RESULTS-2026-11-20.md. No
result in this plan is a measurement.

## Arms
`bare`, `ponytail` (installed plugin, ultra and full), `caveman`, `lean-b-code` (sections 1 to 3 only), `lean-b-full`
(sections 1 to 5, prose full). Same model, reasoning effort and tasks in every arm; arm order randomised per task.

## Tasks
The ladder tasks of #225 (four levels), one model tier first (Opus 5.5, to match Ponytail's claim), 3 runs per task and arm,
paired by task. Accepted means the task's own test passes and a reviewer finds no cut from the never-cut list.

## Record, one JSON line per run
`{"arm","task","run","accepted":bool,"turns","uncached_in","cache_write","cache_read","output","seconds"}`.
Token counts come from the session transcript (`footnote usage --file`). Activation proof: the arm's hook or rule file
appears in the transcript, otherwise the run is void.

## Score
`python bench/experiments/lean_b_score.py runs.jsonl [--price in,write,read,out]` prints per arm: runs, accepted,
cost per accepted task (failed runs count against the arm), the same as a change versus `bare`, output tokens and turns.

## Prose check (third Done-when box)
Static: section 4 of the rule file lists the off-list. Dynamic: in the `lean-b-full` arm, grep every commit message and PR
body the agent wrote for fragment-style compression; any hit fails.

## How to run
Not automated here: it needs paid model runs, so it waits for an explicit go and an idle machine. Run each arm through the
Flow harness (flow/), write the JSONL, then score it. Mark the first two #239 boxes only after real output exists.
