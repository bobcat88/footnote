# Editions: free and commercial

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
  with encryption; rusqlite can build it in).
- **The login is the operating system's.** The database key is kept in the system's key store (Windows Credential Manager, macOS
  Keychain, Linux Secret Service) and unlocked by the person's own session. A separate Footnote password would be one more thing to
  forget, and a forgotten password would mean lost data. An optional passphrase can be added for shared machines.
- **What it protects against, plainly:** another user of the machine, a stolen or copied disk, and anyone opening the file with a
  database tool. It does not protect against malware running as the same person, which can ask the key store too.
- **The keyword index must be covered too.** Tantivy files hold the words of every document and cannot be encrypted. Personal
  memory is small, so its keyword index can be rebuilt in memory at start-up and never written to disk. Library corpora of public
  documentation need no encryption; private corpora rely on full-disk encryption until something better is measured.
- **Device identity and linking.** Each device makes its own key pair on install; its public key is its id. Linking two devices
  shows a short code on one to type on the other (as T3 Code and Signal do). A project has its own random key, given to each member
  device encrypted to that device's public key. Removing a device creates a new project key for the others; what the removed
  device already received cannot be taken back.
- **Peer to peer without a blockchain.** Like Bitcoin's network, each device is a node and there is no server. A blockchain solves
  agreement between strangers who do not trust each other; five linked devices already trust each other, so it is not needed.
  Candidate library: iroh (Rust, peer-to-peer connections between devices identified by their public keys, works across home and
  office networks), to be reviewed.
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
