# Personal corpus: plan (encrypted, triaged, high-precision OCR)

Agreed 17 November 2026. A design plan only: each step ends at an explicit user-approved gate. Personal-source ingestion, whole-vault access, changing the live database, and deleting originals remain disallowed without explicit approval for that step (rule of [NEXT-20-FEATURES.md](NEXT-20-FEATURES.md)).

## Goal

A personal note corpus becomes searchable by Footnote as a purely personal, encrypted corpus, after a sensitivity triage, with PDFs and pictures turned into accurate text; the design follows GDPR household/privacy minimization principles: keep only what is needed, encrypt, make deletion real.

## Shape (design, 17 Nov)

- **Originals** (PDF, scans, photos) go to secure storage, unchanged, never edited.
- **One Markdown file per original**: the extracted text, which the user can read, correct and enrich; this is what Footnote indexes.
- **A content hash links them**: each Markdown file names its original's hash, and the corpus database stores it, so any passage leads back to the exact file that produced it, even after a rename or move.
- **Where the Markdown lives depends on the triage class.** A plain `.md` file is suitable for most documents. For the most sensitive classes (identity, health, others' personal data) the Markdown stays inside the encrypted store and is edited through Footnote.
- **Hash:** SHA-256, already computed for every indexed file by the engine. BLAKE3 is faster but adds a dependency for a gain that does not matter at personal scale unless overridden later.

## Steps and gates

| # | Step | Done when | Gate |
|---|---|---|---|
| 1 | Sensitivity triage tool | A local scan labels each file (health, money and tax, identity, legal, others' personal data, secrets) with Alias and file metadata; the report shows classes and counts, not content; secrets (passwords, keys) are flagged for removal and never indexed | Explicit user approval before running on a personal folder |
| 2 | OCR benchmark | Character error rate on French pages with accents for local, permissively licensed engines (RapidOCR today, Tesseract 5, docTR; recent local options researched first; never a cloud OCR), on a small sample; two engines may be combined; the 100-page and pixel caps of `tools/extract.py` revisited | Sample source selected by user (public French scans or chosen fixtures) |
| 3 | Encrypted Library corpus | A corpus database under SQLCipher with the keyword index in memory (no plaintext index files), vectors encrypted, deletion that purges text, vectors and index; recovery key and key store reused from personal memory (#65); speed and RAM measured on a representative corpus | Measured cost reviewed before promotion |
| 4 | Conversion pipeline | Original to secure storage with its hash, Markdown written per triage class, ingestion with the retrieval gains found so far: heading path (+0.044 nDCG), structure-aware chunks (+0.051), one passage per page (+0.029), hybrid search, lean output, Jina v5 nano for French (+0.10 nDCG on Alloprof; non-commercial licence) | Tested on a copy of an approved subset |
| 5 | Migration of an approved subset | User picks the folders; originals preserved; rollback rehearsed | Explicit user approval per subset |
| 6 | Remaining folders | Same pipeline, folder by folder | Explicit user approval per batch |

## Open

- Photos that are not documents: OCR gives nothing; describing them needs a local vision model (a separate decision).
- Where secure storage for originals lives (an encrypted archive in the state folder, or an encrypted user volume).
- Whether the triage classes map to Enterprise's data-classification tiers (access control planned for organizations).
