# Personal corpus: plan (encrypted, triaged, high-precision OCR)

Agreed with Johan on 17 November 2026. A plan, not authorization: each step below ends at a gate he approves. Personal-source
ingestion, whole-vault access, changing the live database and deleting originals stay unauthorized until he says so for a step
(rule of [NEXT-20-FEATURES.md](NEXT-20-FEATURES.md); [PERSONAL-KNOWLEDGE-TRANSITION.md](PERSONAL-KNOWLEDGE-TRANSITION.md) keeps
its own gates).

## Goal

Johan's Second Brain becomes searchable by Footnote as a purely personal, encrypted corpus, after a sensitivity triage, with PDFs
and pictures turned into accurate text. Personal processing by a person for himself falls under the GDPR household exemption
(Article 2(2)(c), to be checked against the text before this doc is final); the design still follows its principles because
Footnote Enterprise will need them: keep only what is needed, encrypt, make deletion real.

## Shape (Johan's design, 17 Nov)

- **Originals** (PDF, scans, photos) go to secure storage, unchanged, never edited.
- **One Markdown file per original**: the extracted text, which Johan can read, correct and enrich; this is what Footnote indexes.
- **A content hash links them**: each Markdown file names its original's hash, and the corpus database stores it, so any passage
  leads back to the exact file that produced it, even after a rename or move.
- **Where the Markdown lives depends on the triage class.** A plain `.md` file is as exposed as today's vault notes; that is fine
  for most documents. For the most sensitive classes (identity, health, others' personal data) the Markdown stays inside the
  encrypted store and is edited through Footnote.
- **Hash:** SHA-256, already computed for every indexed file by the engine. BLAKE3 is faster but adds a dependency for a gain
  that does not matter at personal scale. Johan can overrule.

## Steps and gates

| # | Step | Done when | Gate |
|---|---|---|---|
| 1 | Sensitivity triage tool | A local scan labels each file (health, money and tax, identity, legal, others' personal data, secrets) with Alias (rules + OpenAI Privacy Filter) and file metadata; the report shows classes and counts, not content; secrets (passwords, keys) are flagged for removal and never indexed | Johan approves running it on the vault |
| 2 | OCR benchmark | Character error rate on French pages with accents for local, permissively licensed engines (RapidOCR today, Tesseract 5, docTR; recent local options researched first; never a cloud OCR), on a small sample; two engines may be combined since time is not a concern; the 100-page and pixel caps of `tools/extract.py` revisited | Sample source chosen by Johan (public French scans, or pages he picks) |
| 3 | Encrypted Library corpus | A corpus database under SQLCipher with the keyword index in memory (no plaintext index files), vectors encrypted, deletion that purges text, vectors and index; recovery key and key store reused from personal memory (#65); speed and RAM measured on a corpus of the vault's size | Measured cost reviewed by Johan |
| 4 | Conversion pipeline | Original to secure storage with its hash, Markdown written per triage class, ingestion with the retrieval gains found so far: heading path (+0.044 nDCG), structure-aware chunks (+0.051), one passage per page (+0.029), hybrid search, lean output, Jina v5 nano for French (+0.10 nDCG on Alloprof; non-commercial licence, fine for personal use) | Tested on a copy of an approved subset |
| 5 | Migration of an approved subset | Johan picks the folders; originals preserved; rollback rehearsed | Johan approves the subset |
| 6 | The rest of the vault | Same pipeline, folder by folder | Johan approves each batch |

## Open

- Photos that are not documents: OCR gives nothing; describing them needs a local vision model (a separate decision).
- Where secure storage for originals lives (an encrypted archive in the state folder, or an encrypted volume Johan already uses).
- Whether the triage classes map to Enterprise's data-classification tiers (memory: access control planned early for companies).
