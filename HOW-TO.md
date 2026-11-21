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

### Docker (when Windows blocks local builds)

Windows **Smart App Control** or a managed **WDAC / AppLocker** policy stops `cargo build` at the first build script (`os error 4551`). Run Footnote in a Linux container instead (Docker Desktop, WSL 2 backend). Build it from the repo root:

```bash
docker build -t footnote:local .
```

Everything Footnote keeps (memory DB, indexes, models) lives in `/state`; mount a named volume there so it survives `--rm`. Mount the project you work on at `/workspace`, read-only. The image runs as a non-root user (`10001`); run it offline with a read-only root filesystem and no capabilities (PowerShell: replace each trailing `\` with a backtick):

```bash
H="--network none --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m --cap-drop=ALL --security-opt=no-new-privileges:true"
docker run --rm -i $H -v footnote-state:/state -v "${PWD}:/workspace:ro" footnote:local status
docker run --rm -i $H -v footnote-state:/state -v "${PWD}:/workspace:ro" footnote:local code-index .
# The only commands that need the network (bridged, never --network host):
docker run --rm -i --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m --cap-drop=ALL \
  --security-opt=no-new-privileges:true -v footnote-state:/state footnote:local models install bekko-a8m gte-small
```

MCP entry for an agent on the host (stdio through `docker run -i`; use the absolute path of `docker` for GUI hosts):

```json
{
  "mcpServers": {
    "footnote-memory": {
      "command": "docker",
      "args": ["run", "--rm", "-i", "--network", "none", "--read-only", "--tmpfs", "/tmp:rw,noexec,nosuid,size=64m",
               "--cap-drop=ALL", "--security-opt=no-new-privileges:true",
               "-v", "footnote-state:/state", "footnote:local", "--db", "/state/memory.db", "mcp"]
    }
  }
}
```

Each such session starts a container and loads the models again. For several agents, keep one daemon running with `compose.yaml` instead: see [Section 8](#8-docker-shared-hub-compose-path-mapping-and-limits). A `footnote-state` volume written by an earlier image that ran as root needs its owner changed once: `docker run --rm -u 0 -v footnote-state:/state --entrypoint chown footnote:local -R 10001:10001 /state`.

Limits: the container gives you the CLI and the MCP server. `footnote install` hooks (Memory recall, Lean shell compaction) call a `footnote` binary on the host, so they do not work from the container. To reach a service on the host (for example Ollama on port 11434), drop `--network none`, add `--add-host=host.docker.internal:host-gateway` and use `http://host.docker.internal:11434`; Footnote's embeddings still run in-process and do not use Ollama yet.

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
| `cargo build` fails with `An Application Control policy has blocked this file (os error 4551)` | Windows Smart App Control / WDAC blocks unsigned build scripts (and an unsigned `footnote.exe`); self-signing does not get past it | Use Docker (Section 2, *Docker*); do not weaken the policy (turning Smart App Control off cannot be undone without reinstalling Windows) |
| `install` says config file changed while updating | Agent (e.g. Claude Code) rewrote its settings file mid-write | Close the agent session and re-run `footnote install ...` |
| MCP host fails to spawn `footnote` | Non-interactive GUI host does not inherit shell `PATH` | Use the absolute path to `footnote` / `footnote.exe` (or `docker.exe` in Docker mode) in MCP config |
| `sccache` fails with `os error 206` on Windows | Long worktree paths exceed Windows `MAX_PATH` under `sccache` | Run `$env:RUSTC_WRAPPER = ''` before `cargo` commands |
| Linux state dir permission warning | Private state folder expects owner-only mode (`0700`) | `chmod 700 ~/.local/state/footnote` |
| `footnote code-index` slow on first Rust run | `rust-analyzer scip` builds the workspace on first index | Expected for compiler-grade Rust graphs; tree-sitter runs in milliseconds on non-Rust projects |
| Need strictly offline testing | `footnote find` may live-fetch up to 3 cached doc pages on a weak match | Set `FOOTNOTE_NO_LIVE_FETCH=1` |
| `scip-python` regex crash on Windows | Upstream `scip-python` path-separator bug | Footnote automatically applies an in-memory Node compatibility shim without editing the installed npm package |

---

## 8. Docker: Shared Hub (Compose), Path Mapping and Limits

Section 2 (*Docker*) builds the image and runs it one command at a time. This section keeps **one hardened daemon** running for every agent on the machine (or on a hub), with warm embedding models.

**Security defaults** (image and `compose.yaml`):
- Non-root user `footnote` (`10001:10001`), `ENTRYPOINT ["footnote"]`, no `curl`/`wget` in the runtime image, base images pinned by `@sha256` digest (Dependabot refreshes them).
- Offline (`--network none`), `--read-only` rootfs with a `noexec` `/tmp` tmpfs, `--cap-drop=ALL`, `--security-opt=no-new-privileges:true`.
- **Always use a named Docker volume** on `/state`, never a Windows host bind-mount, so `0700`/`0600` permissions, atomic `hard_link` secrets, and Unix daemon sockets work reliably on ext4 inside WSL 2.
- Mount repositories **read-only** (`:ro`) at `/workspace` or under `/workspaces`: git trusts mounted repositories there only, and Footnote's git queries never run a repository's `core.fsmonitor` or filter drivers.
- The Compose network profile (`footnote-net`) mounts no workspace. The daemon is bounded (`pids_limit`, `mem_limit`, rotated logs), and live fetch-on-miss is off (`FOOTNOTE_NO_LIVE_FETCH=1`).
- **Access = Docker access.** Anyone who can use the Docker engine (`docker-users` on Windows, `docker` on Linux) can read every memory in the volume. Run one stack per team or tenant. For agents on other machines, use SSH with a forced command, never the Docker TCP socket. See [SECURITY.md → Who can reach the hub](SECURITY.md#who-can-reach-the-hub-access-control).

### Start the Daemon (`compose.yaml`)

`FOOTNOTE_WORKSPACES`, the host folder that holds your repositories (mounted read-only at `/workspaces`), is **required** by every Compose command, including the `net` profile. Put it once in a `.env` file next to `compose.yaml` (git ignores it). `compose.yaml` derives `FOOTNOTE_PATH_MAP` from it unless you set that yourself. Other settings (`FOOTNOTE_CONTAINER_NAME`, `FOOTNOTE_STATE_VOLUME` (default `footnote-state`, the volume of the one-shot commands too), `FOOTNOTE_DB`, `FOOTNOTE_PIDS_LIMIT`, `FOOTNOTE_MEM_LIMIT`, `FOOTNOTE_UID`/`FOOTNOTE_GID`) are listed at the top of `compose.yaml`.

```text
# .env (Windows example; Linux: FOOTNOTE_WORKSPACES=/home/<user>/repos)
FOOTNOTE_WORKSPACES=C:\Users\<user>\repos
```

```bash
docker compose up -d
docker compose ps

# One-time model download, through the explicit-network profile:
docker compose --profile net run --rm footnote-net models install bekko-a8m gte-small
docker compose --profile net run --rm footnote-net models use bekko-a8m gte-small

# Make each repository known to footnote_code (and refresh it after changes):
docker exec footnote footnote code-index /workspaces/<repo>

# Full isolated smoke suite (temporary volume and containers, cleaned up on exit):
bash scripts/docker-smoke.sh footnote:local
```

### Register Your Agents Automatically

Once the daemon runs (`docker compose up -d`), one command points every agent found for your user at it. It detects **Claude Code, Codex, Gemini CLI and Antigravity** and adds the `footnote-memory` MCP server, with the absolute path of `docker`:

```powershell
# Windows (Windows PowerShell 5.1 or pwsh)
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1 -DryRun   # what would change
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1
```

```bash
# Linux / macOS (JSON files for Gemini CLI and Antigravity need python3)
bash scripts/docker-agents.sh --dry-run
bash scripts/docker-agents.sh
```

| Agent | Detected by | Written to |
|---|---|---|
| Claude Code | `claude` on `PATH`, or `~/.claude` | `claude mcp add --scope user` (its own command; `~/.claude.json` is never edited by the script) |
| Codex | `codex` on `PATH`, or `$CODEX_HOME` / `~/.codex` | `[mcp_servers.footnote-memory]` in `config.toml` |
| Gemini CLI | `gemini` on `PATH`, or `~/.gemini/settings.json` | `mcpServers` in `~/.gemini/settings.json` |
| Antigravity | `~/.gemini/config`, `~/.gemini/antigravity` or `~/.gemini/antigravity-cli` | `mcpServers` in `~/.gemini/config/mcp_config.json` |

- **Safe to run again:** an existing `footnote-memory` entry is kept. `-Force` / `--force` replaces it, `-Remove` / `--remove` deletes it (to revoke a machine). `-Container` / `--container` targets another stack (one per team or tenant).
- Each edited file is backed up once (`<file>.footnote-backup`) and replaced atomically. A file that its agent rewrote meanwhile is left alone. A JSON file with comments is left for you to edit.
- When the container runs, the script checks it answers `tools/list` through the registered command. Restart each agent afterwards.
- It only writes MCP entries. Memory hooks (`footnote hook`) are set up by hand in Docker mode (see the compatibility matrix below).

### Host MCP Configuration via Docker (Manual)

Use the **absolute path to your `docker` executable** so GUI MCP hosts (Cursor, Windsurf, Zed, Antigravity, VS Code, Cline, Claude Code, Codex) can spawn it without shell `PATH` inheritance:

- **Windows (`pwsh`):** `(Get-Command docker).Source -replace '\\', '/'`
- **Linux (`bash`):** `command -v docker`

```json
{
  "mcpServers": {
    "footnote-memory": {
      "command": "/ABSOLUTE/PATH/TO/docker",
      "args": ["exec", "-i", "footnote", "footnote", "--db", "/state/memory.db", "mcp"]
    }
  }
}
```

### Host-to-Container Path Mapping (`FOOTNOTE_PATH_MAP`)

An agent on a Windows host sends Windows paths (`footnote_code` `root`, hook `cwd`), such as `C:\Users\<user>\repos\my-app`. `FOOTNOTE_PATH_MAP` translates them with semicolon-separated `HOST_PREFIX=CONTAINER_PREFIX` pairs (`compose.yaml` derives it from `FOOTNOTE_WORKSPACES`):

```text
FOOTNOTE_PATH_MAP=C:\Users\<user>\repos=/workspaces;C:\Users\<user>\.claude=/host-claude
```

- Matches full path-segment boundaries only (`C:\work\app` never matches `C:\work\app2`); drive paths compare case-insensitively, `\` and `/` alike.
- Longest matching prefix wins; any `..` traversal segment in the relative tail fails closed.
- Mounting a parent folder (`FOOTNOTE_WORKSPACES` → `/workspaces:ro`) keeps each repository's code graph separate.

### Backup of the Named Volume

```powershell
# Windows (pwsh): a consistent, SHA-256-verified SQLite backup in a host folder
docker run --rm --network none --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m `
  --cap-drop=ALL --security-opt=no-new-privileges:true `
  -v footnote-state:/state -v "${PWD}:/backups" `
  footnote:local --db /state/memory.db backup --output /backups/memory-backup.db
```

The backup holds every memory in clear text: keep it where only you (or the hub's operators) can read it. On Linux, the target folder must be writable by UID `10001` (for example `sudo install -d -m 0700 -o 10001 -g 10001 ./backups`, then mount `./backups`).

### Docker Feature Compatibility Matrix

| Product / Command | Docker status | Notes |
|---|---|---|
| **Footnote Memory** (`mcp`, `serve --mcp`, `remember`, `forget`, `recall`, `memory-review`) | **Supported** | Named volume on `/state`; the Compose daemon keeps models warm. |
| **Footnote Library** (`index`, `search`, `find`, `docs add/list/refresh/prune`, `models install/use`) | **Supported** (Markdown/text/web docs) | Network only through the `net` profile (`models install`, `docs add`). Live fetch-on-miss is off in the daemon. PDF/OCR needs the Python helpers, not bundled. |
| **Footnote Code** (`code-index`, `code`, `code-map`, `code-impact`, `grep`, `stack`, `frame`, `footnote_code`) | **Supported (tree-sitter)** | Mount repositories `:ro` under `/workspaces`. Compiler indexers (`--deep`: `rust-analyzer`, `scip-typescript`, `scip-python`) are not in the image. |
| **Memory Hooks** (`footnote hook`) | **Partial** | Not installed automatically; runs through `docker exec -i` + `FOOTNOTE_PATH_MAP`. 140k / idle-expiry handoffs also need host transcripts mounted `:ro`. |
| **Usage / Gain** (`usage`, `gain --missed`) | **Partial** | Requires mounting host `~/.claude` / `~/.codex` `:ro` and setting `CLAUDE_CONFIG_DIR` / `CODEX_HOME`. |
| **Footnote Lean** (`run`, `hook lean`, `audit-prefix --probe-mcp`) | **Not supported from a Windows host** | `footnote run` executes inside the Linux container, without the host's toolchains. |
| **`footnote install` / `update` / `rollback`** | **Not applicable** | Register agents with `scripts/docker-agents.ps1` / `scripts/docker-agents.sh` (above). Rebuild the image to upgrade while keeping `footnote-state`. |
| **`footnote personal` (DPAPI encryption) / `apps/launcher`** | **Not supported** | Windows-native DPAPI / Tauri GUI only. |
| **`footnote models recommend`** | **Approximate** | Reads the WSL 2 / container VM RAM (`/proc/meminfo`) and cannot see Windows host GPUs. |

### Several Projects on One Hub: Measured Limits

Measured on Windows 11 + Docker Desktop with three repositories under one `FOOTNOTE_WORKSPACES` folder and three agent sessions at once: path translation, per-project code graphs, per-project memory scopes and concurrent sessions work (a cold `docker exec` MCP session answers `tools/list` in about 0.3 s). The limits:

- **One parent folder.** Repositories outside `FOOTNOTE_WORKSPACES` are not mounted; `footnote_code` refuses them. Keep every repository under one dedicated folder.
- **The whole folder is visible** to the container (read-only), including files that are not repositories: do not point it at a Desktop or a home folder.
- **No automatic code graph.** A repository is known to `footnote_code` once a hook session started there or once `code-index` ran on it; refresh with `code-index` after changes (seconds per repository, tree-sitter only).
- **Pass the project root.** `footnote_code` looks up the code graph of the folder it is given; a subfolder of an indexed repository has none of its own.
- **One memory database.** A project `scope` labels memories; it does not isolate them. Every agent of the stack can read every project's memories (one stack per trust boundary).
- **Git state of Windows checkouts.** Git for Windows converts line endings (`core.autocrlf`); git in the container does not, so a CRLF file whose stat data changed shows as modified in the changed-files hint and in `footnote projects`. `footnote projects` runs `git status` per repository over the Docker Desktop mount (about 1 s each; a repository slower than 5 s shows `null`).
- **A hook `cwd` outside the mapped folder** is recorded as a project that does not exist (named after its full Windows path) in `footnote projects`.

---

## 9. Reporting Test Findings

When opening an issue or PR fix after testing on Linux or Windows, include:

1. OS + architecture + shell (e.g. `Ubuntu 24.04 x86_64 / bash`, `Windows 11 x64 / pwsh 7.4`)
2. `footnote --version` and `git rev-parse --short HEAD`
3. Agent harness tested (Claude Code, Codex CLI, Gemini CLI, Antigravity, Cursor, etc.)
4. Exact command run, exit code, and sanitized output (never paste credentials or private paths)
