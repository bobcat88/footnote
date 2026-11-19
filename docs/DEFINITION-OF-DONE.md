# Definition of done

Every user story in the tracker (GitHub Issues, see [roadmap/EPICS.md](roadmap/EPICS.md)) is done when the shared list below and the
story's own acceptance criteria both hold. A story that cannot meet a line says why in its PR; only Johan waives a line.

## Shared, every story

1. **One branch, one PR**, Conventional Commit title of at most 72 characters, merged by squash, branch deleted locally and on
   GitHub. The PR body says what changed, how it was checked, and what was not checked.
2. **Local checks pass** (CI has no credits): `cargo fmt --all --check`; clippy `-D warnings` on the five CI feature sets (default,
   library, memory, code, llama), plus `encryption` when touched; `cargo test --workspace`.
3. **A test that fails without the change** (for a fix, shown failing first). Tests never write to the real state folder.
4. **Dependencies:** a new crate or package gets a security review (real upstream, code read, install hooks and build scripts
   checked) and passes `python scripts/third_party.py --check`.
5. **Docs follow the code:** README for anything a user sees, the roadmap entry, design notes for a decision.
6. **Installed:** the engine binary is rebuilt and installed (old one renamed `footnote.exe.replaced-<time>`) when the engine changed.
7. **The issue is closed by its PR** ("Closes #N"), with what is left as new issues, never as silent leftovers.

## Extra, by kind of story

| Kind | Also required |
|---|---|
| Screen or UI | QLoop UI gates G1 to G8 on 1920×1080 screenshots, every state (data, empty, error, loading), 0 console errors, keyboard and screen-reader labels, reduced motion; the independent critic before the epic closes |
| Quality claim (search, recall, OCR…) | Decision rule written and committed before any result; adoption only on held-out data; the result recorded in `bench/experiments`, failures included |
| Speed claim | Measured on an idle machine Johan confirms, with the method recorded |
| Personal data | Nothing read, ingested or moved without Johan's go for that step; nothing written outside the state folder; encrypted where the plan says so |
| Security or privacy | Threat model updated; secrets never stored in plain text or logged |
| Platform | Windows checked; Linux and macOS either checked or listed as open in the PR |
