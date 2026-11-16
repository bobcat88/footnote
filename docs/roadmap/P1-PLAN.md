# P1 plan: launcher, options, model manager, icons

Written 15 November 2026. Backlog items 8, 9 and 11 (item 10, the Rust hook, is done). Structure: [ADR 0001](../adr/0001-repository-structure.md).

## Principle: the engine first, the window last

Every launcher action is a `footnote` command that answers JSON. The launcher only shows and calls them. Consequences: every
feature works from the command line and in tests before any window exists; a company can script deployment without the
launcher; the launcher holds no logic that could drift from the engine.

## Milestones

### M1: engine (no interface)

Done 15 November (branch `feat/m1-engine`): every command below, tested; `footnote status` on Johan's machine: 141 memories,
162 prompts given memories in the last 7 days, four agents registered, 27 ms.

| Command | What it does |
|---|---|
| `footnote status` | Products installed and on, agents registered, daemon running, models present, database sizes, last week's activity (memories saved, recalls injected) |
| `footnote config get/set` | Products on or off; options; read from `<state dir>/config.json` |
| `footnote options` | The option catalogue: for each option its effect on speed, precision, privacy and disk, its default, and whether a company policy locks it |
| `footnote models list/install/remove/use` | Model manager; install downloads pinned, hash-checked files (today `tools/provision_embeddings.py`, ported to Rust) |
| `footnote models recommend` | Detects CPU, RAM, GPU and VRAM (any vendor), reads the corpus language, recommends from the model matrix built from Footnote's own results |
| `footnote uninstall --memory` | Removes hooks and the MCP server from every agent (does not exist yet) |

**Option catalogue, first entries:**

| Option | Gain | Cost | Default |
|---|---|---|---|
| Resident daemon (`resident`) | Recall by meaning on each prompt | Models stay in RAM (about 450 MB); a background process; some companies forbid it | On since 16 Nov (recall switch passed) |
| Metrics | Timings for reviews and support | A local file (`metrics.jsonl`), no network | Off (on for Johan) |
| French/mixed model (Jina v5 nano) | +0.08 to +0.10 nDCG on French and mixed corpora | Database about 2.5x larger, indexing about 2.5x slower; latency to confirm on an idle machine | Off |
| Save request frequency (`every_turns`) | More memories kept | More agent tokens per session | 10 prompts |

**Company policy:** a machine-wide file (`%ProgramData%\footnote\policy.json` on Windows, `/etc/footnote/policy.json` elsewhere)
that forces or forbids options and products. Policy wins over the user's settings; the launcher shows a locked option with the
reason. This answers "it depends on security and regulation" without a second build.

**Live progress (Johan, 15 Nov: downloads started from the window must show, and so must any other):** every long step writes
`<state dir>/jobs/<id>.json` up to four times a second (`footnote jobs` lists them; a job whose process died shows as
interrupted). The launcher's Rust side watches that folder with file-system notifications and pushes each change to the window
through Tauri events: live for the user, whoever started the job (window, command line, agent), with or without the daemon.
When the daemon runs, it may relay the same events over its local socket for millisecond latency; never required.

### M2: design (no code)

1. T3 Code study ([pingdotgg/t3code](https://github.com/pingdotgg/t3code), MIT, React + Tailwind + Base UI + Lucide inside
   Electron): structure, layout, motion, links in chat. Structure only, no copied assets.
2. Design studio for the launcher: references, four directions, Johan picks and tunes, `DESIGN.md` locked. Screens: Overview
   (one card per product: on or off, health, this week's activity), Product (its options with their impact), Models (installed,
   recommended, disk), Activity (what memory saved and recalled: the visibility principle), Settings.
   **Every module is always listed (Johan, 16 Nov):** products and models not installed appear greyed out with their install
   action, so the user sees everything Footnote can do. Pattern taken from the ArtCraft launcher
   ([storytold/craft-launcher](https://github.com/storytold/craft-launcher), MIT or Apache-2.0, `crates/ui-egui/src/views.rs`);
   our layout and look stay our own:
   - order: pinned, then installed, then a "Not installed" header with the rest;
   - a missing module is drawn in greys (grey copy of its icon, muted text) and takes its colours back under the pointer, so
     it stays readable and clearly available;
   - a module not released yet shows "Coming soon" instead of Install (Code and Alias today); one locked by company policy
     shows the lock and its reason;
   - filters All / Installed / Updates / Not installed with counts, a grid or sortable table, "Update all";
   - a card's states: available to install, downloading (size, percent, pause, cancel), installed and up to date, update
     available (installed → released version), coming soon;
   - modules can be hidden and found again behind a "Hidden" toggle.
   - **Enterprise modules (Johan, 17 Nov):** Footnote One also lists the paid modules (Alias, sign-on and rights, company
     portfolio...) so users see what Enterprise adds. They sit in one "Footnote Enterprise" group at the end of the list, muted,
     with "Learn more" instead of Install; never a pop-up, a badge or a reminder elsewhere, so the free edition stays simple and
     does not nag. "Upgrading" means installing the Enterprise build (the free build does not contain the paid code), not a key
     that unlocks hidden features.
   - **Support link (Johan, 17 Nov):** one small "Support the project" link in the footer or About screen, to a donation page
     (Liberapay: no platform fee; Ko-fi: 5 % on its free plan; GitHub Sponsors: fee to confirm on GitHub's own page), and a
     "Footnote Enterprise" link for companies. Nothing else: no pop-up, badge or reminder. How donations are taxed in France
     (micro-entreprise or not) is for Johan's accountant; no clear official answer found.
   Engine side: `footnote status` lists only Library and Memory today; M3 adds the full catalogue (every product and model with
   its state) so the window draws from one command.
3. Icons in the same pass: one family, one icon per product (Footnote, Library, Memory, Code, Alias). SVG master, then ICO for
   Windows and PNG sizes; ICNS with macOS.

### M3: launcher

`apps/launcher`: Tauri 2, web interface (React + Tailwind, matching the study), calling `footnote` commands. Windows first, built
for Linux and macOS in CI. QLoop UI gates before acceptance.

### M4: distribution (later)

Installer for Windows (MSI or winget), signed binaries, updates. Not planned in detail.

## Decisions (Johan, 15 November: all four as recommended, decision 3 corrected)

1. **Turning Memory off:** the hook stops injecting at once (a setting, no agent restart), and a separate "Uninstall" removes
   hooks and server from the agents. Recommended: both, the switch for daily use and uninstall for good.
2. **Downloads in Rust:** one small HTTP dependency (`ureq` with rustls) so model installation needs no Python. Recommended.
3. **GPU detection, every vendor from the start** (corrected 15 Nov: Johan's GPU is an AMD Radeon RX 6800, so every GPU result
   in the model matrix is AMD, through DirectML and llama.cpp Vulkan). Windows: DXGI (adapter name, dedicated VRAM; WMI caps VRAM at 4 GB and
   reports 4 GB for this 16 GB card). Linux: `/sys/class/drm`. macOS with the launcher.
4. **Order:** M1 and M2 can run side by side (M2 needs Johan's time, M1 does not).
