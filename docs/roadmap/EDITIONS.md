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
