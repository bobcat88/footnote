# Footnote — Setup & Testing How-To (Linux & Windows)

This guide is written for **both humans and AI coding agents** (Claude Code, Codex CLI, Gemini CLI, Antigravity, Cursor, Windsurf, Zed, Cline, Hermes). Point an LLM at this file and ask it to set up or verify Footnote on Linux or Windows.

---

## Instructions for AI Agents Following This Guide

If you are an LLM setting up or testing Footnote on a user's machine:

1. **Detect OS and shell first** (`uname -s` on Linux/macOS; `$PSVersionTable` on Windows). Never mix POSIX and PowerShell syntax in one command.
2. **Preview before mutating agent configs:** always run `footnote install ... --dry-run` first and show the user which files will be edited. Footnote backs up every file it touches (`*.footnote-backup`) and never overwrites an existing `footnote` entry.
3. **Never pollute live user state during tests:** set `FOOTNOTE_STATE_DIR` to a temporary folder for every smoke test.
4. **Trust check before Rust `code-index`:** on Rust workspaces (`Cargo.toml`), `rust-analyzer scip` compiles build scripts and proc-macros. Only run `footnote code-index` on repositories the user trusts.
5. **Report concrete verification results** (pass/fail per step, binary path, OS, and any error output verbatim).

---

## 1. Prerequisites

| Requirement | Linux | Windows | Needed for |
|---|---|---|---|
| **Rust 1.92+** (`rustup`) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | `winget install Rustlang.Rustup` | Building `footnote` from source |
| **C toolchain / linker** | `sudo apt install build-essential` (or distro equivalent) | Visual Studio Build Tools (Desktop C++ workload) | Linking Rust + SQLite / tree-sitter |
| **Git** | `sudo apt install git` | `winget install Git.Git` | Cloning the repo and `git` integration tests |
| **Node.js + npm** *(optional)* | `sudo apt install nodejs npm` | `winget install OpenJS.NodeJS.LTS` | Compiler-grade Code graphs for TS/JS (`scip-typescript`) and Python (`scip-python`) |
| **Python 3.11+** *(optional)* | `python3 --version` | `python --version` | PDF text extraction & OCR only (`requirements-extract.lock`, Pillow 13.0+) — **not** needed to build, install models, or run Memory, Code, or Lean |

> **Note on optional encryption builds:** default builds need no OpenSSL or Perl. Building with `--features encryption` (Windows DPAPI + SQLCipher) additionally requires Perl (e.g. Strawberry Perl) to compile vendored OpenSSL.

---

## 2. Build & Install the Binary

### Linux (bash / zsh)

```bash
git clone https://github.com/bobcat88/footnote.git
cd footnote
cargo build --locked --release

mkdir -p "$HOME/.local/bin"
cp target/release/footnote "$HOME/.local/bin/footnote"
chmod +x "$HOME/.local/bin/footnote"

# Add to PATH if not already present:
export PATH="$HOME/.local/bin:$PATH"
# Persist in ~/.bashrc or ~/.zshrc:
#   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc

footnote --version
```

### Windows (PowerShell 7 / `pwsh`)

```powershell
git clone https://github.com/bobcat88/footnote.git
Set-Location footnote

# Tip: if sccache is configured globally and hits Windows path-length limits, clear RUSTC_WRAPPER for the build:
$env:RUSTC_WRAPPER = ''
cargo build --locked --release

$binDir = "$env:LOCALAPPDATA\Programs\footnote"
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
Copy-Item .\target\release\footnote.exe "$binDir\footnote.exe" -Force

# Add to current session PATH (and optionally persist to User PATH):
$env:PATH = "$binDir;$env:PATH"
# [Environment]::SetEnvironmentVariable('Path', "$binDir;" + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')

footnote --version
```

### Upgrading & Rollback (Zero-Downtime)

After pulling updates and running `cargo build --locked --release`:

```bash
# Linux
footnote update --from ./target/release/footnote
# Windows
footnote update --from .\target\release\footnote.exe
```

`footnote update` verifies the new binary runs, keeps the previous binary beside it (`footnote.replaced-*`), and swaps cleanly even while agents are active. Run `footnote rollback` to revert. User data lives outside the binary folder (`~/.local/state/footnote` on Linux, `%LOCALAPPDATA%\footnote` on Windows, or `$FOOTNOTE_STATE_DIR`) and is never touched by updates.

---

## 3. Install Local Embedding Models (Zero Python)

Hybrid retrieval blends Tantivy BM25 with two small local CPU models (~200 MB total, MIT-licensed, pinned revisions, SHA-256 verified on every load):

```bash
footnote models list
footnote models install bekko-a8m gte-small
footnote models use bekko-a8m gte-small
footnote models recommend
```

- `models install` is one of the only explicit network commands in Footnote.
- Lexical-only search works without downloading any models.

---

## 4. Connect Your Agent Harness

Always preview first with `--dry-run`, then apply without `--dry-run`.

### A. Footnote Memory (Persistent Cross-Session Agent Memory)

Registers `footnote-memory` MCP server, session/prompt hooks (`footnote hook`), and `/remember` across detected agents (Claude Code, Codex CLI, Gemini CLI, Antigravity `agy`, Grok, DeepSeek Harness):

```bash
footnote install --memory --dry-run
footnote install --memory
```

Optional skill suggestions (off by default): index the skills of one theme (at most 100), then switch the option on. On each prompt the hook adds at most 2 skills (name, path, one line), and only when one holds 40% of the prompt's topic words.

```bash
footnote skills index web ~/.claude/skills/transitions-dev ~/.claude/skills/impeccable
footnote config set memory.skills true
footnote skills list            # themes switched on; `footnote skills remove web` switches one off
```

### B. Footnote Lean (Shell Compaction, Token Accounting & Prefix Audit)

Installs the fast SQLite-free `PreToolUse` hook (`footnote hook lean`) so eligible shell commands (`cargo`, `git`, `pytest`, `jest/vitest`, `rg/grep`, linters, `gh ... --json`) automatically run through `footnote run` with reversible handles:

```bash
# Preview, then wire Memory + Lean + User PATH in one command:
footnote install --memory --lean --add-to-path --dry-run
footnote install --memory --lean --add-to-path
# If migrating from RTK, remove legacy RTK hooks/MCP entries in the same step:
footnote install --lean --remove-rtk
```

### C. Footnote Library (Local Notes, PDFs & Shared Docs Cache)

Index a local folder of Markdown, text, or PDF files:

```bash
# Linux
mkdir -p "$HOME/.footnote"
DB="$HOME/.footnote/knowledge.db"
footnote --db "$DB" index --root "$HOME/notes" --allow "$HOME/notes"
footnote --db "$DB" rebuild-embeddings
footnote --db "$DB" install --dry-run
footnote --db "$DB" install
```

```powershell
# Windows
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.footnote" | Out-Null
$DB = "$env:USERPROFILE\.footnote\knowledge.db"
footnote --db $DB index --root "$env:USERPROFILE\notes" --allow "$env:USERPROFILE\notes"
footnote --db $DB rebuild-embeddings
footnote --db $DB install --dry-run
footnote --db $DB install
```

Add shared library documentation once per machine (searchable from every project via `footnote find`):

```bash
footnote docs add https://docs.rs/rusqlite/latest/rusqlite/ --site rusqlite
footnote docs list
```

### D. Footnote Code (Compiler-Grade / Tree-Sitter Code Graph & Native Grep)

Inside any trusted project repository:

```bash
# Build the code graph (uses rust-analyzer / scip-typescript / scip-python if installed, else built-in tree-sitter):
footnote code-index .

# Check whether compiler-grade SCIP indexers are available (never installs without --yes):
footnote code-index --deep --no-install .

# Optional compiler-grade indexers:
#   Rust:       rustup component add rust-analyzer
#   TS / JS:    npm install -g @sourcegraph/scip-typescript
#   Python:     npm install -g @sourcegraph/scip-python

# Query symbols, architecture map, impact radius, and symbol-annotated native ripgrep:
footnote code-map .
footnote grep "enclosing_symbols" . --glob "*.rs"
```

### E. Manual MCP Entry (Cursor, Windsurf, Zed, VS Code, Cline, Hermes)

Use **absolute paths** (GUI MCP hosts rarely inherit your interactive shell `PATH`):

```json
{
  "mcpServers": {
    "footnote-memory": {
      "command": "/ABSOLUTE/PATH/TO/footnote",
      "args": ["--db", "/ABSOLUTE/PATH/TO/memory.db", "mcp"]
    }
  }
}
```

Default `memory.db` path:
- **Linux:** `$HOME/.local/state/footnote/memory.db`
- **Windows:** `%LOCALAPPDATA%\footnote\memory.db` (use forward slashes in JSON/TOML, e.g. `C:/Users/<user>/AppData/Local/footnote/memory.db`).

### F. Shared Background Daemon (Optional)

- **Windows:** `footnote --db <DB> install` registers a logon task via `schtasks` automatically.
- **Linux:** `footnote --db <DB> mcp` works out of the box in-process with zero daemon needed. To share one warm daemon across multiple agents on Linux, start:
  ```bash
  footnote --db "$HOME/.local/state/footnote/memory.db" serve --mcp --daemon
  ```
  (Or wrap that command in a `~/.config/systemd/user/footnote-memory.service` unit.)

---

## 5. Cross-Platform Harness Smoke Test (Linux & Windows)

Run this checklist to validate a clean install or test a branch on Linux and Windows without touching real user state.

### Step 1 — Automated Rust Test Suite

```bash
# Linux (from repo root)
cargo test --locked --test run --test lean_d2 --test grep --test audit_prefix --test usage --test mcp
cargo clippy --locked --workspace --all-targets -- -D warnings
```

```powershell
# Windows (from repo root)
$env:RUSTC_WRAPPER = ''
cargo test --locked --test run --test lean_d2 --test grep --test audit_prefix --test usage --test mcp
cargo clippy --locked --workspace --all-targets -- -D warnings
```

### Step 2 — Live Binary Smoke Test in Isolated `FOOTNOTE_STATE_DIR`

#### Linux (`bash`)

```bash
TMP_STATE="$(mktemp -d)"
export FOOTNOTE_STATE_DIR="$TMP_STATE"

# 1. Status & options
footnote status
footnote options | head -c 300; echo

# 2. Code graph + native ripgrep with enclosing symbol annotations
footnote code-index .
footnote grep "enclosing_symbols" . --glob "*.rs"

# 3. Lean shell compaction + handle storage + retrieval
footnote run -- git log -n 20
footnote gain --days 7

# 4. Lean PreToolUse hook rewrite (eligible command rewritten; piped command untouched)
echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cargo test --workspace"}}' | footnote hook lean
echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rg foo src | head -n 5"}}' | footnote hook lean

# 5. Turn-1 fixed prefix & MCP schema audit (read-only)
footnote audit-prefix --root . --probe-mcp

# 6. Uncovered shell command discovery from local Claude/Codex transcripts (prefixes & counts only)
footnote gain --missed --days 7

# Cleanup sandbox
rm -rf "$TMP_STATE"
unset FOOTNOTE_STATE_DIR
```

#### Windows (`pwsh`)

```powershell
$tmpState = Join-Path ([IO.Path]::GetTempPath()) ("footnote-smoke-" + [guid]::NewGuid().ToString("N").Substring(0,8))
New-Item -ItemType Directory -Force -Path $tmpState | Out-Null
$env:FOOTNOTE_STATE_DIR = $tmpState

try {
    # 1. Status
    footnote status

    # 2. Code graph + native ripgrep with enclosing symbol annotations
    footnote code-index .
    footnote grep "enclosing_symbols" . --glob "*.rs"

    # 3. Lean shell compaction + savings ledger
    footnote run -- git log -n 20
    footnote gain --days 7

    # 4. Lean PreToolUse hook rewrite
    '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cargo test --workspace"}}' | footnote hook lean
    '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rg foo src | head -n 5"}}' | footnote hook lean

    # 5. Turn-1 fixed prefix & MCP schema audit (read-only)
    footnote audit-prefix --root . --probe-mcp

    # 6. Uncovered shell command discovery (prefixes & counts only)
    footnote gain --missed --days 7
} finally {
    Remove-Item Env:FOOTNOTE_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $tmpState -ErrorAction SilentlyContinue
}
```

### Step 3 — Verify MCP `tools/list` Byte Budget (`<= 5,000 B`)

Verify the MCP server responds to JSON-RPC `initialize` + `tools/list` cleanly on your OS:

```bash
# Linux (or Git Bash)
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' \
  | footnote --db "$HOME/.local/state/footnote/memory.db" serve --mcp
```

Expected: two JSON-RPC `2.0` responses, with 8 tools in `tools/list` totaling ~4.8 KB (`<= 5,000` bytes).

---

## 6. Uninstall

Remove hooks and MCP registrations cleanly without deleting your databases or settings:

```bash
footnote uninstall --memory --lean --dry-run
footnote uninstall --memory --lean
```

To also remove all local data, indexes, and downloaded models, delete the state folder (`~/.local/state/footnote` on Linux, `%LOCALAPPDATA%\footnote` on Windows) and the binary.

---

## 7. Platform Gotchas & Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `install` says config file changed while updating | Agent (e.g. Claude Code) rewrote its settings file mid-write | Close the agent session and re-run `footnote install ...` |
| MCP host fails to spawn `footnote` | Non-interactive GUI host does not inherit shell `PATH` | Use the absolute path to `footnote` / `footnote.exe` in MCP config |
| `sccache` fails with `os error 206` on Windows | Long worktree paths exceed Windows `MAX_PATH` under `sccache` | Run `$env:RUSTC_WRAPPER = ''` before `cargo` commands |
| Linux state dir permission warning | Private state folder expects owner-only mode (`0700`) | `chmod 700 ~/.local/state/footnote` |
| `footnote code-index` slow on first Rust run | `rust-analyzer scip` builds the workspace on first index | Expected for compiler-grade Rust graphs; tree-sitter runs in milliseconds on non-Rust projects |
| Need strictly offline testing | `footnote find` may live-fetch up to 3 cached doc pages on a weak match | Set `FOOTNOTE_NO_LIVE_FETCH=1` |
| `scip-python` regex crash on Windows | Upstream `scip-python` path-separator bug | Footnote automatically applies an in-memory Node compatibility shim without editing the installed npm package |

---

## 8. Reporting Test Findings

When opening an issue or PR fix after testing on Linux or Windows, include:

1. OS + architecture + shell (e.g. `Ubuntu 24.04 x86_64 / bash`, `Windows 11 x64 / pwsh 7.4`)
2. `footnote --version` and `git rev-parse --short HEAD`
3. Agent harness tested (Claude Code, Codex CLI, Gemini CLI, Antigravity, Cursor, etc.)
4. Exact command run, exit code, and sanitized output (never paste credentials or private paths)
