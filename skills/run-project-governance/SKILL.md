---
name: run-project-governance
description: Use when shaping or steering a project rather than writing code: turning a vague request into a spec, interviewing the user to settle decisions, splitting work into tickets, triaging issues, prioritising a backlog, keeping RAID and decision logs, writing status reports or steering packs.
license: MIT
---

# Run project governance

## Settle what to build
- Facts are yours to look up (code, docs, data); decisions are the user's
- Map open decisions as a tree; ask every question whose prerequisites are settled, in one round
- Each question: number / bold title / the question / your recommended answer, worded so "yes" accepts it
- Recompute after answers; done when nothing is open and nothing is silently assumed; confirm before acting

## Spec
- Problem / users / outcome measure / in scope / out of scope / constraints / open questions
- Acceptance criteria as observable checks: "given / when / then" or a command and its expected output
- Name what is deliberately not done and why

## Tickets
- Vertical slices: each one shippable and testable alone, end to end, smallest first (tracer bullet)
- Each ticket: goal / acceptance checks / dependencies / size; no ticket says "and" twice
- Order by dependency, then risk: unknowns early

## Triage
- One category (bug / enhancement) + one state (needs-triage / needs-info / ready / wontfix) per issue
- Check it's not a duplicate and not already built; reproduce bugs from the reporter's steps before labelling
- needs-info → list what is known and the exact questions; wontfix → reason + link; ready → a brief someone can start from

## Prioritise
- Score with one method and show the inputs: RICE = reach × impact × confidence ÷ effort / WSJF = cost of delay ÷ size
- Low confidence → a cheap experiment first, not a big build
- Say what drops when something new comes in

## Logs
- RAID: risk (probability, impact, owner, mitigation, trigger) / assumption (how and when it gets checked) / issue (owner, due) / dependency (who, by when)
- Decision record: context / options / decision / consequences / date / who decided; superseded, never edited away
- Review assumptions on a schedule; an unchecked assumption is a risk

## Status and steering
- Status: RAG with trend and one-line reason / done since last / next / blocked by / decisions needed, with deadline
- Steering pack: the decisions asked first, each with options, recommendation, cost of delay; detail in appendix
- Red needs a recovery plan; amber needs a trigger for turning red
- Same format every time, so changes stand out
