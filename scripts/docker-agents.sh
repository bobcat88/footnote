#!/usr/bin/env bash
# Register the Footnote Docker daemon with every agent found for this user: MCP server and memory hooks (#392).
#
# Detects Claude Code, Codex, Gemini CLI and Antigravity (also its T3 Code provider folders and $GEMINI_HOME), and points
# each of them at the long-lived Compose daemon of this user:
#     MCP server:   <absolute path of docker> exec -i <container> footnote --db <db> mcp
#     memory hooks: <absolute path of docker> exec -i <container> footnote hook
# The hooks are the ones `footnote install --memory` writes on a host (same files, events and timeouts).
# - An existing entry is left as it is; a Footnote memory hook already there is not doubled. --force replaces it,
#   --remove deletes what this script added.
# - Claude Code's MCP server goes through `claude mcp add --scope user` (~/.claude.json is never edited here); its hooks
#   live in ~/.claude/settings.json. Every edited file is backed up once (<file>.footnote-backup) and replaced atomically;
#   a file that the agent rewrote meanwhile is not overwritten; a JSON file with comments is left to you. JSON needs python3.
# - Nothing goes over the network. --dry-run shows what would change and writes nothing.
# - Anyone who can use this Docker engine can read every memory in the container: see SECURITY.md.
#
# Usage: scripts/docker-agents.sh [--container NAME] [--db PATH] [--name NAME] [--home DIR] [--dry-run] [--force] [--remove]
set -euo pipefail

CONTAINER="${FOOTNOTE_CONTAINER_NAME:-footnote}"
DB=/state/memory.db
NAME=footnote-memory
HOME_DIR="${HOME:-}"
DRY_RUN=0
FORCE=0
REMOVE=0

usage() { sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; }
while [ $# -gt 0 ]; do
  case "$1" in
    --container) CONTAINER="${2:?--container needs a value}"; shift 2 ;;
    --db) DB="${2:?--db needs a value}"; shift 2 ;;
    --name) NAME="${2:?--name needs a value}"; shift 2 ;;
    --home) HOME_DIR="${2:?--home needs a value}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --force) FORCE=1; shift ;;
    --remove) REMOVE=1; shift ;;
    -h | --help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

die() { echo "docker-agents: $*" >&2; exit 1; }
# Values end up in JSON, TOML and command lines: accept only what Docker and Footnote accept, nothing to escape.
[[ "${CONTAINER}" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$ ]] || die "invalid --container '${CONTAINER}'"
[[ "${DB}" =~ ^/[A-Za-z0-9._/-]+$ ]] || die "--db must be an absolute container path such as /state/memory.db"
[[ "/${DB}/" != *"/../"* && "/${DB}/" != *"/./"* ]] || die "--db must not contain . or .. segments"
[[ "${NAME}" =~ ^[A-Za-z0-9_-]{1,64}$ ]] || die "invalid --name '${NAME}'"
[ "${FORCE}${REMOVE}" != "11" ] || die "--force and --remove cannot be combined"
[ -d "${HOME_DIR}" ] || die "home folder not found: '${HOME_DIR}'"

DOCKER="$(command -v docker || true)"
[ -n "${DOCKER}" ] || [ "${REMOVE}" = 1 ] || die "docker is not on PATH: install Docker Engine (or Docker Desktop) first"
if command -v cygpath >/dev/null 2>&1; then
  # Git Bash: keep MSYS from rewriting container paths (/state/...) passed to Windows programs, and register a
  # Windows path with forward slashes.
  export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'
  if [ -n "${DOCKER}" ] && [ -f "${DOCKER}.exe" ]; then DOCKER="${DOCKER}.exe"; fi
  if [ -n "${DOCKER}" ]; then DOCKER="$(cygpath -m "${DOCKER}")"; fi
fi
# GUI agents do not inherit the shell PATH: register the absolute path.
DOCKER="${DOCKER:-docker}"
case "${DOCKER}" in *'"'* | *$'\n'* | *$'\r'* | *\\*) die "unsupported characters in the docker path: ${DOCKER}" ;; esac
MCP_ARGS=(exec -i "${CONTAINER}" footnote --db "${DB}" mcp)
# Agents run a hook command through a shell: quote the path only when it has a space.
case "${DOCKER}" in *' '*) HOOK_EXE="\"${DOCKER}\"" ;; *) HOOK_EXE="${DOCKER}" ;; esac
HOOK_COMMAND="${HOOK_EXE} exec -i ${CONTAINER} footnote hook"

RESULTS=()
result() { # agent target status
  local status="$3"
  if [ "${DRY_RUN}" = 1 ]; then
    case "${status}" in registered* | replaced* | removed*) status="${status} (dry run)" ;; esac
  fi
  RESULTS+=("$1|$2|${status}")
}

# (modified time, length): notices an agent that rewrites its file while this script works on it.
file_stamp() {
  if [ -f "$1" ]; then
    stat -c '%Y.%s' "$1" 2>/dev/null || stat -f '%m.%z' "$1"
  fi
}

replace_file() { # path tmp stamp
  local path="$1" tmp="$2" stamp="$3"
  if [ "$(file_stamp "${path}")" != "${stamp}" ]; then
    rm -f -- "${tmp}"
    die "${path} changed while it was being updated (the agent is probably running): close it and run this again"
  fi
  if [ -f "${path}" ] && [ ! -e "${path}.footnote-backup" ]; then cp -p -- "${path}" "${path}.footnote-backup"; fi
  mv -f -- "${tmp}" "${path}"
}

# One JSON edit (python3): mode `mcp` (mcpServers.<NAME>), `hooks` (Claude-format or Gemini event hooks) or `agy`
# (Antigravity's named entry). Prints the status.
json_edit() { # agent path mode [format]
  local agent="$1" path="$2" mode="$3" format="${4:-claude}" status
  # A real interpreter, not the Microsoft Store stub that only prints a hint.
  if ! python3 -c 'import json' >/dev/null 2>&1; then
    result "${agent}" "${path}" "skipped: python3 is needed to edit JSON; add the ${mode} entry yourself (DOCKER.md)"
    return
  fi
  status="$(FOOTNOTE_AGENTS_PATH="${path}" FOOTNOTE_AGENTS_MODE="${mode}" FOOTNOTE_AGENTS_FORMAT="${format}" \
    FOOTNOTE_AGENTS_NAME="${NAME}" FOOTNOTE_AGENTS_DOCKER="${DOCKER}" FOOTNOTE_AGENTS_HOOK="${HOOK_COMMAND}" \
    FOOTNOTE_AGENTS_DRY="${DRY_RUN}" FOOTNOTE_AGENTS_FORCE="${FORCE}" FOOTNOTE_AGENTS_REMOVE="${REMOVE}" \
    python3 - "${MCP_ARGS[@]}" <<'PY'
import json, os, re, sys, tempfile

env = os.environ
path, mode, fmt, name = env["FOOTNOTE_AGENTS_PATH"], env["FOOTNOTE_AGENTS_MODE"], env["FOOTNOTE_AGENTS_FORMAT"], env["FOOTNOTE_AGENTS_NAME"]
docker, hook = env["FOOTNOTE_AGENTS_DOCKER"], env["FOOTNOTE_AGENTS_HOOK"]
dry, force, remove = (env[k] == "1" for k in ("FOOTNOTE_AGENTS_DRY", "FOOTNOTE_AGENTS_FORCE", "FOOTNOTE_AGENTS_REMOVE"))
mcp_args = sys.argv[1:]
CLAUDE_EVENTS = {"SessionStart": 15, "UserPromptSubmit": 10, "Stop": 5}
GEMINI_EVENTS = {"SessionStart": 15000, "BeforeAgent": 10000, "AfterAgent": 5000}
DOCKER_HOOK = re.compile(r" exec -i [A-Za-z0-9][A-Za-z0-9_.-]* footnote hook$")

def stamp():
    try:
        st = os.stat(path)
        return (st.st_mtime_ns, st.st_size)
    except FileNotFoundError:
        return None

def memory_hook(command):
    # This script's `docker exec ... footnote hook`, the native `footnote hook`, or the Python hook it replaced.
    c = command.strip()
    if "memory_hook.py" in c or DOCKER_HOOK.search(c):
        return True
    if not c.endswith(" hook"):
        return False
    exe = c[:-5].strip().strip("'\"").replace("\\", "/")
    return exe.rsplit("/", 1)[-1] in ("footnote", "footnote.exe")

def has_comment(text):
    # A `//` or `/*` comment outside a JSON string. A glob inside a string, such as a permission "Read(src/**)" in
    # Claude Code's settings.json, is not a comment.
    in_string = escaped = False
    for i, ch in enumerate(text):
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
        elif ch == '"':
            in_string = True
        elif ch == "/" and text[i + 1 : i + 2] in ("/", "*"):
            return True
    return False

def done(status, root=None):
    if root is not None and not dry:
        folder = os.path.dirname(path) or "."
        os.makedirs(folder, exist_ok=True)
        fd, tmp = tempfile.mkstemp(dir=folder, prefix=".footnote-", suffix=".tmp")
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(json.dumps(root, indent=2, ensure_ascii=False) + "\n")
        if stamp() != before:
            os.unlink(tmp)
            sys.exit(f"{path} changed while it was being updated (the agent is probably running): close it and run this again")
        if before and not os.path.exists(path + ".footnote-backup"):
            with open(path, "rb") as src, open(path + ".footnote-backup", "wb") as dst:
                dst.write(src.read())
        if before:
            os.chmod(tmp, os.stat(path).st_mode & 0o777)
        os.replace(tmp, path)
    print(status)
    sys.exit(0)

before = stamp()
raw = open(path, encoding="utf-8").read() if before else ""
if raw.strip():
    # Rewriting would drop comments: leave such a file to the user.
    if has_comment(raw):
        done(f"skipped: the file has comments; add the {mode} entry yourself (DOCKER.md)")
    try:
        root = json.loads(raw)
    except ValueError as e:
        sys.exit(f"{path} is not valid JSON: {e}")
    if not isinstance(root, dict):
        sys.exit(f"{path} does not hold a JSON object")
else:
    root = {}

if mode == "mcp":
    servers = root.get("mcpServers")
    if servers is not None and not isinstance(servers, dict):
        sys.exit(f"{path}: mcpServers is not an object")
    present = servers is not None and name in servers
    if remove:
        if not present:
            done("mcp: not registered")
        del servers[name]
        done("removed (mcp)", root)
    if present and not force:
        done("mcp: already registered (--force replaces it)")
    if servers is None:
        servers = root["mcpServers"] = {}
    servers[name] = {"command": docker, "args": mcp_args}
    done(("replaced" if present else "registered") + " (mcp)", root)

if mode == "agy":
    handler = lambda t: [{"type": "command", "command": hook, "timeout": t}]
    desired = {"SessionStart": handler(15), "PreInvocation": handler(10), "Stop": handler(5)}
    current = root.get(name)
    if remove:
        dumped = json.dumps(current) if current is not None else ""
        if " exec -i " not in dumped or "footnote hook" not in dumped:
            done("hooks: not registered")
        del root[name]
        done("removed (hooks)", root)
    if current == desired:
        done("hooks: already registered")
    if current is not None and not force:
        done(f'hooks: a "{name}" entry is already there (--force replaces it)')
    root[name] = desired
    done(("replaced" if current is not None else "registered") + " (hooks: SessionStart, PreInvocation, Stop)", root)

# mode == "hooks"
events = GEMINI_EVENTS if fmt == "gemini" else CLAUDE_EVENTS
hooks = root.get("hooks")
if hooks is not None and not isinstance(hooks, dict):
    sys.exit(f"{path}: hooks is not an object")
if hooks is None:
    if remove:
        done("hooks: not registered")
    hooks = root["hooks"] = {}
groups_of = lambda ev: [g for g in (hooks.get(ev) or []) if isinstance(g, dict)]
other = sum(
    1 for ev in events for g in groups_of(ev) for h in (g.get("hooks") or [])
    if isinstance(h, dict) and memory_hook(str(h.get("command", ""))) and h.get("command") != hook
)
if other and not force and not remove:
    done("hooks: another Footnote memory hook is registered (--force replaces it)")
changed = added = False
for ev, timeout in events.items():
    kept_groups, found = [], False
    for g in groups_of(ev):
        inner = [h for h in (g.get("hooks") or []) if isinstance(h, dict)]
        kept = []
        for h in inner:
            cmd = str(h.get("command", ""))
            if memory_hook(cmd):
                if remove:
                    if DOCKER_HOOK.search(cmd.strip()):
                        changed = True
                        continue
                elif cmd != hook:
                    h["command"] = hook
                    changed = True
                found = True
            kept.append(h)
        if inner and not kept:
            continue  # every hook of this group was ours: drop the group
        g["hooks"] = kept
        kept_groups.append(g)
    if not found and not remove:
        entry = {"type": "command", "command": hook, "timeout": timeout}
        kept_groups.append({"matcher": "*", "hooks": [entry]} if fmt == "gemini" else {"hooks": [entry]})
        changed = added = True
    if kept_groups:
        hooks[ev] = kept_groups
    else:
        hooks.pop(ev, None)
if not changed:
    done("hooks: not registered" if remove else "hooks: already registered")
status = "removed" if remove else ("registered" if added and not other else "replaced")
done(f"{status} (hooks: {', '.join(events)})", root)
PY
)" || die "${agent}: ${path} was not changed"
  result "${agent}" "${path}" "${status}"
}

# `[mcp_servers.<NAME>]` in Codex's config.toml (with its subtables, such as `.env`).
codex_server() { # path
  local path="$1" stamp present=0 status tmp
  stamp="$(file_stamp "${path}")"
  if [ -f "${path}" ] && grep -Eq "^[[:space:]]*\[mcp_servers\.(${NAME}|\"${NAME}\")\][[:space:]]*$" "${path}"; then
    present=1
  fi
  if [ "${REMOVE}" = 1 ] && [ "${present}" = 0 ]; then result codex "${path}" "mcp: not registered"; return; fi
  if [ "${present}" = 1 ] && [ "${FORCE}" = 0 ] && [ "${REMOVE}" = 0 ]; then
    result codex "${path}" "mcp: already registered (--force replaces it)"
    return
  fi
  if [ "${REMOVE}" = 1 ]; then status=removed; elif [ "${present}" = 1 ]; then status=replaced; else status=registered; fi
  if [ "${DRY_RUN}" = 0 ]; then
    mkdir -p -- "$(dirname -- "${path}")"
    tmp="$(mktemp "${path}.footnote-XXXXXX")"
    if [ -f "${path}" ]; then
      awk -v name="${NAME}" '
        { t = $0; sub(/^[ \t]+/, "", t); sub(/[ \t\r]+$/, "", t) }
        t == "[mcp_servers." name "]" || t == "[mcp_servers.\"" name "\"]" { skip = 1; next }
        skip && t ~ /^\[/ && index(t, "[mcp_servers." name ".") != 1 && index(t, "[mcp_servers.\"" name "\".") != 1 { skip = 0 }
        !skip { print }
      ' "${path}" | awk '{ lines[NR] = $0 } END { n = NR; while (n > 0 && lines[n] ~ /^[ \t\r]*$/) n--; for (i = 1; i <= n; i++) print lines[i] }' >"${tmp}"
      chmod --reference="${path}" "${tmp}" 2>/dev/null || true
    fi
    if [ "${REMOVE}" = 0 ]; then
      local quoted="" a
      for a in "${MCP_ARGS[@]}"; do quoted="${quoted:+${quoted}, }\"${a}\""; done
      {
        [ -s "${tmp}" ] && echo
        echo "[mcp_servers.${NAME}]"
        echo "command = \"${DOCKER}\""
        echo "args = [${quoted}]"
      } >>"${tmp}"
    fi
    replace_file "${path}" "${tmp}" "${stamp}"
  fi
  result codex "${path}" "${status} (mcp)"
}

# Claude Code's MCP server: its own command keeps ~/.claude.json consistent (Claude Code rewrites that file while it runs).
claude_server() {
  local target="claude mcp (user scope)" present=0 status
  if ! command -v claude >/dev/null 2>&1; then
    result claude-code "${target}" "skipped: the claude command is not on PATH; run: claude mcp add ${NAME} --scope user -- ${DOCKER} ${MCP_ARGS[*]}"
    return
  fi
  if (cd "${HOME_DIR}" && claude mcp get "${NAME}" >/dev/null 2>&1); then present=1; fi
  if [ "${REMOVE}" = 1 ] && [ "${present}" = 0 ]; then result claude-code "${target}" "mcp: not registered"; return; fi
  if [ "${present}" = 1 ] && [ "${FORCE}" = 0 ] && [ "${REMOVE}" = 0 ]; then
    result claude-code "${target}" "mcp: already registered (--force replaces it)"
    return
  fi
  if [ "${REMOVE}" = 1 ]; then status=removed; elif [ "${present}" = 1 ]; then status=replaced; else status=registered; fi
  if [ "${DRY_RUN}" = 0 ]; then
    if [ "${present}" = 1 ]; then
      (cd "${HOME_DIR}" && claude mcp remove --scope user "${NAME}" >/dev/null) || die "claude mcp remove --scope user ${NAME} failed"
    fi
    if [ "${REMOVE}" = 0 ]; then
      (cd "${HOME_DIR}" && claude mcp add --scope user "${NAME}" -- "${DOCKER}" "${MCP_ARGS[@]}" >/dev/null) ||
        die "claude mcp add --scope user ${NAME} failed"
    fi
  fi
  result claude-code "${target}" "${status} (mcp)"
}

# Antigravity config folders, found as `footnote install` finds them: ~/.gemini/config, T3 Code's isolated provider
# folders (~/.t3/userdata/providers/antigravity/<id>/config), and $GEMINI_HOME/config when it is inside the home folder.
# Prints "label|folder" lines.
antigravity_dirs() {
  local g="${HOME_DIR}/.gemini" seen="" d cfg
  add() { case "|${seen}|" in *"|$2|"*) ;; *) seen="${seen}|$2"; echo "$1|$2" ;; esac; }
  if [ -e "${g}/config" ] || [ -e "${g}/antigravity" ] || [ -e "${g}/antigravity-cli" ]; then add antigravity "${g}/config"; fi
  if [ -d "${HOME_DIR}/.t3/userdata/providers/antigravity" ]; then
    for d in "${HOME_DIR}/.t3/userdata/providers/antigravity"/*/; do
      [ -d "${d}" ] || continue
      d="${d%/}"
      cfg="${d}/config"
      if [ -d "${cfg}" ] || [ -d "${d}/antigravity-acp" ]; then add "antigravity (t3-code)" "${cfg}"; fi
    done
  fi
  if [ -n "${GEMINI_HOME:-}" ] && [ -d "${GEMINI_HOME}" ]; then
    case "${GEMINI_HOME%/}/" in "${HOME_DIR%/}/"*) add "antigravity (GEMINI_HOME)" "${GEMINI_HOME%/}/config" ;; esac
  fi
}

CLAUDE_DIR="${CLAUDE_CONFIG_DIR:-${HOME_DIR}/.claude}"
CODEX_DIR="${CODEX_HOME:-${HOME_DIR}/.codex}"
GEMINI_SETTINGS="${HOME_DIR}/.gemini/settings.json"
found=0
claude_hooks=0
if command -v claude >/dev/null 2>&1 || [ -e "${CLAUDE_DIR}" ]; then
  found=1
  claude_server
  json_edit claude-code "${CLAUDE_DIR}/settings.json" hooks claude
  claude_hooks=1
fi
if command -v codex >/dev/null 2>&1 || [ -e "${CODEX_DIR}" ]; then
  found=1
  codex_server "${CODEX_DIR}/config.toml"
  json_edit codex "${CODEX_DIR}/hooks.json" hooks claude
fi
if command -v gemini >/dev/null 2>&1 || [ -f "${GEMINI_SETTINGS}" ]; then
  found=1
  json_edit gemini-cli "${GEMINI_SETTINGS}" mcp
  json_edit gemini-cli "${GEMINI_SETTINGS}" hooks gemini
fi
while IFS='|' read -r label cfg; do
  [ -n "${cfg}" ] || continue
  found=1
  json_edit "${label}" "${cfg}/mcp_config.json" mcp
  json_edit "${label}" "${cfg}/hooks.json" agy
done < <(antigravity_dirs)
[ "${found}" = 1 ] || echo "docker-agents: no agent found (Claude Code, Codex, Gemini CLI, Antigravity): nothing to do" >&2

if [ "${#RESULTS[@]}" -gt 0 ]; then
  # One line per entry, the status before the path: a long path (T3 Code folders) must not hide the status.
  printf '%s\n' "${RESULTS[@]}" | awk -F'|' '
    { a[NR] = $1; t[NR] = $2; s[NR] = $3; if (length($1) > w) w = length($1) }
    END { f = "%-" w "s %s [%s]\n"; for (i = 1; i <= NR; i++) printf f, a[i], s[i], t[i] }'
fi

if [ "${REMOVE}" = 0 ] && [ -n "$(command -v docker || true)" ]; then
  if [ "$(docker inspect -f '{{.State.Running}}' "${CONTAINER}" 2>/dev/null || true)" != "true" ]; then
    echo "docker-agents: container '${CONTAINER}' is not running: start it with 'docker compose up -d' (agents connect to it)" >&2
  else
    reply="$(printf '%s\n%s\n' \
      '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"docker-agents","version":"1"}}}' \
      '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' |
      docker "${MCP_ARGS[@]}" 2>/dev/null | tail -n 1 || true)"
    tools="$(printf '%s' "${reply}" | grep -o '"inputSchema"' | wc -l | tr -d ' ')"
    if [ "${tools}" -gt 0 ]; then
      echo "Checked: '${CONTAINER}' answers over MCP with ${tools} tools."
    else
      echo "docker-agents: '${CONTAINER}' is running but did not answer tools/list: check 'docker logs ${CONTAINER}'" >&2
    fi
    # Claude Code's handoff at Stop reads the size of the session from its transcript.
    transcripts="$(docker inspect -f '{{range .Mounts}}{{if eq .Destination "/host/claude-projects"}}{{.Type}}{{end}}{{end}}' "${CONTAINER}" 2>/dev/null || true)"
    if [ "${claude_hooks}" = 1 ] && [ "${transcripts}" != "bind" ]; then
      echo "Note: '${CONTAINER}' cannot read Claude Code's session files, so the handoff of a long session at Stop is off. Set FOOTNOTE_CLAUDE_PROJECTS (DOCKER.md, Section 3.1)."
    fi
  fi
fi
if [ "${DRY_RUN}" = 0 ] && printf '%s\n' "${RESULTS[@]:-}" | grep -Eq '\|(registered|replaced|removed)'; then
  echo "Restart each agent to load the change."
fi
