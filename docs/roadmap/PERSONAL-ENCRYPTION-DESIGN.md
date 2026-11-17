# Personal memory encryption: implementation and threat design

Status: draft for implementation and security review, 16 November 2026. No product encryption or real recovery key exists yet.
Applies to personal memory in Footnote One; project memory and Library have separate decisions. User decision and scope are in
[EDITIONS.md](EDITIONS.md); measured feasibility and limitations are in
[SQLCIPHER-FEASIBILITY.md](../../bench/experiments/SQLCIPHER-FEASIBILITY.md).

An experimental `encryption` build feature now provides a keyed database opener in the search crate. Fixture tests exercise
encrypted creation and reopening, wrong and missing keys, plaintext rejection, schema and integrity. This primitive is not wired
into normal memory commands or agent hooks: integration and protected recovery remain unfinished.
The 16 November 2026 Windows checks passed the encrypted-store fixture tests and the default debug workspace suite after upgrading
rusqlite to 0.43; feature-specific release clippy passed. A full release workspace test did not pass: its policy test expects a
test-only environment override that release builds deliberately disable. This is a validation limitation, not a security review.

The experimental recovery primitive generates a 32-byte key with OS entropy and encodes it as
`footnote-recovery-v1:<64 lowercase hex characters>:<16 lowercase hex checksum characters>`. The checksum is the first 8 bytes of
SHA-256 over `footnote-recovery-v1 checksum` followed by a zero byte and the raw key. It catches accidental transcription errors;
it is not authentication. The payload is secret and is identical for text and QR. A fixed test vector protects format
compatibility; fixture tests decode the payload to reopen an encrypted database and independently scan a synthetic QR image.
The returned text and QR matrix are zeroized on drop, but the QR encoder's internal buffers are not guaranteed to be wiped.
The terminal QR is not yet visually verified or scanned into a password manager; key-store integration into normal memory
operations and the complete restore workflow remain unfinished.

The experimental Windows adapter stores a 32-byte key as a generic Credential Manager credential with machine-local
persistence, under a fresh random 128-bit identifier. The identifier is not secret and must later be recorded in database
metadata. Fixture tests verified store/read, absent and malformed entries, deletion of the temporary test entries, and a
same-machine encrypted backup and restore using recovered text and a newly stored key. A child-process test also verifies the
fixture key can be read in a separate process. The adapter is not connected to normal memory commands, has not been tested across
logoff or on another account/computer, and does not implement macOS or Linux stores.
`CredWriteW` can overwrite an existing identifier and has no create-only flag; the implementation checks first and generates
an unpredictable identifier, but the check and write are not atomic against a competing same-user process. This is not a
security review or proof of protection from other processes running as the same user. Product integration must also make
database-to-identifier association durable and recover safely from interruption between storing the key and writing the mapping;
an orphaned key or lost mapping must never trigger silent plaintext creation.

An experimental Windows personal-store lifecycle now creates only a new directory, writes the non-secret credential identifier,
creates an encrypted database and writes a versioned ready marker last. Opening requires the marker, a complete identifier, the
stored key and an existing valid encrypted database; it never creates or migrates a missing one. Tests exercise interrupted or
incomplete metadata, unavailable keys, missing, empty and corrupt databases, no overwrite, and a temporary real Credential
Manager entry. File contents are flushed, but power-loss durability of directory entries and cleanup/recovery of an orphaned
credential after interruption are not yet proven. This API is not connected to normal app commands or an existing store.
The experimental restore API validates the recovery key before creating a new destination, copies the encrypted backup, then
registers the key and writes the ready marker. Fixture tests cover wrong keys and existing-destination refusal; this is not yet
a real-device, password-manager or interrupted-restore acceptance test. A separate explicit API can reveal recovery text
again after verifying the stored key against the database, so interruption before the first display does not itself make the
key inaccessible while the local credential remains available. Read-only Windows file attributes are tested; real removable
media and cross-device use remain to be verified.

An opt-in Windows build exposes experimental `footnote personal` commands: `init`, `status`, `recovery-qr`, `recovery-text`,
`backup`, `restore`, `remember` and `recall`, each requiring an explicit `--store` path. These do not change the live memory database or connect the
agent hooks. `recovery-qr` only writes to an interactive terminal, using ANSI background colors and a quiet zone; the QR
encoder/decoder and the terminal's cell rendering have fixture tests, but no actual password-manager scan or terminal
compatibility acceptance has been performed. `recovery-text` deliberately prints the secret; protect terminal scrollback and
redirection. `restore` reads the text from a pipe and rejects an interactive stdin so it is not echoed or placed in an argument.
Use only disposable fixtures until security review, integration and recovery acceptance are complete. Running `init` creates a
Windows credential; the temporary automated-test credentials are explicitly removed afterward.
`remember` reads bounded JSON and `recall` reads its bounded query from standard input; the latter currently uses lexical search
only and prints returned memory text. The command-level fixture test saves and recalls global and project-scoped memories across
processes, backs up and restores the database, verifies recall on the restored store, and checks that no Tantivy directory is
created. This does not establish the security of other derived data, an agent integration, or a production rollout.

An experimental encrypted-copy primitive exports into a keyed temporary database, checks SQLCipher is active, carries over
`user_version`, `application_id` and `auto_vacuum`, validates SQLite integrity and foreign keys, and publishes only to a new
path. Fixture tests cover a wrong key, an absent source, existing-destination refusal, restored memory and recall, and committed
data still in WAL. The source is attached read-only through an escaped SQLite file URI; Windows fixtures cover a read-only file,
special characters in paths and unchanged source bytes. A controlled WAL test takes five backups while a separate writer has
half of a transaction uncommitted; each snapshot contains only complete, previously committed pairs. Network paths and other
platforms have not been verified. The primitive is available through the experimental personal-store commands; extended
concurrent write stress, interrupted-copy cleanup, crash durability,
password-manager transfer and new-computer restore remain unverified.

For a keyed SQLCipher connection, the experimental keyword path now builds its Tantivy index in RAM and does not read, write or
delete an existing on-disk index. Tests cover memory writes, recall, backup/restore, replacement of a database at the same path,
and a pre-existing plaintext index. Ordinary databases retain their disk index. Old plaintext indexes remain on disk and require
an explicit migration/cleanup decision; this change does not erase them. The in-process cache may retain decrypted index data
until replaced or the process exits, and OS swap/hibernation, other caches and temporary files remain outside this test's scope.
Encrypted connections and the copy operation request SQLite `temp_store=MEMORY` and reject a build that reports forced disk
temporary storage or omits compile-option diagnostics. This covers SQLite temporary tables and indexes, not every possible
temporary artifact, memory paging, swap or crash dump. See [SQLite's pragma documentation](https://www.sqlite.org/pragma.html#pragma_temp_store).

## Protection boundary

Protect an encrypted personal-memory backup or database copied from the machine, including database pages, against a person
without the key. Keep the local key under the user's OS account and provide the same key as a QR code and copyable text for
password-manager backup. Protect all derived artifacts under the same confidentiality policy. A QR code is the secret itself;
no secret or QR image goes into logs, analytics, crash reports, command-line arguments or long-lived temporary files.

This does not protect against malicious code running as the unlocked user, a compromised OS, a copied recovery key, screenshots
or clipboard managers that capture the secret, or plaintext that existed before migration in backups or other applications.
These limits must be explained without claiming that account login is a second encryption factor.

## Required lifecycle

1. Generate a cryptographically random database key locally, using the operating system's secure randomness. Define and version
   its encoding and QR payload; verify exact key bytes round-trip rather than relying on a password manager's OTP scanner.
2. Store a local copy through the platform's credential facility, scoped to the user and database identity. Windows is first;
   macOS Keychain and Linux Secret Service require their own implementations and tests before being advertised as supported.
   If the store is unavailable or locked, fail clearly; never silently create or open a plaintext personal database.
3. Present the QR code and copyable text only on an explicit user action. Make the need to save the key and a separate encrypted
   database backup clear. Confirmation must not log, persist or transmit the secret. Clearing clipboard or screen state is best
   effort and must not claim to erase copies held by other applications.
4. Key every SQLite connection before reading schema or data, including CLI, MCP, hook, daemon, status, model and backup paths.
   Verify the key by reading the schema and fail closed on wrong key, corruption or unavailable key. Existing corpus databases
   remain distinguishable; don't infer the security mode solely from an empty or unreadable file.
5. Restore to a new path from an encrypted backup and the saved key, verify schema version, integrity, record counts and recall,
   then register the key locally. Never overwrite a live database during the rehearsal. Loss of both key copies makes an
   encrypted backup unrecoverable; do not claim a vendor recovery mechanism exists.
6. For migration, create and verify a separate encrypted copy with a coherent snapshot, preserve schema and metadata, test
   interruption and rollback, and retain the original until an explicit production switch. Do not assume `sqlcipher_export`
   copies `user_version` or `auto_vacuum`: validate and set them deliberately. No current instruction authorizes deleting old
   plaintext backups or source files.

## Engineering constraints

- `crates/search/src/store.rs::open` currently serves memory and Library and reads `user_version` immediately. Its callers include
  the hook, MCP, installer and status flows, so simply adding a key to one command leaves other routes broken. Some caller
  relationships are not resolved by the graph; source search is required during implementation.
- `crates/search/src/store.rs::backup` currently uses SQLite's backup API on an ordinary connection. SQLCipher backup behavior,
  destination keying and restore must be proven on the selected version; use an encrypted destination and verify by reopening
  without a key and with the correct key. On 16 November, a disposable SQLCipher 4.20.0 fixture with an encrypted source and
  rusqlite 0.43 returned `backup is not supported with encrypted databases` from this API. This is a fixture result, not a product
  restore test. Never assume the existing backup command preserves encryption.
- The disk-backed Tantivy index contains memory text. An in-memory index is required for protected personal data; audit WAL,
  journal, SQLite temporary files, vector caches, model inputs, hook/daemon data, logs, exports and crash behavior separately.
  OS swap and hibernation are outside the application's direct guarantee and must be described honestly.
- The experiment bundled SQLCipher **4.10.0**, not a production security baseline. Zetetic's
  [4.15.0 release](https://www.zetetic.net/blog/2026/04/28/sqlcipher-4.15.0-release/) fixes a conditional defensive-mode bypass
  requiring an existing SQL injection or unrestricted SQL access; its stated impact is schema integrity/availability, not
  disclosure or broken cryptography. [4.19.0](https://github.com/sqlcipher/sqlcipher/releases) includes additional low-risk
  fixes. Select and pin a current maintained version with these fixes, inspect the actual bundled version, review licences and
  rerun compatibility, benchmark and security tests. The prior 4.10 measurement cannot establish the new build's performance.
  A throwaway Windows MSVC release probe using rusqlite 0.43 and its bundled source reported SQLCipher 4.20.0; it emitted the
  same missing debug-symbol linker warnings. This is a build probe, not yet a product integration or security acceptance.
- Windows Credential Manager can store a local key using the user's logon session; it is not protection from the same user's
  malware. See [Microsoft's password handling guidance](https://learn.microsoft.com/en-us/windows/win32/SecBP/handling-passwords)
  and [SQLCipher API and export notes](https://www.zetetic.net/sqlcipher/sqlcipher-api/).

## Acceptance and unresolved choices

Prove encryption on disk and rejection of missing/wrong keys, deterministic correct read/write/reopen, QR and text recovery,
new-device restore, no plaintext fallback, interruption recovery, corpus compatibility, protected derived data and honest failure
messages. Measure cold and warm latency, rebuild and write costs, and retain the performance methodology. Review the design and
implementation against the threat model; a benchmark or this document alone is not a security review.

Before coding, choose the exact versioned recovery payload and checksum, database identity and key-store naming, locked-store
UX, supported platform rollout, key rotation and backup interoperability. Bind those decisions to actual tests. No production
data migration or deletion is included in this design step.

## Derived-data audit (17 November 2026, story #132)

Every place personal-memory content or something derived from it can land, checked against the protection boundary above. "Text"
means memory text or prompt text; hashes, ids, sizes and timings are metadata.

| Artifact | Holds | Status with an encrypted store |
|---|---|---|
| Memory database (rows, vectors) | Text, vectors | **Protected**: SQLCipher pages (#65) |
| Keyword index (Tantivy) | Text terms | **Protected**: built in RAM for keyed connections, never on disk (#65). An older plaintext `memory.db.tantivy` stays until the migration decides its cleanup (#129) |
| SQLite temporary tables | Text | **Protected**: `temp_store=MEMORY` forced on keyed connections (#65) |
| Encrypted backup | Text | **Protected**: same key (#65) |
| `audit.jsonl` | Tool name, args and result **hashes**, timings, session id | Metadata only. A hash of a short query could be guessed by trying candidates; acceptable, recorded |
| `memory-hook/activity.jsonl` | Memory ids, project names, times | Metadata only; never prompt text |
| `metrics.jsonl`, `usage/lean.jsonl`, `jobs/` | Step names, timings, sizes | Metadata only |
| `memory-hook/hook.log` | Error messages | **Gap**: an error can quote its input; the old Python hook wrote a prompt fragment there on 14 Nov, and some SQLite FTS5 errors quote search terms. Story #172 |
| Process memory (daemon, MCP server, launcher) | Decrypted text, vectors, index | Outside the boundary (malicious code as the user is excluded); lives until the process exits |
| Swap, hibernation file | Pages of process memory | Outside the boundary; BitLocker on the system disk covers them |
| Windows crash dumps | Process memory | No local dumps configured on Johan's machine (`LocalDumps` absent); a dump would hold decrypted pages. Story #173 |
| Agent transcripts (`~/.claude`, `~/.codex`, `~/.dsh`) | Memories injected into prompts, in plain text | **By design outside Footnote's reach**: the agent receives the text and its vendor stores it. The encrypted store protects the store, not what agents keep; the README must say so |
| `corpora.json`, `projects/` | Paths and project names | Metadata; not memory content |
