# TypeScript and Python call graph precision on 15 projects

18 November 2026, issue #216. Parser (tree-sitter base layer) links scored against a fresh scip-typescript or scip-python
graph for each project, with the four classes of #215. Rust is in `CODE-RUST-PRECISION-2026-11-18.md`.

## Corpora

| Group | Projects | How chosen |
|---|---|---|
| Private, TypeScript | private-ts-5, private-ts-3, private-ts-1 `dashboard/`, private-ts-2 `desktop/`, private-ts-4 | every private TypeScript repo with a `tsconfig.json` and more than 40 TypeScript files |
| Private, Python | private-py-1 `backend/`, private-py-2, private-rust-6, private-py-3, private-ts-1 | the five largest private Python repos, excluding copies and vendored trees |
| Online, held out | TypeScript: hono a8ae5b1, nest bbbfaf1, zustand 9fed2eb. Python: click 08b803c, httpx 7cff5f7, rich 3a109cc | picked by hand before any score, shallow clones; nothing installed, so no project code ran (both indexers only type-check) |

Dropped before scoring, with the reason: ky (its `tsconfig.json` extends a package from `node_modules`, so indexing would need
`npm install`, which runs third-party scripts), immer (scip-typescript reached 4 of its 57 files), typeorm (no root
`tsconfig.json`). The rules were designed from samples of private-ts-1, private-py-1 and private-ts-3, so the Private group
is not held out; the online group is.

## Results

| Group | Language | Rule | Links | Same | Compiler links elsewhere | Analyzed, no edge | Not analyzed | Analyzed precision | Conflict rate |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| online | Python | same_file | 1452 | 1440 | 5 | 3 | 4 | 0.994 | 0.003 |
| online | Python | receiver_type | 1814 | 1711 | 0 | 84 | 19 | 0.953 | 0.000 |
| online | Python | import | 5943 | 3891 | 0 | 1011 | 1041 | 0.794 | 0.000 |
| online | Python | all | 9209 | 7042 | 5 | 1098 | 1064 | 0.865 | 0.001 |
| online | TypeScript | same_file | 2684 | 1715 | 1 | 350 | 618 | 0.830 | 0.001 |
| online | TypeScript | receiver_type | 2153 | 2030 | 0 | 98 | 25 | 0.954 | 0.000 |
| online | TypeScript | import | 2670 | 2656 | 0 | 0 | 14 | 1.000 | 0.000 |
| online | TypeScript | all | 7507 | 6401 | 1 | 448 | 657 | 0.934 | 0.000 |
| Private | Python | same_file | 6257 | 6245 | 0 | 12 | 0 | 0.998 | 0.000 |
| Private | Python | receiver_type | 2091 | 2082 | 0 | 4 | 5 | 0.998 | 0.000 |
| Private | Python | import | 3725 | 3670 | 0 | 33 | 22 | 0.991 | 0.000 |
| Private | Python | all | 12073 | 11997 | 0 | 49 | 27 | 0.996 | 0.000 |
| Private | TypeScript | same_file | 1877 | 1634 | 0 | 76 | 167 | 0.956 | 0.000 |
| Private | TypeScript | receiver_type | 210 | 210 | 0 | 0 | 0 | 1.000 | 0.000 |
| Private | TypeScript | import | 2036 | 2036 | 0 | 0 | 0 | 1.000 | 0.000 |
| Private | TypeScript | all | 4123 | 3880 | 0 | 76 | 167 | 0.981 | 0.000 |

On held-out code, 6 of 13,449 comparable links conflict with the compiler (0.04%). The "analyzed, no edge" links are compiler
gaps, checked by sample:
- **Python imports (online):** click's tests resolve `import click` to the copy installed in site-packages, so their calls point
  outside the repository; httpx's `httpx.Response(...)` goes through a re-export in `__init__` that scip-python does not link.
- **TypeScript `same_file`:** calls to functions declared inside another function (`const run = ...` in a component); 
  scip-typescript files them as locals, which Footnote does not store.

## What changed

1. **TypeScript had no parser graph at all.** tree-sitter-typescript's tags query only holds TypeScript-only items and is meant
   to be combined with the JavaScript one; Footnote used it alone, so `.ts` and `.tsx` files got no definitions and no calls.
   The parser now uses both. (On private-ts-3, before: only its Rust links.)
2. **`obj.f(x)` was read as a bare call in JavaScript and TypeScript**: the JavaScript query tags member calls on their
   arguments, so the qualifier was taken from the first argument. The call is now the arguments' parent.
3. **Receiver typing for TypeScript, JavaScript and Python** (`receiver_type`), the Rust approach of #201: method tables from
   class bodies, field types from annotations, `self.x = T()`, `this.x = new T()` and constructor parameter properties;
   receivers traced through `self`/`this`, annotated or constructed locals, annotated parameters, `await` (TypeScript
   `Promise<T>`), `Optional[T]` and `T | null`. It abstains on loop, `with`, `except` and comprehension variables, on any
   rebinding in between, and on a name assigned anywhere in a Python function before looking outside it. A class imported from
   the project's own module counts; one from a package does not.
4. **Fixes found on hono**: a call on a computed value (`new Hono().get()`, `f()?.g()`) no longer resolves through an import;
   a module's file is matched by its exact stem (`./hono` is not `hono.test.ts`); `{ getPath: () => .. }` and
   `Deno.upgradeWebSocket = () => ..` bind no name, so bare calls skip them; a name imported at the top of the file wins over a
   same-file definition; a relative TypeScript or JavaScript import resolves to the one file beside the importer (or that
   folder's `index`), with or without a `.js` extension. The last fix took held-out TypeScript imports from 55 conflicts to 0 and
   added about 1,700 links in total, mostly ESM imports written with `.js`.
5. **Product fixes found on the way**: scip-python crashed outside a git repository (it asks git for a version), so Footnote
   now passes `--project-version HEAD`; scip-typescript's object-literal members (`spawnSession0:`) and inline-type members
   (`typeLiteral10:entry_date`) now get their plain name, so `code lookup spawnSession` finds them (28% of private-ts-3's
   compiler symbols were affected).
6. **Scoring**: a compiler target of any kind counts as the same link when name, line and target file match
   (scip-typescript files `const f = () => ..` as a value).

## Rust unchanged

The new rules apply to every language, so the parser was rerun on nine Rust projects from #214 against their existing
rust-analyzer graphs, main against this branch: no correct link lost; zip 268 to 277 correct with 16 to 7 conflicts (the
top-of-file import rule), private-rust-1 one conflict fixed. A file-wide version of the import rule first hid `tocost` in lindera
(a `use super::tocost` inside `mod tests`), hence "top of the file only".

## Gain from receiver typing (Python, same projects)

private-py-1 backend 1,133 to 1,437 links, private-py-2 690 to 791, private-ts-1 8,250 to 9,039, rich 1,350 receiver links,
all at analyzed precision 0.95 to 1.00 with no conflict.

## Scripts

`code-graph/score_project.ps1` (copy, compiler run, parser run) and `code-graph/score_by_language.py` (this table).
