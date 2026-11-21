# Footnote in Docker — Setup & Operations (Windows, Linux, macOS)

Run Footnote in a hardened Linux container when:

- **Windows blocks local builds**: Smart App Control or a managed WDAC / AppLocker policy stops `cargo build` at the first build script (`os error 4551`) and blocks an unsigned `footnote.exe`. Self-signing does not get past it, and turning Smart App Control off cannot be undone without reinstalling Windows: use the container instead of weakening the policy.
- **Several agents share one memory** (a team machine or a hub): one long-lived daemon keeps the embedding models loaded for every agent.

Like [HOW-TO.md](HOW-TO.md), this guide is written for humans and AI coding agents. The security boundary of the container, and who can reach a hub, are in [SECURITY.md → Container security boundary](SECURITY.md#container-security-boundary-dockerfile-composeyaml).

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

Each such session starts a container and loads the models again. For several agents, keep one daemon running instead (Section 3).

Limits: the container gives you the CLI and the MCP server. `footnote install` hooks (Memory recall, Lean shell compaction) call a `footnote` binary on the host, so they do not work from the container. To reach a service on the host (for example Ollama on port 11434), drop `--network none`, add `--add-host=host.docker.internal:host-gateway` and use `http://host.docker.internal:11434`; Footnote's embeddings still run in-process and do not use Ollama yet.

---

## 3. Shared Hub: One Daemon for Every Agent (`compose.yaml`)

`compose.yaml` keeps **one hardened daemon** running (`serve --mcp --daemon`) with warm embedding models, for every agent on the machine or on a hub.

**Security defaults:**
- Offline (`network_mode: none`), `read_only` rootfs with a `noexec` `/tmp` tmpfs, `cap_drop: [ALL]`, `no-new-privileges`, non-root user `10001`.
- Repositories mounted **read-only** under `/workspaces`: git trusts mounted repositories there only, and Footnote's git queries never run a repository's `core.fsmonitor` or filter drivers.
- The network profile (`footnote-net`) mounts no workspace. The daemon is bounded (`pids_limit`, `mem_limit`, rotated logs), and live fetch-on-miss is off (`FOOTNOTE_NO_LIVE_FETCH=1`).
- **Access = Docker access.** Anyone who can use the Docker engine (`docker-users` on Windows, `docker` on Linux) can read every memory in the volume. Run one stack per team or tenant. For agents on other machines, use SSH with a forced command, never the Docker TCP socket. See [SECURITY.md → Who can reach the hub](SECURITY.md#who-can-reach-the-hub-access-control).

### 3.1 Start the Daemon

`FOOTNOTE_WORKSPACES`, the host folder that holds your repositories (mounted read-only at `/workspaces`), is **required** by every Compose command, including the `net` profile. Put it once in a `.env` file next to `compose.yaml` (git ignores it). `compose.yaml` derives `FOOTNOTE_PATH_MAP` from it unless you set that yourself. Other settings are listed at the top of `compose.yaml`: `FOOTNOTE_CONTAINER_NAME`, `FOOTNOTE_STATE_VOLUME` (default `footnote-state`, the volume of the one-shot commands too), `FOOTNOTE_DB`, `FOOTNOTE_PIDS_LIMIT`, `FOOTNOTE_MEM_LIMIT`, `FOOTNOTE_UID` / `FOOTNOTE_GID`.

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

# Full isolated smoke suite (temporary volume and containers, cleaned up on exit; Linux, macOS, Git Bash):
bash scripts/docker-smoke.sh footnote:local
```

One stack per team or tenant (its own project name, container and volume):

```bash
FOOTNOTE_CONTAINER_NAME=footnote-team-a FOOTNOTE_STATE_VOLUME=footnote-state-team-a docker compose -p team-a up -d
```

### 3.2 Register Your Agents Automatically

Once the daemon runs, one command points every agent found for your user at it. It detects **Claude Code, Codex, Gemini CLI and Antigravity** and adds the `footnote-memory` MCP server, with the absolute path of `docker`:

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

- **Safe to run again:** an existing `footnote-memory` entry is kept. `-Force` / `--force` replaces it, `-Remove` / `--remove` deletes it (to revoke a machine). `-Container` / `--container` targets another stack.
- Each edited file is backed up once (`<file>.footnote-backup`) and replaced atomically. A file that its agent rewrote meanwhile is left alone. A JSON file with comments is left for you to edit.
- When the container runs, the script checks it answers `tools/list` through the registered command. Restart each agent afterwards.
- It only writes MCP entries. Memory hooks (`footnote hook`) are set up by hand in Docker mode (Section 5).

### 3.3 Register an Agent by Hand

Every agent gets the same command: `docker exec -i footnote footnote --db /state/memory.db mcp`. It is not a one-shot container: the process lives as long as the agent's session and talks to the running daemon, whose models stay loaded (`docker run --rm -i … mcp`, Section 2, is the one-shot form). Use the **absolute path of `docker`** for GUI hosts, which do not inherit the shell `PATH` (Windows: `(Get-Command docker).Source -replace '\\', '/'`; Linux / macOS: `command -v docker`), and replace `footnote` (the container) with `FOOTNOTE_CONTAINER_NAME` for a team stack.

| Agent | Register it once (user scope) |
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

Check a registration with `claude mcp get footnote-memory` (it connects and reports `✔ Connected`) or `codex mcp get footnote-memory`. Remove it with `claude mcp remove footnote-memory --scope user` or `codex mcp remove footnote-memory`.

### 3.4 Host-to-Container Path Mapping (`FOOTNOTE_PATH_MAP`)

An agent on a Windows host sends Windows paths (`footnote_code` `root`, hook `cwd`), such as `C:\Users\<user>\repos\my-app`. `FOOTNOTE_PATH_MAP` translates them with semicolon-separated `HOST_PREFIX=CONTAINER_PREFIX` pairs (`compose.yaml` derives it from `FOOTNOTE_WORKSPACES`):

```text
FOOTNOTE_PATH_MAP=C:\Users\<user>\repos=/workspaces;C:\Users\<user>\.claude=/host-claude
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

The backup holds every memory in clear text: keep it where only you (or the hub's operators) can read it. On Linux, the target folder must be writable by UID `10001` (for example `sudo install -d -m 0700 -o 10001 -g 10001 ./backups`, then mount `./backups`).

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
| **Footnote Library** (`index`, `search`, `find`, `docs add/list/refresh/prune`, `models install/use`) | **Supported** (Markdown/text/web docs) | Network only through the `net` profile (`models install`, `docs add`). Live fetch-on-miss is off in the daemon. PDF/OCR needs the Python helpers, not bundled. |
| **Footnote Code** (`code-index`, `code`, `code-map`, `code-impact`, `grep`, `stack`, `frame`, `footnote_code`) | **Supported (tree-sitter)** | Mount repositories `:ro` under `/workspaces`. Compiler indexers (`--deep`: `rust-analyzer`, `scip-typescript`, `scip-python`) are not in the image. |
| **Memory Hooks** (`footnote hook`) | **Partial** | Not installed automatically; runs through `docker exec -i` + `FOOTNOTE_PATH_MAP`. 140k / idle-expiry handoffs also need host transcripts mounted `:ro`. |
| **Usage / Gain** (`usage`, `gain --missed`) | **Partial** | Requires mounting host `~/.claude` / `~/.codex` `:ro` and setting `CLAUDE_CONFIG_DIR` / `CODEX_HOME`. |
| **Footnote Lean** (`run`, `hook lean`, `audit-prefix --probe-mcp`) | **Not supported from a Windows host** | `footnote run` executes inside the Linux container, without the host's toolchains. |
| **`footnote install` / `update` / `rollback`** | **Not applicable** | Register agents with `scripts/docker-agents.ps1` / `scripts/docker-agents.sh` (Section 3.2). Rebuild the image to upgrade, keeping the state volume. |
| **`footnote personal` (DPAPI encryption) / `apps/launcher`** | **Not supported** | Windows-native DPAPI / Tauri GUI only. |
| **`footnote models recommend`** | **Approximate** | Reads the WSL 2 / container VM RAM (`/proc/meminfo`) and cannot see Windows host GPUs. |

---

## 6. Several Projects on One Hub: Measured Limits

Measured on Windows 11 + Docker Desktop with three repositories under one `FOOTNOTE_WORKSPACES` folder and three agent sessions at once: path translation, per-project code graphs, per-project memory scopes and concurrent sessions work (a cold `docker exec` MCP session answers `tools/list` in about 0.3 s). The limits:

- **One parent folder.** Repositories outside `FOOTNOTE_WORKSPACES` are not mounted; `footnote_code` refuses them. Keep every repository under one dedicated folder.
- **The whole folder is visible** to the container (read-only), including files that are not repositories: do not point it at a Desktop or a home folder.
- **No automatic code graph.** A repository is known to `footnote_code` once a hook session started there or once `code-index` ran on it; refresh with `code-index` after changes (seconds per repository, tree-sitter only).
- **Pass the project root.** `footnote_code` looks up the code graph of the folder it is given; a subfolder of an indexed repository has none of its own.
- **One memory database.** A project `scope` labels memories; it does not isolate them. Every agent of the stack can read every project's memories (one stack per trust boundary).
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
| The agent shows `footnote-memory` as failed | The container is not running, or the GUI host cannot find `docker` | `docker compose up -d`; register the absolute path of `docker` (Section 3.3) |
| Files show as changed in the save hint or `footnote projects` while `git status` on the host is clean | CRLF checkout of a Windows repository, read by git in the container (Section 6) | Expected: only CRLF files whose stat data changed are affected |
| New memories are not found by hybrid recall | The volume moved and `memory.json` points at the old model paths | `docker exec footnote footnote models use bekko-a8m gte-small` (Section 4) |
