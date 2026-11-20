# Personal memory encryption: build check (17 November 2026, overnight item 5)

Rule written before the run (docs/roadmap/EDITIONS.md, 16 Nov): personal memory is encrypted regardless; project memory and
Library only if encrypted search keeps at least 90 % of today's speed. This item was the build check and, if it built, the
measure on a copy of `memory.db`.

## Initial attempt: build blocked; no timing taken

- `rusqlite 0.38` with `bundled-sqlcipher-vendored-openssl`, in a throwaway crate outside the repository (MSVC 1.100,
  Windows 11): **fails** while configuring OpenSSL. `openssl-src` runs OpenSSL's `Configure` with Perl, and the only Perl here is
  Git's Cygwin build, which lacks `Locale::Maketext::Simple` and produces Unix paths. OpenSSL documents a native Windows Perl
  (Strawberry Perl) for MSVC builds.
- No measure was taken (the plan said stop if the build fails). The live `memory.db` was not touched.

## Options (none taken tonight)

1. **Install Strawberry Perl** (`winget install StrawberryPerl.StrawberryPerl`), then the planned build should work; GitHub's
   Windows runners already ship it. Cost: a third tool to build Footnote on Windows, and OpenSSL compiled into every binary
   (a few MB, and its own security updates to follow). Needs your go: it is a new third-party install.
2. **SQLite3 Multiple Ciphers** (utelle/sqlite3multipleciphers, MIT, version 2.7.1 of November 2026): an encryption extension
   built into the SQLite amalgamation with its own ciphers, **no OpenSSL**, developed mainly on Windows, and able to read and
   write the SQLCipher 4 format (so either choice keeps files compatible). Rust projects already use it: Turso's libsql bundles
   it (`libsql-ffi/bundled/SQLite3MultipleCiphers`), and dbx ships it on desktop as a `sqlite-multiple-ciphers` feature
   (dbx PR #7286). Not evaluated tonight: how it plugs into rusqlite 0.38 (libsqlite3-sys has no feature for it), its build
   on Linux and macOS, and its speed. Found by one search; read nothing beyond the READMEs and the PR text.

Recommendation: try option 2 first in a throwaway crate (no new system tool, smaller binary), keep option 1 as the fallback,
then run the measure planned here: recall p50 and p95 on a copy of `memory.db` (backup API, never the live file), plain against
encrypted with the same library, keyword index in memory, on an idle machine.

## Follow-up, 16 November 2026: SQLCipher build and warm recall

After authorization to install and benchmark, Strawberry Perl was installed through winget; the installer hash was verified. A
native MSVC release build of rusqlite 0.38 with `bundled-sqlcipher-vendored-openssl` succeeded and reported SQLCipher 4.10.0
community. OpenSSL's build failed on the original very long temporary path with missing source files; a short
`CARGO_TARGET_DIR` fixed it. Linking emitted LNK4099 warnings for missing OpenSSL debug symbols. This is a Windows build check,
not a cross-platform or security audit.

At commit `161afc38ac10a1b49fcd63f287f5393cd56d7fb6`, an isolated copy of the source used SQLCipher for both variants and
replaced the on-disk Tantivy index with a RAM index. The live database was opened read-only; SQLite's backup API made a
temporary snapshot, which contained 234 memories. SQLCipher exported that snapshot to a separate encrypted database. An
unkeyed attempt to read its schema failed. Both temporary databases were removed when the process exited. No production code
or live data was changed.

The harness called the actual `footnote_memory::recall` with scope `footnote`, no embedding models or tag filters, limit 10,
and 10 fixed lexical queries. It alternated 20 blocks in each order, rebuilding and warming the RAM index outside the timed
interval. Each condition has 200 timed calls. Result counts per query were 10, 10, 10, 10, 1, 10, 10, 10, 10, 10; all 400
outputs matched the plaintext reference exactly. Three total CPU samples before the final run were 8.9%, 5.6%, 5.6% on an
Intel Core Ultra 7 270K Plus.

| Warm recall | Plain | Encrypted | Plain time / encrypted time |
|---|---:|---:|---:|
| p50 | 0.899 ms | 0.912 ms | 98.6% |
| p95 | 1.057 ms | 1.063 ms | 99.4% |

These results exceed the predeclared 90% speed ratio **for this small, warm personal-memory workload only**. They do not
establish the threshold for project memory or Library, cold start, opening/key derivation, index rebuild, writes, concurrent
access, or the existing disk-index implementation. The RAM index is an experimental adaptation and has not shipped. Initial
runs with Tantivy's multi-threaded writer produced changing result order even across plaintext rebuilds; using one writer
thread made this run deterministic. Those initial timings are not used for the verdict.

**Decision, 16 November:** This encryption approach is validated for personal data on the strength of the measured performance
gate. Personal memory encryption remains the product direction. This validates feasibility and performance for the measured
workload; implementation, key management, migration and recovery, protection of derived data, and security review remain to be
completed before claiming that personal data is protected in the product. Project memory and Library remain separate decisions.

To reproduce, use a disposable copy of this commit, change the workspace rusqlite features from `bundled` to
`bundled-sqlcipher-vendored-openssl`, copy [sqlcipher_recall_bench.rs](sqlcipher_recall_bench.rs) to `src/bin/encryption_bench.rs`,
and in `crates/search/src/lexical.rs` replace the disk-index creation/open block with `let built: Option<i64> = None; let index =
Index::create_in_ram(schema);`, remove the `write_built` call, and replace `.writer(WRITER_HEAP)` with
`.writer_with_num_threads(1, WRITER_HEAP)`. Build in release mode with `--no-default-features --features memory --bin
encryption_bench` and run the resulting binary with the source database path as its sole argument. Use a short Cargo target
path on Windows. The key in the harness is intentionally a temporary benchmark key, not a production key-management design.

### Follow-up: SQLCipher 4.20.0

On 16 November, a separate isolated Windows release build upgraded the disposable workspace to rusqlite 0.43 and reported
SQLCipher 4.20.0. This addresses the version concern documented in the [encryption design](../../docs/roadmap/PERSONAL-ENCRYPTION-DESIGN.md);
it does not establish a completed security review. The API update required typing the `None` progress callbacks in both the
existing backup code and the harness as `None::<fn(rusqlite::backup::Progress)>`. The same missing debug-symbol linker warnings
were emitted. No production dependency was changed.

With the same harness, query set and 234-memory snapshot procedure, three CPU samples before this run were 1.5%, 2.3%, 2.7%.
Plain p50/p95 were **0.972 / 1.211 ms**; encrypted p50/p95 were **0.974 / 1.205 ms**, respectively 99.8% and 100.5% of plain
speed by the same time-ratio calculation. Each condition had 200 calls; all outputs matched and an unkeyed read failed. Small
differences, including encrypted p95 being faster, should not be interpreted beyond measurement noise. The same warm-workload,
RAM-index and non-extrapolation limits apply. To reproduce with 0.43, use the callback type above in addition to the RAM-index
changes described earlier.
