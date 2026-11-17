# Editions: Footnote One (free) and Footnote Enterprise (paid)

Decided by Johan on 16 November 2026. Nothing built yet.

## The decision

Footnote has two editions, and every feature idea is filed under one of them before it is designed.

- **Free:** for one person or a small team. It must stay simple: no sign-in screen, no rights settings, no server, nothing a
  personal user has to understand or configure. It is also the minimum a company can try before buying.
- **Commercial:** for companies. Everything that needs a company directory: single sign-on, rights, hierarchy, many team indexes,
  compliance. This is the paid value on top of the free edition.

The line is "needs a company directory", not "more than one person": a small team runs the free edition.

## One engine, not a fork

A fork would mean every fix and every improvement landing twice, and the two copies drifting apart. Instead:

- **Free edition:** this repository, Apache-2.0, as today.
- **Commercial edition:** a private crate in its own repository that depends on the free engine. The company build is the same
  engine with that crate compiled in; the free build does not contain its code at all. Footnote already works this way with opt-in
  build features (`alias`, `code`).
- **The one seam:** the engine asks one question at its door, "who is asking and what may they read". The free build answers "you,
  everything on this machine"; the commercial crate answers from single sign-on and the rights register. Nothing else in the free
  engine knows the commercial edition exists.

Cost: the engine's interface to that crate must stay stable, so changing it sometimes means updating both. Far less than a fork.

## Where features go

| Free (solo and small team) | Commercial (company) |
|---|---|
| Library, Memory, Code, Alias | Single sign-on, sessions, forced revocation ([FEDERATION-DESIGN.md](../FEDERATION-DESIGN.md) section 10) |
| Launcher, model manager | Rights register connector, unit hierarchy, shares |
| Company policy file (useful to a small team too) | Router across many team indexes; promotion from team to company library |
| Local audit log | Central audit export, compliance kit for French companies, cost per 1,000 users |
| Projects list across one person's repositories | Portfolio across teams and departments, with rights |

## Open

- Licence of the commercial crate.
- Whether team-level promotion (personal memory to a small team's index) is free, with only department and company levels paid.
- Company session flow ([FEDERATION-DESIGN.md](../FEDERATION-DESIGN.md) section 10): Johan's fallback is a sign-in screen when the
  launcher opens and 4-hour sessions. Recommended: at normal expiry Footnote refuses calls ("sign in again") instead of killing
  agents, which would lose their work; agents are stopped only on a real revocation (someone leaves, an admin cuts access).

## Decisions of 16 November 2026 (evening)

- **Names:** Footnote One (free, this repository) and Footnote Enterprise (paid, private repository bobcat88/footnote-enterprise,
  which depends on this engine and holds the paid roadmap).
- **Footnote One keeps:** agent memory (personal and project), Code graph, Library, the best engine for personal use, token-saving
  measures, single-project views. It must be good on its own: it is the product people talk about.
- **Footnote Enterprise gets:** Alias (anonymisation), single sign-on and rights, encryption with company keys, federation, the
  view across every project, curated company skills, Johan's GSD layer and design room. Every model and library it uses must
  allow commercial use.
- **Alias** was copied to the Enterprise repository (builds and passes its tests there). It stays here, unchanged, until an
  Enterprise build can include it; removing it now would leave no runnable build with it.
- **Before this repository goes public:** its git history contains Alias and company designs. Pushed history is not rewritten, so
  the public repository starts as a fresh copy without them.

**Push-backs, checked 16 Nov**
1. **The best personal engine cannot ship as Footnote One's default for teams.** Jina v5 nano is CC BY-NC 4.0 (its Hugging Face
   card: "for commercial use, please contact us"). A small company running the free edition at work is commercial use. Options:
   a permissive default model with Jina as an optional install for personal, non-commercial use, or a commercial licence from Jina.
2. **Encryption is split, not all paid.** Earlier the same day Johan asked that the free edition encrypt personal data. Proposed:
   encryption of one's own data on one's own machine stays free (trust is what makes people adopt a memory tool); company key
   management, sensitive compartments and key recovery are Enterprise.
3. **A plain list of one person's projects stays free;** the graph, roll-ups and reporting across projects are Enterprise. A solo
   developer with many repositories is Footnote One's first user.

## Free edition: private by default, up to five linked devices (Johan, 16 November 2026; design only)

**What Johan asked for**
- A local account per person. What a person creates on their machine is encrypted, so no app can read it by simply opening the
  file; it opens only through Footnote, tied to that login.
- Personal data (memories, private corpora) never leaves its owner. Projects can be shared.
- Up to five devices linked together: one person with five computers, or five people with one each. Linked devices see the
  projects shared with them, never anyone's personal data, and nothing outside the group.
- Peer to peer: each device has its own identity and is a node; no central server. Whoever creates a project creates its key.

**Proposed mechanisms (to be checked and security-reviewed before any build)**
- **Encryption, not hashing.** A hash cannot be reversed, so data must be encrypted. The SQLite files would use SQLCipher (SQLite
  with encryption; rusqlite's `bundled-sqlcipher` feature builds it in, with a vendored OpenSSL variant). Licence: BSD-style, usable
  in open-source and commercial software if Zetetic's notice is shown in the app and its documentation. Checked 16 Nov (zetetic.net).
- **The login is the operating system's.** The database key is kept in the system's key store (Windows Credential Manager, macOS
  Keychain, Linux Secret Service) and unlocked by the person's own session. A separate Footnote password would be one more thing to
  forget, and a forgotten password would mean lost data. An optional passphrase can be added for shared machines.
- **What it protects against, plainly:** another user of the machine, a stolen or copied disk, and anyone opening the file with a
  database tool. It does not protect against malware running as the same person, which can ask the key store too.
- **The keyword index must be covered too.** Tantivy files hold the words of every document and Tantivy has no encryption of its
  own, but it reads and writes through a replaceable storage layer: Seshat (Element's encrypted message search, matrix-org) wraps
  Tantivy's directory to encrypt every file, so it is possible, at a read-speed cost to measure. Simplest first: personal memory is
  small, so its keyword index can live in memory only and be rebuilt at start-up. Checked 16 Nov (docs.rs seshat 4.3.0,
  `encrypted_dir.rs`).
- **Device identity and linking.** Each device makes its own key pair on install; its public key is its id. Linking two devices
  shows a short code on one to type on the other (as T3 Code and Signal do). A project has its own random key, given to each member
  device encrypted to that device's public key. Removing a device creates a new project key for the others; what the removed
  device already received cannot be taken back.
- **Peer to peer without a blockchain.** Like Bitcoin's network, each device is a node. A blockchain solves agreement between
  strangers who do not trust each other; five linked devices already trust each other, so it is not needed. Candidate library:
  iroh 1.0 (Rust, MIT or Apache-2.0, maintained by n0): a device's public key is its identity, connections go direct through home
  and office networks. **Not serverless by default:** devices first meet through a relay and find each other through n0's lookup
  servers, and fall back to the relay when a direct path fails. The free edition would point to relays it can run itself (or, on
  one network, local discovery only), never to a third party's by default. Checked 16 Nov (iroh README on docs.rs).
- **What syncs:** records, not database files. Memories and project records are already written once and corrected by a new
  record (`supersedes`), so devices exchange new records and each rebuilds its own indexes; there are no edit conflicts to merge.

**Push-backs to decide**
- **The five-seat limit cannot be enforced in open-source code:** anyone can rebuild Footnote without the check. Either it stays a
  stated limit, or the commercial edition's value is the directory features (sign-on, rights, compliance) rather than more seats.
  Recommended: the second, with the limit kept as a design default.
- **One person's own devices:** should personal memory sync between them (a "me" group, separate from project groups)? Johan's
  "one person with five computers" suggests yes.
- **Shared projects: encrypted or not.** Johan called this debatable. Sharing them between devices already requires the project
  key, so encrypting them at rest costs little more once the mechanism exists.

## Johan's review of 16 November 2026 (night), with checks

- **Code graph is in Footnote One,** per project, with agent memory (personal and project) and the plain list of one's projects.
  The graph and reporting across all projects are Enterprise.
- **Encryption rule (written before any measure):** personal memory is always encrypted in Footnote One. Project memory and Library
  corpora are encrypted too only if encrypted search keeps at least 90 % of today's speed (Johan: 90 to 95 %); otherwise they stay
  as they are, protected by the operating system's per-user folder permissions.
- **Personal data decision, 16 November:** Johan validated the SQLCipher and in-memory keyword-index approach for personal data
  after its warm recall benchmark exceeded the 90% speed gate. This is a feasibility decision, not a claim that encryption has
  shipped or passed security review. See [the benchmark](../../bench/experiments/SQLCIPHER-FEASIBILITY.md) for its scope and
  remaining work; the project memory and Library decisions require their own evidence.
- **Personal recovery requirement:** present the recovery key as a QR code so it can be saved in a password manager; the QR code
  contains the key itself and must be protected as a secret. Also provide the key as copyable text for password managers or
  devices that cannot save the QR code. Test restoration of an encrypted backup on another computer using the saved key before
  treating recovery as complete. No personal key has been created yet.
- **Licence of Footnote One: Apache-2.0 (decided by Johan, 16 Nov).** Footnote One is the useful bare-bones edition, free for
  everyone including companies; company features (sign-on, rights, Alias...) need Footnote Enterprise. Jina stays an optional model
  for personal, non-commercial use, with its licence shown at install; the default model for companies must allow commercial use.
  Background, for the record: He wants "free for personal use; for commercial use, contact us", with Jina
  as the engine. Apache-2.0 cannot say that: it lets anyone use the code commercially. Two consistent options:
  - Keep Apache-2.0 for the code; Jina (CC BY-NC 4.0) is an optional model the user accepts for non-commercial use; companies use
    a permissive model or buy Enterprise.
  - Release Footnote One under the PolyForm Noncommercial License 1.0.0 (personal, hobby, research use free; not OSI open source):
    any company use needs a paid licence, matching Jina's own terms. This is what Johan described.
- **Stay in this private repository until a v0.9 beta,** then publish. Labels "alpha"/"beta" until v1.
- **Contributions:** issues yes, outside pull requests no. GitHub can restrict pull requests to collaborators or turn them off
  (repository setting, February 2026). Automated triage of issues by an agent comes later (GitHub Actions is blocked by billing).
- **Third-party notices:** a list of every dependency and model with its licence ships with each edition, generated by
  `cargo-about` (and checked by `cargo-deny`); models and SQLCipher, if adopted, are added by hand. Footnote One uses only free
  components; Enterprise uses permissive ones by default, and any paid model or software is licensed by the client in its name.

- **CI:** GitHub Actions is out of credit (16 Nov); failing checks are not code failures until credits return. Merging goes on;
  pushed history is still never rewritten.
