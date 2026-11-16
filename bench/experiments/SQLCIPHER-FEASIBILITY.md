# Personal memory encryption: build check (17 November 2026, overnight item 5)

Rule written before the run (docs/roadmap/EDITIONS.md, 16 Nov): personal memory is encrypted regardless; project memory and
Library only if encrypted search keeps at least 90 % of today's speed. This item was the build check and, if it built, the
measure on a copy of `memory.db`.

## Result: does not build on this machine as planned; no timing taken

- `rusqlite 0.38` with `bundled-sqlcipher-vendored-openssl`, in a throwaway crate outside the repository (MSVC 1.100,
  Windows 11): **fails** while configuring OpenSSL. `openssl-src` runs OpenSSL's `Configure` with Perl, and the only Perl here is
  Git's Cygwin build, which lacks `Locale::Maketext::Simple` and produces Unix paths. OpenSSL documents a native Windows Perl
  (Strawberry Perl) for MSVC builds.
- No measure was taken (the plan said stop if the build fails). The live `memory.db` was not touched.

## Options for Johan (none taken tonight)

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
