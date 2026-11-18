---
type: reference
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Footnote Flow Working Integration
date_maj: 2026-11-17
---
# Footnote Flow: first working integration

Up: [[AI-Agents/projects/footnote-framework/FOOTNOTE-FLOW-DESIGN|Integration design]] · Related: [[AI-Agents/projects/footnote-framework/STATUS|Status]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## What actually ran

The integration loop ran against a pinned copy of the real Footnote executable, not retrieval mocks. An explicit FOOTNOTE_STATE_DIR and database paths kept all product writes in build/integrated-loop-v1. No live Memory database, model settings, global agent configuration or D:/Projets source changed.

The workload is a controlled Python interval-merging fixture, not a real-project effectiveness benchmark. A seeded project decision preserves touching half-open intervals as separate billing segments. A Library contract adds validation and no-mutation requirements. The initial code violates the boundary rule.

1. Footnote Memory stored the scoped seed decision. Library indexed the allowlisted contract. Code indexed the actual Python module through the installed SCIP indexer.
2. integration/footnote.py queried Code, Memory and Library and assembled a bounded context receipt with file hashes, project identity, retrieval provenance and the pinned executable hash.
3. One fresh working agent read that receipt and the selected Python skill, repaired the code, wrote public tests and proposed a lesson. There was no model coordinator.
4. The parent ran independent checks: five applicable checks passed; one follow-up-only check was skipped. The parent reviewed the lesson's incremental validation/no-mutation content, checked artifact hashes, recorded its rationale and admitted it to the isolated Memory database as record 2.
5. Preparing against the edited code detected the stale Code index and refused to continue. After explicit reindexing, a new context receipt included the learned Memory record.
6. A second fresh working agent, without the previous conversation or lesson file, added an opt-in join_touching flag. Its receipt identifies the newly recalled Memory record, existing decision, Code and Library references it used. Fourteen public tests and all six independent follow-up checks passed.

This establishes live product interoperability, a checked change, reviewed isolated admission and cross-session retrieval. It does not prove that the learned memory caused success: the code, original decision and Library contract overlap. An ablation and a real maintenance sequence are still required to establish practical benefit.

## Concrete boundary checks

- Real edited Code source was refused until reindexing.
- A contradictory memory in an unrelated project was added only to the isolated test profile. Project-scoped retrieval excluded it.
- Changing the Library source after indexing caused context preparation to refuse the stale passage. The original fixture document was restored afterward.
- Five adapter unit tests cover source-hash changes, stale/wrong-project graphs, foreign memory, unlisted/stale documentation, response budget and removal of unfiltered related-context snippets.
- The combined framework, telemetry and integration selection passes 66 tests. The product-level checks above are separate real-CLI evidence, not mocked-unit results.

The context-size self-field originally omitted its own serialization overhead. It was corrected and tested; the delivered historical receipts are preserved unchanged, with their actual file sizes and the discrepancy noted in result.json. No efficiency claim depends on those byte counts.

## Source and repeatability

integration/footnote.py is a thin standard-library bridge. It calls Footnote through RTK, pins the executable, restricts commands/database locations, enforces this proof's explicit scope and documentation allowlist, and refuses stale or oversized context instead of silently calling it complete. It does not implement another search engine.

integration/proof.py supplies reproducible setup, context preparation, explicit reindexing and reviewed admission. It runs a fixed parent-owned check suite, binds the candidate's artifact hashes, refuses failed verification and prevents replaying an already-recorded admission. It assumes a trusted host and trusted controlled source; it is not an authenticated service, hostile-code sandbox or crash-safe production admission transaction. A crash between Memory write and receipt persistence needs reconciliation rather than a blind retry. Footnote's own memory deduplication is not a complete recovery protocol.

From the project root, choose a new destination:

```powershell
rtk proxy python -m integration.proof setup build/new-integrated-proof --binary D:/Projets/BorgKnowledge/target/release/footnote.exe
rtk proxy python -m integration.proof context build/new-integrated-proof --session 1
```

Give one fresh agent the context receipt, owned workspace and applicable skill. Do not give it parent-only checks or permission to write Memory. After its implementation and candidate lesson have been inspected:

```powershell
rtk proxy python -m integration.proof admit build/new-integrated-proof --rationale 'State the actual host review decision here'
rtk proxy python -m integration.proof reindex build/new-integrated-proof
rtk proxy python -m integration.proof context build/new-integrated-proof --session 2
```

Give a different fresh agent the second receipt and follow-up objective, then run the common external checks:

```powershell
rtk proxy python -m integration.check_intervals build/new-integrated-proof/workspace --followup
rtk proxy python -m unittest integration.test_footnote -v
```

The installed scip-python dependency was reused; nothing was downloaded or installed. A second disposable profile exercised setup, context, admission and reindexing from the helper commands using a copied verified candidate. That checks script repeatability, not another independent model outcome.

## Remaining work

This is an experimental external bridge into the actual family products, not a shipped Rust Flow module. The next production work is to integrate these responsibilities at Footnote's composition boundary and reuse existing Usage for run attribution. This proof does not wire the full governor lifecycle, authenticated actors, interruption-safe admission, personal-store encryption, automatic corpus/version policy, daemon routing or a live token predictor.

Automatic Memory admission is not enabled. The parent explicitly reviewed and saved only the controlled fixture lesson, in an isolated database. Existing live Memory permissions and review policy remain unchanged. The connector currently accepts only this explicit project, excludes global memories unless a future contract permits them, and does not forward the live related-context payload without sufficient scope metadata.

Before claiming improved output or efficiency, run the equivalent Footnote-backed baseline, skills and integrated Flow on independent real maintenance sequences. Include failed work, human effort and full phase/agent usage. No additional coordinator should be introduced by default. The old parser calibration remains overhead evidence for a different execution path.

## Durable evidence

- evidence/integrated-loop-v1/result.json and file-hashes.json: product identity, outcomes and limitations.
- context-session-1.json and context-session-2.json: actual Footnote responses and supplied evidence, copied unchanged into that evidence folder.
- admission-session-1.json and live-boundary-checks.json: host review/check binding, actual stored Memory receipt and real stale/scope checks.
- agent-usage.json: the two explicitly identified native sessions and reconciled provider response usage. These are not billed cost estimates or complete host-phase timings.
- intervals.py, test_intervals.py, lesson.json and followup-receipt.json: final candidate and its traceable context claims. First-session artifacts were subsequently changed by the follow-up; their original hashes remain in the admission record and their native session evidence is identified.
- evidence/integrated-session-1-checks.txt, integrated-session-2-checks.txt, footnote-context-tests.txt and integrated-loop-tests.txt.

The pinned executable hash, rather than a guessed source commit, identifies the product build that ran. The live repository was advancing independently. Source inspection used graph generation 2026-11-17T18:25:58Z and exact source fallback; this is not a complete product audit.
