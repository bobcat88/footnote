# Federated knowledge: many small indexes, owned by the teams that know them

14 November 2026. Status: **validated in the lab on 14 November 2026 (section 6, results): viable if the router shares keyword statistics across indexes. Not built yet.** Decision owner: Johan.

## 1. The question

Footnote runs today as one SQLite file per corpus and one per person's agent memory, on one machine. Can the same design serve a company, with many teams, millions of documents, permissions and shared knowledge? Or does it need a central database server, like most AI knowledge tools?

## 2. How companies actually hold knowledge

Knowledge belongs to the people who produce and use it. Finance knows its procedures, the platform team knows its systems, legal knows its contracts. Each department is responsible for the quality of its knowledge, and the best of it should be able to rise from a team to the whole company. A central team curating everything does not scale and does not know the content.

This design follows that: **each team owns its index, and knowledge is promoted upward by the people who can vouch for it.**

## 3. Options considered

| Option | What it is | Why not, or why |
|---|---|---|
| A. One central database with vectors (for example Postgres with pgvector, or a vector database beside it) | Every document of the company in one database; access rules as filters on each query | The common AI-tool answer. Database-built-in text search is in the slow class we measured (SQLite FTS5: 420 ms at 1 million documents, against 3 ms for Tantivy), so it would lose the quality and speed lead that comes from Tantivy plus our fusion. Every query must apply permission filters correctly, so one bug leaks documents. It needs a server, so the local product and the company product diverge. |
| B. A search cluster (OpenSearch, Elasticsearch, a vector cluster) | A distributed search service | Strong at very large scale, but heavy to run, a second system to secure, and the same filter-leak risk as A. Kept as the fallback in section 7. |
| **C. A federation of small indexes (proposed)** | One self-contained index per team, space or repository, in today's format; a router queries only the indexes a person may read and merges the answers | Detailed below. |

## 4. The proposed design

```
 Personal (today)                Company (later), same file format
 memory.db  (my agent memory)    Finance   Platform   Legal   ...   one index per team, space or repository,
 kb.db      (my documents)          \        |        /             owned by that team
                                     router (per person)
                     queries only the indexes this person may read, merges with the same fusion
```

- **Index:** exactly what Footnote builds now: SQLite (source of truth for chunks, revisions, hashes), Tantivy (keyword index), int8 vectors from the two models. Each index stays in the size range where we measured it to be fast and accurate.
- **Router:** for each question, picks the indexes the person may read, queries them in parallel and merges the results with Footnote's normalized fusion. Citations stay exact: every hit still names its file, line, hash and freshness, plus the index it came from.
- **Permissions:** by index, not by filter. A document outside a person's indexes is never searched, so it cannot leak through a ranking bug. Fine-grained rules inside one index (a confidential folder) become a separate index, or an access field per document as a second layer.
- **Writes:** one writer per index (Footnote already runs one resident process per index), so SQLite's single-writer rule is respected. Many indexes can be written at the same time.
- **Scale:** grows by adding indexes, not by growing one database. The one measured limit, exact vector search up to about 50,000 entries per index, is lifted by a small vector-index file per index (an HNSW file next to the SQLite file, still serverless), only for indexes that outgrow it.
- **Operations:** backing up an index is copying a file; keeping data in a country means placing its files there; a read-only replica for another office is a file copy. A solo developer and a company run the same format.

## 5. Knowledge promotion: from a person to the company

```
 personal memory  ──propose──▶  team index  ──validate──▶  department library  ──publish──▶  company library
 (private, mine)               (team reviews)             (department owns quality)         (cited, company-wide)
```

- A memory stays private in its person's `memory.db` until that person proposes it.
- A proposal lands in the team's index as a draft record; a team member validates it with its evidence (the decision, the runbook, the commit). Footnote already has this lifecycle for structured records: `proposal`, `validated`, `superseded`, with pinned evidence hashes (`crates/search/src/records.rs`; Beads `bk-doj` continues it).
- Once validated, it is a cited entry in the department's library. Superseding it later keeps the history, as memory corrections already do.
- Each department is responsible for the quality of its own library: it decides what is promoted, and stale entries show as stale because their evidence changed.

This is where Library and Memory become one product: what agents learn in daily work turns into team and company knowledge, with a person accountable at every step.

## 5b. Sensitivity compartments (far roadmap, enterprise grade)

Separate indexes also make sensitivity a boundary between files instead of a filter inside one database (Johan, 14 November 2026). Sensitive knowledge (HR, health, legal, security) lives in its own indexes, each carrying a sensitivity level:

- **Stronger protection per compartment:** its own encryption key, its own storage location or machine, no extraction or answering through a non-local model, shorter retention if required.
- **Simpler audit:** every index already has its own tamper-evident log, so "who read health data, and when" is that one log.
- **Agent rights mirror the user's rights:** an agent acting for a person is given exactly the indexes that person may read, at their sensitivity level, never more. The agent never holds standing access of its own.

Footnote already has a small version of this: memory extraction keeps "sensitive projects" off non-local models (`docs/AGENT-MEMORY-DESIGN.md`, decision 4).

Mirroring a user's rights onto their agents (directory groups, single sign-on, delegation, revocation) is complex and enterprise grade. **It is far down the roadmap.** Nothing is built for it now beyond keeping the index format ready for it (section 8).

## 6. Validation test (written before any result)

**Data:** mix-fr-en (3,039 test questions, 17,053 chunks), the largest corpus with receipts. Single-index baseline: the existing shipped-binary receipt (nDCG@10 0.489, query p50 27.8 ms, p95 44.6 ms).

**Splits:**
1. Random: chunks assigned to N indexes by a hash of the document id. Worst case for merging, since every index is small and mixed.
2. Realistic: by source collection where the corpus has one (French and English parts), then by topic clusters of the document vectors, N indexes.

N = 4, 16 and 48. Every index is queried (the router's permission step is not under test here; it only reduces the set).

**Method (amended 14 November 2026, before any result):** the test runs in the Python lab (`bench/experiments/federation.py`) with the lab configuration of the shipped pair, and the stored document vectors, so nothing is re-encoded. The baseline is the same lab code run as one index, not the Rust receipt, so only the split differs. Each index computes its own keyword statistics (word rarity, average length): that is the effect under test. Vector scores are exact cosines and their per-model normalisation uses totals each index can return cheaply (count, sum, sum of squares), so the vector side equals the single index by design. Speed: Python timings cannot be compared with the Rust binary's 89 ms, so criterion 2 becomes: per query, the slowest index's work plus the merge, at p95, is at most twice the single index's p95 in the same lab code.

**Federation is viable if, at N = 16, for both splits:**
1. **Quality:** the paired bootstrap 95 % interval (10,000 resamples) of the nDCG@10 difference, federated minus single index, has a lower bound of -0.01 or higher.
2. **Speed:** query p95 with parallel fan-out is at most twice the single index's (89 ms).
3. **Correctness:** a unit test shows a router never returns a hit from an index outside the allowed set.

N = 4 and 48 are reported, not judged, to show the trend. If quality fails, the first fix to test is calibrating the merge (global term statistics shared across indexes for the keyword scores). If it still fails, see section 7.

### Results (14 November 2026)

Lab, mix-fr-en test split, 3,039 questions, 11,462 documents (the lab indexes whole documents, so its one-index score is 0.5009, not the binary's 0.489). Raw: `bench/work/federation-mix.json`, `bench/work/federation-mix-shared.json`.

| Split | Indexes | Each index scores alone: nDCG@10, difference [95 % CI] | With statistics shared by the router | Speed (slowest index + merge, p95) |
|---|---|---|---|---|
| random | 4 | 0.4994, -0.0015 [-0.0034, +0.0005] | 0.5009, 0.0 | 1.23 to 1.24× one index |
| random | **16** | 0.4971, **-0.0037 [-0.0064, -0.0011]** | 0.5009, 0.0 | 0.84 to 0.92× |
| random | 48 | 0.4895, -0.0113 [-0.0147, -0.0080] | 0.5009, 0.0 | 0.76 to 0.77× |
| topic | 4 | 0.4732, -0.0277 [-0.0343, -0.0212] | 0.5009, 0.0 | 1.29 to 1.34× |
| topic | **16** | 0.4795, **-0.0214 [-0.0280, -0.0148]** | 0.5009, 0.0 | 0.83 to 0.91× |
| topic | 48 | 0.4763, -0.0246 [-0.0312, -0.0179] | 0.5009, 0.0 | 0.74 to 0.75× |

**Verdict against the rules:**
- **Indexes scoring alone: fails.** At 16 indexes the random split passes (lower bound -0.0064) but the topic split fails (-0.0280). When an index holds one subject, its own words look common there, so its keyword scores come out lower and the router merges scores on different scales. Realistic team indexes are topic-shaped, so this matters.
- **With the first fix, the router sharing statistics: passes every criterion.** Each searched index reports its document count, total length and, per query word, how many of its documents contain it. The router sums them and every index scores with the same statistics. Rankings are then identical to one index over the same documents (difference 0.0 at every size, both splits), as the formula guarantees. Speed stays within 1.34× and is faster than one index from 16 indexes on. The router never returns a document outside the allowed indexes (`test_federation.py`).
- Only counts of the indexes a person may search are shared, so the fix leaks nothing outside their permissions. The timing above does not include gathering the counts; a router can cache them per index and refresh them when an index changes, so a query needs no extra round trip.

**Decision:** federation is viable, on one condition written into the design: **the router must share keyword statistics across the indexes it searches.** The vector side already uses shared totals. The fallback in section 7 is not needed for quality.

## 7. If the test fails

Keep the federation for permissions and ownership, and put a shared search service (option B) behind the same router for the largest customers only. The local product and the file format do not change.

## 8. What to change now (small, no behavior change)

1. Give every index a stable identifier stored in the database, so a router can name the index each hit came from.
2. Reserve an access field per document (unused for now), for the second permission layer.
3. Reserve a sensitivity level per index (unused for now), for the compartments in section 5b.
4. Keep `memory.db` and corpus indexes in the same format, as today.

Nothing else is built until the test in section 6 has run.

## 9. Open questions

- How a router learns which indexes a person may read: answered by section 10 (single sign-on plus the company's rights register).
- How far Footnote Code fits the same model: one index per repository is natural (see `docs/CODE-GRAPH-DESIGN.md`).
- Whether promotion needs one validator or two for some departments (legal, security).

## 10. Rights from company single sign-on (decided 16 November 2026, not built)

Commercial edition only ([roadmap/EDITIONS.md](roadmap/EDITIONS.md)); the free edition never shows a sign-in.

Johan's company already signs people in through single sign-on (SSO), and its developers already give an agent the same rights as
the person logged in, by passing that person's JWT. Footnote follows the same model; what it adds is how those rights live inside
an agent session, and for how long.

**Who sees what**
- A manager sees everything in their unit and the units below it, and nothing above unless someone above shares it. A share is
  explicit, dated, revocable and logged.
- The company's rights register (its directory) is the only source of truth. Footnote never keeps its own copy of who may read what.
- An agent gets exactly its person's rights, never more, and holds no standing access of its own. A sub-agent inherits its parent
  session and cannot extend it.

**Three gates, all required; any failure is a refusal, and every refusal is logged**
1. The app is one the company allows: proved by a client certificate the company issued (mutual TLS), not by a name it claims.
2. The person is signed in: an SSO token whose signature checks against the company's published keys, with the right issuer and
   audience, and not expired.
3. The person's rights, read from the register for that token, cover the index asked for.

Gates 1 and 2 replace a permanent API key per employee: such keys get copied into scripts, never expire, and keep working after
someone leaves.

**The session**
- The launcher signs the person in (the standard browser flow, OpenID Connect with PKCE). The agent's Footnote process receives the
  token and turns it into one in-memory session: person id, email, units, the indexes they may read, and an expiry. Nothing of it
  is written to disk. The refresh token, if any, stays with the launcher in the operating system's key store; the agent process
  never sees it.
- **Expiry:** the earlier of the token's own expiry and a maximum set by the company policy file (the policy file that already
  locks other options). Default 1 hour, the usual lifetime of an SSO access token; the company may set it between 5 minutes and
  8 hours (one working day). A sensitive compartment (section 5b) may set a shorter one for its own indexes.
- Checked 16 Nov (Microsoft Entra documentation): an Entra access token lives 60 to 90 minutes by default; administrators can set
  10 minutes to one day, and Microsoft recommends its Conditional Access "sign-in frequency" to force people to sign in again.
  Entra's Continuous Access Evaluation revokes in near real time, but only for resources built for it (Exchange, Teams, SharePoint
  at first), and gives those clients tokens of up to 28 hours. Footnote must therefore enforce its own maximum, not trust the token's.
- Every call checks the expiry first. An expired session is refused with "sign in again"; it is never extended silently. When the
  launcher refreshes the token, the rights are read again from the register, so someone who leaves or changes unit loses access
  within one expiry at most.
- Every answer is logged with the person's id and the indexes searched, in the tamper-evident audit log Footnote already keeps.

**In Rust:** JWT checking is a known crate (`jsonwebtoken`, to be security-reviewed before use) against the company's published
keys, cached and re-fetched when they rotate. Nothing else needs a new dependency.

**Open:** which claim carries the units (SSO groups or a register lookup per person), decided with the company's identity team;
whether some indexes need the person present (a fresh sign-in) rather than a session started earlier.
