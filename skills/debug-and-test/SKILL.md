---
name: debug-and-test
description: Use when something is broken or a test fails (bug report, crash, wrong output, flaky test, build error, regression) and before claiming a fix works. A reproducing loop first, one hypothesis at a time, a regression test proved red then green, no completion claim without fresh output. Runs red-green and leftover-check.
license: MIT
---

# Debug and test

## Run
- `node <skills>/debug-and-test/scripts/red-green/red-green.mjs --test "<cmd>" [--base HEAD] <fix files...>` → reverts the fix files to `base`, the test must fail; restores them, the test must pass. Exit 0 proved / 1 not proved / 2 usage. Prints a backup path before touching files.
- `node <skills>/debug-and-test/scripts/leftover-check/leftover-check.mjs [base] [--json]` → lines added since `base` (default: uncommitted). Exit 1 on FAIL; `leftover-ok` on a line keeps it.

| Id | Rule | Level |
|---|---|---|
| D1 | `[DEBUG-xxxx]` tag | FAIL |
| D2 | breakpoint: `debugger;` / `breakpoint()` / `pdb.set_trace()` | FAIL |
| D3 | focused test: `.only(` / `fit(` | FAIL |
| D4 | disabled test: `.skip(` / `xit(` / `#[ignore]` / `@pytest.mark.skip` | WARN |
| D5 | debug print: `console.log` / `dbg!` / bare `print(` | WARN |
| D6 | stub: `todo!()` / `unimplemented!()` / `NotImplementedError` | WARN |

## 1. Loop before theory
- One command that goes red on this exact bug: failing test / curl or CLI output diff / browser script / replayed input / `git bisect run <cmd>` / old vs new build
- Tighten it: seconds, not minutes / pinned time, seed, network / asserts the reported symptom, not a nearby one
- Flaky: run it 100× in parallel, measure the failure rate; that is the loop
- No loop possible → stop; list what you tried; ask for access, a redacted log or trace, or leave to add instrumentation
- Redact secrets in anything you capture or quote

## 2. Find the cause
- Read the whole error and stack / what changed since it last worked: `git log -p`, deps, config, env
- Minimise: drop inputs, callers, config one at a time until every piece matters
- Several components: log what enters and leaves each boundary once → work only in the failing layer
- Find a working sibling in the codebase → list every difference
- 3–5 ranked hypotheses, each "if X, changing Y removes the bug" / one probe = one variable
- Debugger or REPL first / logs tagged `[DEBUG-xxxx]` so one grep removes them / never log everything and grep
- Slow, not wrong → baseline first, then `make-code-faster`

## 3. Fix
- Regression test at the seam where the bug lives → red for the right reason → fix the cause at its source → green → rerun the step 1 loop
- No seam to test it → say so; that is a design finding
- `red-green` proves the test catches the bug
- Failed fix → back to step 2, never stack fixes / 3 failed fixes → stop, report, question the design before a 4th

## 4. Tests that earn their place
- Name the code change that turns the test red before writing it
- Assert behaviour, not mocks / mock only slow or external edges / no test-only methods in production code
- One behaviour per test / name says the behaviour / no sleeps: wait on a condition with a timeout
- New behaviour: failing test first → simplest code to green → refactor only while green

## 5. Done means shown
- Claim → evidence from this session: tests pass = full-suite output, 0 failures / build = exit 0 / bug fixed = step 1 loop green / delegated work = its diff, read
- Not evidence: an earlier run / "should pass" / a linter standing in for a build / an agent's report
- Report every failure in the full run, related or not
- Clean up: `leftover-check` passes / throwaway harnesses deleted / root cause in the commit message
