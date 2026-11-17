# Footnote: next 20 feature goals

Prepared 16 November 2026 from [BACKLOG](BACKLOG.md), [P1 plan](P1-PLAN.md), [editions](EDITIONS.md), and the
[encryption experiment](../../bench/experiments/SQLCIPHER-FEASIBILITY.md). This is a prioritized planning wave, not a claim that
the items have shipped or a commitment to finish all of them overnight. Carry unfinished items forward. Since 17 Nov 2026
the open goals are stories on GitHub Issues ([EPICS.md](EPICS.md)), which hold their status. Done by 17 Nov: 1-5 (#65), 9 (tried,
failed on dev, #69 #71), 10 (#161), 11 (#81), 12 (#68 #72), 13 (#73 #74 #143), 14 (#78), 15 (#79), 17 (measured 16 Nov, opt-in
`per_source`), 18 (#80), 19 (#70). Open: 6-8 (E6), 16 catalogue details (E4), 20 installer (E5).

## Order and acceptance

Each row is a distinct goal, with its minimum observable outcome. The dependency column gives sequencing, not permission to
skip review. Work in order within a group where practical; a blocked item does not prevent independent later work. Size is an
initial relative estimate, not a time promise.

| # | Goal | Done when | Depends on | Size |
|---|---|---|---|---|
| 1 | Personal encryption design and threat model | Document key lifecycle, supported platforms, protected assets, upgrade and recovery, failure modes and threat boundaries; review dependencies and failure cases before implementation. | Validated personal feasibility | M |
| 2 | Encrypted personal-memory database | New personal databases use SQLCipher; wrong or missing keys fail clearly; tests cover writes, reads and reopen. No production migration yet. | 1 | M |
| 3 | Local key storage | A generated key is stored in the OS key store for supported platforms; unavailable or locked stores fail clearly without plaintext fallback. | 1–2 | M |
| 4 | Recovery key as QR code | Show the actual recovery key as a scannable QR code for saving in a password manager, with copyable text fallback; disclose that either form grants access. Test QR round-trip and compatible save/restore workflow. | 1–3 | M |
| 5 | Encrypted backup and new-computer restore | Back up consistently and restore to a new location with the saved recovery key; verify records and search, reject a wrong key, and rehearse on a separate test profile. | 2–4 | M |
| 6 | Safe migration of existing personal memory | On an explicitly selected copy, migrate, verify and rehearse rollback; preserve the original until Johan approves the production switch. Report interrupted and failed migrations clearly. | 2–5 | L |
| 7 | Protect derived personal data | Personal keyword index runs in memory and is rebuilt without leaving plaintext index files; audit caches, WAL/journals, temporary files and logs against the threat model. | 1–3 | L |
| 8 | Personal encryption acceptance | Test a full restart and restore, failure and recovery paths, protected artifacts, and performance including cold startup and rebuild. Record review findings and limitations; do not claim a security audit passed without one. | 2–7 | M |
| 9 | Memory recall abstention | Evaluate a no-answer decision on held-out answerable and unanswerable prompts; record false injections and missed relevant memories before enabling it by default. | Existing recall evaluation | L |
| 10 | Recall feedback and provenance | Expose which memories informed a recall, their scope and dates, and whether no result met the threshold, without logging private text unnecessarily. | 9 for threshold status | M |
| 11 | Memory review workflow | Provide a bounded review of stale, duplicate or superseded memories with explicit user choices and reversible outcomes; preserve history. | Existing memory review | M |
| 12 | Product switches | Enable and disable Memory, Library and Code independently; disabling Memory stops hook injection immediately, with a separate explicit uninstall path. | Existing CLI and hooks | M |
| 13 | Launcher global search | Search the enabled local products from the launcher and identify result type, source and failure or empty states; respect product scope and permissions. | 12, existing engine search | M |
| 14 | Launcher model installation | Install an available model from the window with licence, download size, progress, failure and cancellation states; no hidden download. | Existing model manager | M |
| 15 | Launcher tray and mini-mode controls | Offer predictable show, hide, reopen and quit actions, with accessible keyboard and status feedback; verify lifecycle and reduced motion. | Existing launcher and mini mode | M |
| 16 | Product and model catalogue | Present installed, available and unavailable modules consistently from the engine, including edition boundaries and reasons an action is unavailable. | 12, existing status/models commands | M |
| 17 | Library page-level results | Measure grouping multiple passages by page on development queries; adopt only with a recorded quality and output-size comparison and no hidden evidence loss. | Existing Library benchmark | M |
| 18 | Documentation version vector reuse | Cache embeddings by normalized chunk text and model identity; compare two versions and verify changed text and model changes cannot reuse stale vectors. | Existing Library ingestion | L |
| 19 | Usage: measured savings view | Show observed before/after output reduction with its source and period, separately from counterfactual estimates and API-price estimates; avoid double counting. | Existing usage command | M |
| 20 | Windows installation and update path | Define and test an installer and update/rollback flow on a clean Windows profile, preserving user data and stating signing status; document uninstall behavior. | Stable launcher and engine | L |

## Boundaries

- The 90% result applies to the measured small, warm personal-memory workload. It does not validate encryption performance for
  project memory or Library; those remain separate decisions and measurements.
- The QR code and its copyable text contain the same secret. A password manager's ability to save and restore that exact format
  must be tested; scanning it as an authenticator code is not assumed. No personal encryption key exists yet.
- Personal-source ingestion, whole-vault access, changing the live database, or deleting originals is not authorized by this plan.
  The [personal-knowledge transition](PERSONAL-KNOWLEDGE-TRANSITION.md) retains its own scope and acceptance gates.
- UI work follows the project's design contract, Apple design reference, accessibility requirements and QLoop protocol. An
  independent critic is a separate acceptance requirement where applicable; do not describe it as complete without evidence.
- Feature delivery, external installation, deployment and any necessary approval follow the existing project and session rules.
  Benchmark receipts are immutable; retain failures and label incomplete coverage.
