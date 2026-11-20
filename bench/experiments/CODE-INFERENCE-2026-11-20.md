# Call-graph recall: local type inference for Python and TypeScript receivers (20 November 2026)

Goal (#299): resolve `x.method()` when `x` is a local of a known type, without lowering the default tier's precision.

Finding: most of the requested rules were already in `receiver_dyn` / `declared_dyn` (`crates/code/src/ts.rs`), from #290 and earlier:
(a) `x = Foo(...)` and `const x = new Foo(...)`, (b) annotated parameters and annotated locals (`x: Foo`, `x: Foo = ...`),
(d) `self` / `this`, plus fields, method chains and `await`, all followed through the member lookup including inherited methods.
A variable that a later statement of its block may rebind (`if c: x = Other()`), a loop, `with`, `except` or comprehension
variable, an unannotated parameter and a union of classes already stayed unresolved. Only one rule was missing:

(c) `x = make()` where `make` is the one project function of that name with a declared return type (`def make() -> Foo`,
`function make(): Foo`, `Promise<Foo>` under `await`): `x` has that type. Not used when the name has two definitions (overloads
can return different types), is a method, is imported from outside the project, or declares no return type. A Python call of a
project class (`Foo()`) keeps its constructor meaning. New link rule name `return_type`, in the default tier (precision below).

Method: same six fixtures, truth and scorer as `CODE-OVERLOADS-2026-11-20.md` (compiler graphs rebuilt from the same sources,
parser-only run with `FOOTNOTE_CODE_RULES` and `FOOTNOTE_CODE_GUESSES=1`, `score_tiers.py`). Before = main at 2072047, after = this change.

| Scope | Links | Same | No edge | Analyzed precision | Recall |
|---|---:|---:|---:|---:|---:|
| Python default | 9284 -> 9298 | 7070 -> 7084 | 1110 -> 1110 | 0.864 -> 0.864 | 0.458 -> 0.460 |
| TypeScript default | 8249 -> 8249 | 7073 -> 7073 | 470 -> 470 | 0.938 -> 0.938 | 0.611 -> 0.611 |
| Python default + guesses | 10163 -> 10164 | 7880 -> 7881 | 1160 -> 1160 | 0.871 -> 0.871 | 0.561 -> 0.562 |
| TypeScript default + guesses | 8672 -> 8672 | 7375 -> 7375 | 551 -> 551 | 0.927 -> 0.927 | 0.659 -> 0.659 |
| rich default | 5407 -> 5421 | 5402 -> 5416 | 2 -> 2 | 0.999 -> 0.999 | 0.597 -> 0.600 |
| click, httpx, hono, nest, zustand default | unchanged | | | unchanged | unchanged |

All 14 new Python links are in rich and all 14 are the compiler's own edge (0 wrong, precision change 0.000 on every language:
the 0.5-point rule holds, so the rule stays in the default tier). TypeScript: the 75 new `return_type` links in nest and the 6 in
platform-fastify are in spec files the compiler graph does not cover, so they are not scored (unverified, not shown to be wrong);
hono and zustand gain none.

Why the gain is small: a recall run of the remaining misses (httpx sample) is mostly property reads on `self.x` / `url.host`
(not calls, counted in the compiler's references) and values whose type comes from a parameter without annotation, a loop or an
external library; none of those can be inferred locally without guessing. What this does not show: Java, Rust, private-ts-1 and
Spring are not rescored (not touched); inference through generics, unions and `Optional` narrowing is still left unresolved.

Tests: `a_local_takes_the_declared_return_type_of_the_one_function_it_calls` (return type, inheritance through the declared
type, an async `Promise<T>` under `await`, no annotation, two definitions, a variable reassigned in an `if`, constructor, annotation
wins). The existing `python_and_typescript_method_calls_link_through_the_receiver_type` covers rules (a), (b) and (d), including
loop and comprehension variables and an unannotated parameter staying unlinked.
