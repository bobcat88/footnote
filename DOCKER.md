# Footnote in Docker — Setup & Operations (Windows, Linux, macOS)

Docker support is for **one person**: your agents (Claude Code, Codex, Gemini CLI, Antigravity), their parallel sessions and MCP hosts share one memory, as they do on a host. Run Footnote in a hardened Linux container when:

- **Windows blocks local builds**: Smart App Control or a managed WDAC / AppLocker policy stops `cargo build` at the first build script (`os error 4551`) and blocks an unsigned `footnote.exe`. Self-signing does not get past it, and turning Smart App Control off cannot be undone without reinstalling Windows: use the container instead of weakening the policy.
- **You want one warm daemon** for all your agents: it keeps the embedding models loaded, and the memory hooks reach it through `docker exec`.

A stack shared between several people is not a supported mode: see [SECURITY.md → One person per container](SECURITY.md#one-person-per-container). Team memory is designed in #434.

Like [HOW-TO.md](HOW-TO.md), this guide is written for humans and AI coding agents.

---

## 1. Build the Image

Requirements: Docker Desktop (Windows / macOS, WSL 2 backend on Windows) or Docker Engine (Linux). From the repository root:

```bash
docker build -t footnote:local .
```

The image (`Dockerfile`):

- builds on Debian 13 (trixie): the static ONNX Runtime that `ort-sys` downloads fails to link on bookworm (undefined `__isoc23_strtoll`);
- runs as the non-root user `footnote` (`10001:10001`), `ENTRYPOINT ["footnote"]`, with no `curl`/`wget`;
- pins both base images by `@sha256` digest (Dependabot refreshes them every week);
- keeps everything Footnote stores (memory DB, indexes, models, keys, daemon socket) in `/state`, and reads code from `/workspace` or `/workspaces`.

Mount a **named Docker volume** on `/state`, never a Windows host folder, so `0700`/`0600` permissions, atomic `hard_link` secrets and Unix daemon sockets work (ext4 inside WSL 2).

---

## 2. One Command at a Time

Mount the project you work on at `/workspace`, read-only, and run offline with a read-only root filesystem and no capabilities (PowerShell: replace each trailing `\` with a backtick):

```bash
H="--network none --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m --cap-drop=ALL --security-opt=no-new-privileges:true"
docker run --rm -i $H -v footnote-state:/state -v "${PWD}:/workspace:ro" footnote:local status
docker run --rm -i $H -v footnote-state:/state -v "${PWD}:/workspace:ro" footnote:local code-index .
# The only commands that need the network (bridged, never --network host):
docker run --rm -i --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m --cap-drop=ALL \
  --security-opt=no-new-privileges:true -v footnote-state:/state footnote:local models install bekko-a8m gte-small
```

MCP entry for an agent on the host, one container per session (use the absolute path of `docker` for GUI hosts):

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

Each such session starts a container and loads the models again, and there are no memory hooks. For daily use, keep one daemon running instead (Section 3). To reach a service on the host (for example Ollama on port 11434), drop `--network none`, add `--add-host=host.docker.internal:host-gateway` and use `http://host.docker.internal:11434`; Footnote's embeddings still run in-process and do not use Ollama yet.

---

## 3. The Compose Daemon: One for All Your Agents (`compose.yaml`)

`compose.yaml` keeps **one hardened daemon** running (`serve --mcp --daemon`) with warm embedding models. Your agents reach it with `docker exec -i`: the MCP server for their tools, and the memory hook for the memory pack at session start, recall on each prompt, and the save request or handoff at stop.

**Security defaults:**
- Offline (`network_mode: none`), `read_only` rootfs with a `noexec` `/tmp` tmpfs, `cap_drop: [ALL]`, `no-new-privileges`, non-root user `10001`.
- Repositories mounted **read-only** under `/workspaces`: git trusts mounted repositories there only, and Footnote's git queries never run a repository's `core.fsmonitor` or filter drivers.
- The network profile (`footnote-net`) mounts no workspace. The daemon is bounded (`pids_limit`, `mem_limit`, rotated logs), and live fetch-on-miss is off (`FOOTNOTE_NO_LIVE_FETCH=1`).
- **One person per stack.** Anyone who can use the Docker engine can read, change and delete every memory in the volume: do not share a stack between people ([SECURITY.md](SECURITY.md#one-person-per-container)).

### 3.1 Start the Daemon

`FOOTNOTE_WORKSPACES`, the host folder that holds your repositories (mounted read-only at `/workspaces`), is **required** by every Compose command, including the `net` profile. Put it once in a `.env` file next to `compose.yaml` (git ignores it). `compose.yaml` derives `FOOTNOTE_PATH_MAP` from it unless you set that yourself.

`FOOTNOTE_CLAUDE_PROJECTS` (optional) is the folder of Claude Code's session files. It is mounted read-only at `/host/claude-projects` and mapped like the workspaces, so the hook can size a long session from its `transcript_path` and ask for a handoff at Stop. Without it the handoff is off; everything else works. The container can then read every Claude Code session file (it only reads the tail of the current one: token counts and a timestamp).

Other settings are listed at the top of `compose.yaml`: `FOOTNOTE_CONTAINER_NAME`, `FOOTNOTE_STATE_VOLUME` (default `footnote-state`, the volume of the one-shot commands too), `FOOTNOTE_DB`, `FOOTNOTE_PIDS_LIMIT`, `FOOTNOTE_MEM_LIMIT`, `FOOTNOTE_UID` / `FOOTNOTE_GID`.

```text
# .env (Windows example; Linux: /home/<user>/repos and /home/<user>/.claude/projects)
FOOTNOTE_WORKSPACES=C:\Users\<user>\repos
FOOTNOTE_CLAUDE_PROJECTS=C:\Users\<user>\.claude\projects
```

```bash
docker compose up -d
docker compose ps

# One-time model download, through the explicit-network profile:
docker compose --profile net run --rm footnote-net models install bekko-a8m gte-small
docker compose --profile net run --rm footnote-net models use bekko-a8m gte-small

# Make each repository known to footnote_code (and refresh it after changes):
docker exec footnote footnote code-index /workspaces/<repo>

# Full isolated smoke suite (temporary volume and containers, cleaned up on exit; Linux, macOS, Git Bash):
bash scripts/docker-smoke.sh footnote:local
```

### 3.2 Register Your Agents Automatically

Once the daemon runs, one command registers it with every agent found for your user: the `footnote-memory` MCP server and the memory hooks, with the absolute path of `docker`. It detects **Claude Code, Codex, Gemini CLI and Antigravity** (also Antigravity's T3 Code provider folders and `$GEMINI_HOME`), and writes the same hooks as `footnote install --memory` on a host:

```powershell
# Windows (Windows PowerShell 5.1 or pwsh)
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1 -DryRun   # what would change
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1
```

```bash
# Linux / macOS (JSON files need python3)
bash scripts/docker-agents.sh --dry-run
bash scripts/docker-agents.sh
```

| Agent | Detected by | MCP server | Memory hooks (events, timeouts) |
|---|---|---|---|
| Claude Code | `claude` on `PATH`, or `~/.claude` (`$CLAUDE_CONFIG_DIR`) | `claude mcp add --scope user` (its own command; `~/.claude.json` is never edited by the script) | `~/.claude/settings.json`: SessionStart 15 s, UserPromptSubmit 10 s, Stop 5 s |
| Codex | `codex` on `PATH`, or `$CODEX_HOME` / `~/.codex` | `[mcp_servers.footnote-memory]` in `config.toml` | `hooks.json`: SessionStart 15 s, UserPromptSubmit 10 s, Stop 5 s |
| Gemini CLI | `gemini` on `PATH`, or `~/.gemini/settings.json` | `mcpServers` in `~/.gemini/settings.json` | same file, `matcher: "*"`: SessionStart 15 000 ms, BeforeAgent 10 000 ms, AfterAgent 5 000 ms |
| Antigravity | `~/.gemini/config` (`antigravity`, `antigravity-cli`), `~/.t3/userdata/providers/antigravity/<id>/config`, `$GEMINI_HOME/config` | `mcpServers` in `<config>/mcp_config.json` | `<config>/hooks.json`, entry `"footnote-memory"`: SessionStart 15, PreInvocation 10, Stop 5 |

- **Safe to run again:** an existing `footnote-memory` entry is kept, and a Footnote memory hook already there (this script's or the native `footnote hook`) is not doubled. `-Force` / `--force` replaces them, `-Remove` / `--remove` deletes what the script added (to revoke a machine). `-Container` / `--container` targets another container name.
- Each edited file is backed up once (`<file>.footnote-backup`) and replaced atomically. A file that its agent rewrote meanwhile is left alone. A JSON file with comments is left for you to edit.
- When the container runs, the script checks it answers `tools/list` through the registered command, and says when Claude Code's session files are not mounted (no handoff at Stop). Restart each agent afterwards.

### 3.3 Register an Agent by Hand

Every agent gets the same two commands:

- **MCP server:** `docker exec -i footnote footnote --db /state/memory.db mcp`. It is not a one-shot container: the process lives as long as the agent's session and talks to the running daemon, whose models stay loaded (`docker run --rm -i … mcp`, Section 2, is the one-shot form).
- **Memory hook:** `docker exec -i footnote footnote hook`, on the events of the table above. It reads the agent's event on stdin; `FOOTNOTE_PATH_MAP` translates its `cwd` and `transcript_path`.

Use the **absolute path of `docker`** for GUI hosts, which do not inherit the shell `PATH` (Windows: `(Get-Command docker).Source -replace '\\', '/'`; Linux / macOS: `command -v docker`), quoted in a hook command when it has a space, and replace `footnote` (the container) with `FOOTNOTE_CONTAINER_NAME` if you renamed it.

| Agent | MCP server, registered once (user scope) |
|---|---|
| Claude Code | `claude mcp add footnote-memory --scope user -- docker exec -i footnote footnote --db /state/memory.db mcp` |
| Codex | `codex mcp add footnote-memory -- docker exec -i footnote footnote --db /state/memory.db mcp` |
| Gemini CLI | `mcpServers` entry below in `~/.gemini/settings.json` |
| Antigravity | `mcpServers` entry below in `~/.gemini/config/mcp_config.json` |
| Cursor, Windsurf, Cline and other `mcpServers` hosts | `mcpServers` entry below in the host's MCP settings file |

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

Memory hooks for Claude Code (`~/.claude/settings.json`; Codex's `hooks.json` takes the same `hooks` object):

```json
{
  "hooks": {
    "SessionStart": [{"hooks": [{"type": "command", "command": "/ABSOLUTE/PATH/TO/docker exec -i footnote footnote hook", "timeout": 15}]}],
    "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "/ABSOLUTE/PATH/TO/docker exec -i footnote footnote hook", "timeout": 10}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "/ABSOLUTE/PATH/TO/docker exec -i footnote footnote hook", "timeout": 5}]}]
  }
}
```

Check a registration with `claude mcp get footnote-memory` (it connects and reports `✔ Connected`) or `codex mcp get footnote-memory`. Remove it with `claude mcp remove footnote-memory --scope user` or `codex mcp remove footnote-memory`.

### 3.4 Hook Latency (measured)

A hook runs on every prompt, so its round trip matters. Measured on Windows 11 + Docker Desktop (WSL 2 engine) with a warm daemon, two embedding models and about 70 memories; each call is a new session whose prompt recalls memories (about 580 bytes of context added); 20 calls each, medians:

| | Hook started through `cmd` | Through Git Bash (Claude Code on Windows) |
|---|---|---|
| `docker exec` alone (no-op) | 123 ms | 211 ms |
| `UserPromptSubmit` (recall, every prompt) | **192 ms** (p90 200) | **316 ms** (p90 330) |
| `SessionStart` (memory pack) | 212 ms | 337 ms |
| `Stop` | 184 ms | 283 ms |

- The hook itself (recall through the daemon) is about 70 to 100 ms; the rest is `docker exec`. Starting a Windows program from Git Bash adds about 90 ms more, so Claude Code on Windows pays about 320 ms per prompt, above a 300 ms target; agents that start hooks without bash stay near 200 ms.
- A Stop that asks the agent to save its memories also runs `git status` for the changed-files hint: up to 3.4 s on this repository through the Docker Desktop mount (Stop's timeout is 5 s).
- Not measured on a Linux host.

### 3.5 Host-to-Container Path Mapping (`FOOTNOTE_PATH_MAP`)

An agent on a Windows host sends Windows paths (`footnote_code` `root`, hook `cwd` and `transcript_path`), such as `C:\Users\<user>\repos\my-app`. `FOOTNOTE_PATH_MAP` translates them with semicolon-separated `HOST_PREFIX=CONTAINER_PREFIX` pairs (`compose.yaml` derives it from `FOOTNOTE_WORKSPACES` and `FOOTNOTE_CLAUDE_PROJECTS`):

```text
FOOTNOTE_PATH_MAP=C:\Users\<user>\repos=/workspaces;C:\Users\<user>\.claude\projects=/host/claude-projects
```

- Matches full path-segment boundaries only (`C:\work\app` never matches `C:\work\app2`); drive paths compare case-insensitively, `\` and `/` alike.
- Longest matching prefix wins; any `..` traversal segment in the relative tail fails closed.
- Mounting a parent folder (`FOOTNOTE_WORKSPACES` → `/workspaces:ro`) keeps each repository's code graph separate.

---

## 4. Backup, Upgrade and Migration

**Backup** (Windows `pwsh`; a consistent, SHA-256-verified SQLite copy in a host folder):

```powershell
docker run --rm --network none --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m `
  --cap-drop=ALL --security-opt=no-new-privileges:true `
  -v footnote-state:/state -v "${PWD}:/backups" `
  footnote:local --db /state/memory.db backup --output /backups/memory-backup.db
```

The backup holds every memory in clear text: keep it where only you can read it. On Linux, the target folder must be writable by UID `10001` (for example `sudo install -d -m 0700 -o 10001 -g 10001 ./backups`, then mount `./backups`).

**Upgrade:** pull the repository, then `docker compose build` and `docker compose up -d`. The state volume is kept.

**A volume written by an image that ran as root** (the first Dockerfile, #415) needs its owner changed once, or the non-root image fails with `attempt to write a readonly database`:

```bash
docker run --rm -u 0 -v footnote-state:/state --entrypoint chown footnote:local -R 10001:10001 /state
```

**A volume mounted somewhere else than `/state` before:** `memory.json` lists the models by absolute path. Run `docker exec footnote footnote models use bekko-a8m gte-small` once after the move, or new memories are saved without vectors.

---

## 5. Feature Compatibility Matrix

| Product / Command | Docker status | Notes |
|---|---|---|
| **Footnote Memory** (`mcp`, `serve --mcp`, `remember`, `forget`, `recall`, `memory-review`) | **Supported** | Named volume on `/state`; the Compose daemon keeps models warm. |
| **Memory Hooks** (`footnote hook`) | **Supported** | Registered by `scripts/docker-agents.*` (Section 3.2) through `docker exec -i`, with `FOOTNOTE_PATH_MAP`. The handoff of a long Claude Code session needs `FOOTNOTE_CLAUDE_PROJECTS`. Latency: Section 3.4. |
| **Footnote Library** (`index`, `search`, `find`, `docs add/list/refresh/prune`, `models install/use`) | **Supported** (Markdown/text/web docs) | Network only through the `net` profile (`models install`, `docs add`). Live fetch-on-miss is off in the daemon. PDF/OCR needs the Python helpers, not bundled. |
| **Footnote Code** (`code-index`, `code`, `code-map`, `code-impact`, `grep`, `stack`, `frame`, `footnote_code`) | **Supported (tree-sitter)** | Mount repositories `:ro` under `/workspaces`. Compiler indexers (`--deep`: `rust-analyzer`, `scip-typescript`, `scip-python`) are not in the image. |
| **Usage / Gain** (`usage`, `gain --missed`) | **Partial** | Requires mounting host `~/.claude` / `~/.codex` `:ro` and setting `CLAUDE_CONFIG_DIR` / `CODEX_HOME`. |
| **Footnote Lean** (`run`, `hook lean`, `audit-prefix --probe-mcp`) | **Not supported from a Windows host** | `footnote run` executes inside the Linux container, without the host's toolchains. |
| **`footnote install` / `update` / `rollback`** | **Not applicable** | Register agents with `scripts/docker-agents.ps1` / `scripts/docker-agents.sh` (Section 3.2). Rebuild the image to upgrade, keeping the state volume. |
| **`footnote personal` (DPAPI encryption) / `apps/launcher`** | **Not supported** | Windows-native DPAPI / Tauri GUI only. |
| **`footnote models recommend`** | **Approximate** | Reads the WSL 2 / container VM RAM (`/proc/meminfo`) and cannot see Windows host GPUs. |

---

## 6. Several Projects in One Daemon: Measured Limits

Measured on Windows 11 + Docker Desktop with three repositories under one `FOOTNOTE_WORKSPACES` folder and three agent sessions at once: path translation, per-project code graphs, per-project memory scopes and concurrent sessions work (a cold `docker exec` MCP session answers `tools/list` in about 0.3 s). The limits:

- **One parent folder.** Repositories outside `FOOTNOTE_WORKSPACES` are not mounted; `footnote_code` refuses them. Keep every repository under one dedicated folder.
- **The whole folder is visible** to the container (read-only), including files that are not repositories: do not point it at a Desktop or a home folder.
- **No automatic code graph.** A repository is known to `footnote_code` once a hook session started there or once `code-index` ran on it; refresh with `code-index` after changes (seconds per repository, tree-sitter only).
- **Pass the project root.** `footnote_code` looks up the code graph of the folder it is given; a subfolder of an indexed repository has none of its own.
- **Scopes are labels.** A project `scope` sorts your memories; it does not hide one project's memories from an agent working on another.
- **Git state of Windows checkouts.** Git for Windows converts line endings (`core.autocrlf`); git in the container does not, so a CRLF file whose stat data changed shows as modified in the changed-files hint and in `footnote projects`. `footnote projects` runs `git status` per repository over the Docker Desktop mount (about 1 s each; a repository slower than 5 s shows `null`).
- **A hook `cwd` outside the mapped folder** is recorded as a project that does not exist (named after its full Windows path) in `footnote projects`.

---

## 7. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `cargo build` fails with `os error 4551` on Windows | Smart App Control / WDAC blocks unsigned build scripts | Use this guide; do not weaken the policy |
| `required variable FOOTNOTE_WORKSPACES is missing a value` | Every Compose command needs the workspace folder, the `net` profile too | Set it in `.env` next to `compose.yaml` (Section 3.1) |
| `attempt to write a readonly database` | The volume was written by an image that ran as root | Change its owner once (Section 4) |
| `… is not a project Footnote knows` from `footnote_code` | No hook session and no `code-index` for that repository yet, or it is outside `FOOTNOTE_WORKSPACES` | `docker exec footnote footnote code-index /workspaces/<repo>` |
| `no code graph at …` from `footnote_code` | `root` is a subfolder of the indexed repository | Pass the repository root |
| The agent shows `footnote-memory` as failed, or a hook error on each prompt | The container is not running, or the GUI host cannot find `docker` | `docker compose up -d`; register the absolute path of `docker` (Section 3.3) |
| No handoff is asked at the end of a long Claude Code session | Claude Code's session files are not mounted | Set `FOOTNOTE_CLAUDE_PROJECTS` and `docker compose up -d` (Section 3.1) |
| Files show as changed in the save hint or `footnote projects` while `git status` on the host is clean | CRLF checkout of a Windows repository, read by git in the container (Section 6) | Expected: only CRLF files whose stat data changed are affected |
| New memories are not found by hybrid recall | The volume moved and `memory.json` points at the old model paths | `docker exec footnote footnote models use bekko-a8m gte-small` (Section 4) |
