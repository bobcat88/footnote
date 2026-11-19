# Call-graph recall: confidence tiers, unresolved-call report, existing-index overlay, miss buckets (19 November 2026)

Goal: raise recall without lowering the default tier's precision. The default tier (receiver_type, same_file, import, type_path,
module_path) is untouched: for every scored project the default links are byte-identical to the baseline build (`cmp` on the
`FOOTNOTE_CODE_RULES` tsv of the old and new binaries: click, hono, httpx, nest, rich, zustand). Lower tiers are opt-in
(`FOOTNOTE_CODE_GUESSES=1` at `code-index`), stored in their own table (`guesses`), shown only as `guessed_callers` in a lookup.

Truth: the compiler graphs of 18 November (scip-python, scip-typescript, same unchanged sources) and javac on `src/main/java`
(`JavacOracle.java`). Scored with `score_tiers.py`, `score_java.py`, and `classify_misses.py`. Rust projects (brotli, BorgInvestor)
and Spring are not on this machine and were not rescored; this change touches the parser's unresolved branch only.

## 1. Tiers, scored one by one

Python = click, httpx, rich; TypeScript = hono, nest, zustand. Recall = correct links / compiler edges to functions (constructors
and accessors left out), same denominator in every row. A single tier's recall is its own gain.

| Scope | Tier | Links | Same | Elsewhere | No edge | Not analyzed | Analyzed precision | Conflict rate | Recall | Recall delta vs default |
| Python | default | 9214 | 7047 | 5 | 1098 | 1064 | 0.865 | 0.001 | 0.455 | +0.000 |
| Python | guess_receiver_name | 860 | 802 | 0 | 44 | 14 | 0.948 | 0.000 | 0.103 | +0.103 |
| Python | default+all guesses | 10074 | 7849 | 5 | 1142 | 1078 | 0.872 | 0.001 | 0.558 | +0.103 |
| TypeScript | default | 7743 | 6571 | 1 | 466 | 705 | 0.934 | 0.000 | 0.581 | +0.000 |
| TypeScript | guess_receiver_name | 368 | 299 | 23 | 34 | 12 | 0.840 | 0.071 | 0.050 | +0.050 |
| TypeScript | default+all guesses | 8111 | 6870 | 24 | 500 | 717 | 0.929 | 0.003 | 0.629 | +0.048 |

By project:

| Scope | Tier | Links | Same | Elsewhere | No edge | Not analyzed | Analyzed precision | Conflict rate | Recall | Recall delta vs default |
| click | default | 2331 | 757 | 4 | 508 | 1062 | 0.597 | 0.005 | 0.351 | +0.000 |
| click | guess_receiver_name | 39 | 11 | 0 | 18 | 10 | 0.379 | 0.000 | 0.006 | +0.006 |
| click | default+all guesses | 2370 | 768 | 4 | 526 | 1072 | 0.592 | 0.005 | 0.357 | +0.006 |
| hono | default | 3927 | 3090 | 1 | 328 | 508 | 0.904 | 0.000 | 0.256 | +0.000 |
| hono | guess_receiver_name | 197 | 174 | 16 | 1 | 6 | 0.911 | 0.084 | 0.086 | +0.086 |
| hono | default+all guesses | 4124 | 3264 | 17 | 329 | 514 | 0.904 | 0.005 | 0.342 | +0.086 |
| httpx | default | 1486 | 898 | 0 | 588 | 0 | 0.604 | 0.000 | 0.224 | +0.000 |
| httpx | guess_receiver_name | 432 | 427 | 0 | 5 | 0 | 0.988 | 0.000 | 0.233 | +0.233 |
| httpx | default+all guesses | 1918 | 1325 | 0 | 593 | 0 | 0.691 | 0.000 | 0.457 | +0.233 |
| nest | default | 3573 | 3281 | 0 | 126 | 166 | 0.963 | 0.000 | 0.747 | +0.000 |
| nest | guess_receiver_name | 171 | 125 | 7 | 33 | 6 | 0.758 | 0.053 | 0.031 | +0.031 |
| nest | default+all guesses | 3744 | 3406 | 7 | 159 | 172 | 0.954 | 0.002 | 0.778 | +0.031 |
| rich | default | 5397 | 5392 | 1 | 2 | 2 | 0.999 | 0.000 | 0.595 | +0.000 |
| rich | guess_receiver_name | 389 | 364 | 0 | 21 | 4 | 0.945 | 0.000 | 0.085 | +0.085 |
| rich | default+all guesses | 5786 | 5756 | 1 | 23 | 6 | 0.996 | 0.000 | 0.680 | +0.085 |
| zustand | default | 243 | 200 | 0 | 12 | 31 | 0.943 | 0.000 | 0.578 | +0.000 |
| zustand | default+all guesses | 243 | 200 | 0 | 12 | 31 | 0.943 | 0.000 | 0.578 | +0.000 |

Java, javac as truth (precision, recall of all links; default tier = the base row, identical to the published 1.00 precision):

| project | default tier | + guess_receiver_name | guess_receiver_name alone |
|---|---|---|---|
| gson | 1,688 links, 1.00, 0.85 | 1,756 links, 1.00, 0.88 | 68 links at 0.93 |
| jsoup | 4,168 links, 1.00, 0.70 | 4,455 links, 1.00, 0.75 | 287 at 0.97 |
| commons-lang | 4,004 links, 1.00, 0.79 | 4,302 links, 1.00, 0.85 | 298 at 0.98 |

Kept: `guess_receiver_name` (a call on a value whose name is a project type's name, `store.save()` with type `Store`, `my_store`
with `MyStore`, when that type has the method). Precision 0.84 to 0.99 by language, conflict rate with the compiler 0.0 to 0.07
(TypeScript is the weak one), recall gain +0.10 Python, +0.05 TypeScript, +0.03 to +0.06 Java.

Failed and removed (precision too low to ship even as a labelled tier):
- `guess_member_unique` (a method name defined once in the project, receiver unknown): 0.52 Python, 0.61 TypeScript, 0.04 / 0.50 /
  0.06 on gson / jsoup / commons-lang (`get`, `add`, `size` of the JDK against one project method). Zustand: n/a.
- `guess_bare_unique` (a bare call to a name defined once in another file, not imported): 0.43 Python, 0.65 TypeScript, 0.00 on
  zustand (155 links, none right). The same rule scored 0 in 12 on Rust on 17 November.
- Not scorable against a static compiler graph: "trait or interface method with a single implementation". The compiler links the
  call to the interface method; the implementation is a dispatch edge no compiler graph holds. Not built.

## 2. Unresolved-call report

`footnote code-unresolved <file or function> [--root]` and `footnote_code {unresolved: "<file or function>"}` (MCP): every call in
that file, or inside that function, the parser did not link, with the line, the enclosing function, and the project definitions
sharing the callee's name as `path:line` (at most 5), or none when the callee lives outside the project. The agent greps those.
Always built (table `unresolved`), no flag. Test: `unresolved_calls_are_reported_and_guesses_stay_apart`. Graphs built before this
say to run `code-index` again.

## 3. Existing-index overlay

The compiler graph was already the top tier by construction: `index()` runs rust-analyzer, scip-typescript or scip-python when on
PATH, the parser only fills files the compiler did not cover (and, in a compiler file, only macro calls), so a guess can never sit
over a compiler link (guesses in compiler-covered files are dropped). New: `FOOTNOTE_CODE_SCIP=<index.scip>` reads an index that
already exists (any SCIP indexer, including scip-java) instead of running one. Opt-in, no new dependency, a bad file is an error.
Test: `an_existing_scip_index_is_the_compiler_graph_when_asked`; the subfolder test `rust_in_a_subfolder_gets_the_compiler_index`
still passes.

## 4. Miss buckets

Compiler function edges the default tier does not link, by the text before the callee (`classify_misses.py`, same six projects):

Python: 4257 missed of 7808 compiler function edges
| Cause | Missed | Share | Example |
|---|---:|---:|---|
| x.method(): value of unknown type | 2186 | 51.4% | `examples/repo/repo.py:57` ctx.obj.set_config(key, value) |
| bare name: not linked (imported, re-exported, local, alias) | 939 | 22.1% | `examples/aliases/aliases.py:93` callback=read_config, |
| import, export or declaration line | 323 | 7.6% | `src/click/__init__.py:18` from .decorators import argument as argument |
| self.method() not linked | 202 | 4.7% | `src/click/_termui_impl.py:171` return self.time_per_iteration * (self.length - self.pos) |
| decorator | 173 | 4.1% | `examples/aliases/aliases.py:101` @cli.command() |
| self.attr.method(): field of unknown type | 134 | 3.1% | `src/click/core.py:568` "command": self.command.to_info_dict(self), |
| call chain: f().method() | 120 | 2.8% | `tests/client/test_redirects.py:237` assert response.history[0].url == "https://example.org/multiple_redirects?count=20" |
| construction or keyword before the name | 64 | 1.5% | `examples/imagepipe/imagepipe.py:51` return processor |
| name not on the line (multi-line call) | 51 | 1.2% | `src/click/termui.py:982` _getchar = f |
| super call | 36 | 0.8% | `src/click/core.py:1809` info_dict = super().to_info_dict(ctx) |
| constructor: `new X()` or `X()` not linked at all | 27 | 0.6% | `src/click/core.py:1766` super().__init__(name, **kwargs) |
| constructor: linked to the class under its own name (scoring artifact, not a miss) | 2 | 0.0% | `src/click/exceptions.py:336` super().__init__(ctx.get_help(), ctx=ctx) |

TypeScript: 5712 missed of 9370 compiler function edges
| Cause | Missed | Share | Example |
|---|---:|---:|---|
| scip-typescript <constructor>/<get>/<set> reference: a `new X()` or property access, not a call | 3076 | 53.9% | `runtime-tests/bun/index.test.tsx:22` const app = new Hono() |
| x.method(): value of unknown type | 1428 | 25.0% | `runtime-tests/bun/index.test.tsx:164` stream.onAbort(() => { |
| bare name: not linked (imported, re-exported, local, alias) | 470 | 8.2% | `runtime-tests/bun/index.test.tsx:24` c.header('x-param', c.req.param('foo')) |
| self.attr.method(): field of unknown type | 222 | 3.9% | `src/hono-base.ts:414` this.router.add(method, path, [handler, r]) |
| import, export or declaration line | 211 | 3.7% | `runtime-tests/workerd/index.ts:3` import { getColorEnabledAsync } from '../../src/utils/color' |
| self.method() not linked | 135 | 2.4% | `src/router/reg-exp-router/matcher.ts:31` this.match = match |
| call chain: f().method() | 55 | 1.0% | `src/adapter/aws-lambda/handler.test.ts:250` const request = getProcessor(event).createRequest(event) |
| construction or keyword before the name | 31 | 0.5% | `src/client/client.test.ts:1033` const json = await res.json() |
| optional chain a?.method() | 29 | 0.5% | `src/jsx/dom/server.ts:30` const res = element instanceof Promise ? element : (element?.toString() ?? '') |
| other member access | 21 | 0.4% | `src/client/client.test.ts:397` .route('/api', api) |
| super call | 17 | 0.3% | `src/hono.test.ts:3830` super.route(path, app) |
| decorator | 17 | 0.3% | `packages/common/pipes/file/parse-file.pipe.ts:19` @Injectable() |

Reading it: the biggest real bucket is **a method called on a value of unknown type**: 51% of Python misses and 25% of all, 54% once
non-call references are left out, in TypeScript. In TypeScript the leading bucket is not a call at all: scip-typescript records `new X()` as a
reference to `<constructor>` and property reads as `<get>`/`<set>`, which a call graph does not hold (score_tiers.py now leaves
them out of the recall denominator; the earlier TypeScript recall figures used all function edges). The second bucket, bare names,
is mostly functions passed as values (`callback=read_config`), also not calls.

The fix for the unknown-type bucket is the receiver-name tier of section 1, which recovers 0.10 of recall in Python and 0.05 in
TypeScript at 0.95 and 0.84 precision. It cannot join the default tier: precision on those links is below the default's. The
remaining unknown-type calls (`ctx.obj.set_config(...)`: a framework object) have no type written anywhere in the project; that
is the part only the compiler index or the unresolved-call report (section 2) can serve.
