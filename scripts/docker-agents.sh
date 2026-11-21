#!/usr/bin/env bash
# Register the Footnote Docker hub as an MCP server in every agent found for this user (#392).
#
# Detects Claude Code, Codex, Gemini CLI and Antigravity, and points each of them at the long-lived Compose daemon:
#     <absolute path of docker> exec -i <container> footnote --db <db> mcp
# - An existing entry with the same name is left as it is: --force replaces it, --remove deletes it.
# - Claude Code is configured through its own command (`claude mcp add --scope user`), so its configuration file is
#   never edited here. The other agents' files are backed up once (<file>.footnote-backup) and replaced atomically.
#   A file that the agent rewrote in the meantime is not overwritten. JSON files (Gemini CLI, Antigravity) need python3.
# - Nothing goes over the network. --dry-run shows what would change and writes nothing.
# - Anyone who can use this Docker engine can read every memory in the hub: see SECURITY.md, "Who can reach the hub".
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

usage() { sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; }
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
  # Git Bash: keep MSYS from rewriting container paths (/var/lib/...) passed to Windows programs, and register a
  # Windows path with forward slashes.
  export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'
  if [ -n "${DOCKER}" ] && [ -f "${DOCKER}.exe" ]; then DOCKER="${DOCKER}.exe"; fi
  if [ -n "${DOCKER}" ]; then DOCKER="$(cygpath -m "${DOCKER}")"; fi
fi
# GUI agents do not inherit the shell PATH: register the absolute path.
case "${DOCKER}" in *'"'* | *$'\n'* | *$'\r'*) die "unsupported characters in the docker path: ${DOCKER}" ;; esac
MCP_ARGS=(exec -i "${CONTAINER}" footnote --db "${DB}" mcp)

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

# `mcpServers.<NAME>` in a JSON settings file (Gemini CLI, Antigravity).
json_server() { # agent path
  local agent="$1" path="$2"
  # A real interpreter, not the Microsoft Store stub that only prints a hint.
  if ! python3 -c 'import json' >/dev/null 2>&1; then
    result "${agent}" "${path}" "skipped: python3 is needed to edit JSON; add \"${NAME}\" under mcpServers yourself"
    return
  fi
  local status
  status="$(python3 - "${path}" "${NAME}" "${DOCKER}" "${DRY_RUN}" "${FORCE}" "${REMOVE}" "${MCP_ARGS[@]}" <<'PY'
import json, os, re, sys, tempfile

path, name, docker, dry, force, remove, *args = sys.argv[1:]
dry, force, remove = dry == "1", force == "1", remove == "1"

def stamp():
    try:
        st = os.stat(path)
        return (st.st_mtime_ns, st.st_size)
    except FileNotFoundError:
        return None

before = stamp()
raw = open(path, encoding="utf-8").read() if before else ""
if raw.strip():
    # Rewriting would drop comments: leave such a file to the user.
    if re.search(r"(?m)^\s*//", raw) or "/*" in raw:
        print(f'skipped: the file has comments; add "{name}" under mcpServers yourself')
        sys.exit(0)
    try:
        root = json.loads(raw)
    except ValueError as e:
        sys.exit(f"{path} is not valid JSON: {e}")
    if not isinstance(root, dict):
        sys.exit(f"{path} does not hold a JSON object")
else:
    root = {}
servers = root.get("mcpServers")
if servers is not None and not isinstance(servers, dict):
    sys.exit(f"{path}: mcpServers is not an object")
present = servers is not None and name in servers
if remove:
    if not present:
        print("not registered")
        sys.exit(0)
    del servers[name]
    status = "removed"
else:
    if present and not force:
        print("already registered (--force replaces it)")
        sys.exit(0)
    if servers is None:
        servers = root["mcpServers"] = {}
    servers[name] = {"command": docker, "args": args}
    status = "replaced" if present else "registered"
if not dry:
    folder = os.path.dirname(path) or "."
    os.makedirs(folder, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=folder, prefix=".footnote-", suffix=".tmp")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(json.dumps(root, indent=2, ensure_ascii=False) + "\n")
    if stamp() != before:
        os.unlink(tmp)
        sys.exit(f"{path} changed while it was being updated (the agent is probably running): close it and run this again")
    backup = path + ".footnote-backup"
    if before and not os.path.exists(backup):
        with open(path, "rb") as src, open(backup, "wb") as dst:
            dst.write(src.read())
    if before:
        os.chmod(tmp, os.stat(path).st_mode & 0o777)
    os.replace(tmp, path)
print(status)
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
  if [ "${REMOVE}" = 1 ] && [ "${present}" = 0 ]; then result codex "${path}" "not registered"; return; fi
  if [ "${present}" = 1 ] && [ "${FORCE}" = 0 ] && [ "${REMOVE}" = 0 ]; then
    result codex "${path}" "already registered (--force replaces it)"
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
  result codex "${path}" "${status}"
}

# Claude Code: its own command keeps ~/.claude.json consistent (Claude Code rewrites that file while it runs).
claude_server() {
  local target="claude mcp (user scope)" present=0 status
  if ! command -v claude >/dev/null 2>&1; then
    result claude-code "${target}" "skipped: the claude command is not on PATH; run: claude mcp add --scope user ${NAME} -- ${DOCKER} ${MCP_ARGS[*]}"
    return
  fi
  if (cd "${HOME_DIR}" && claude mcp get "${NAME}" >/dev/null 2>&1); then present=1; fi
  if [ "${REMOVE}" = 1 ] && [ "${present}" = 0 ]; then result claude-code "${target}" "not registered"; return; fi
  if [ "${present}" = 1 ] && [ "${FORCE}" = 0 ] && [ "${REMOVE}" = 0 ]; then
    result claude-code "${target}" "already registered (--force replaces it)"
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
  result claude-code "${target}" "${status}"
}

GEMINI="${HOME_DIR}/.gemini"
CODEX_DIR="${CODEX_HOME:-${HOME_DIR}/.codex}"
found=0
if command -v claude >/dev/null 2>&1 || [ -e "${HOME_DIR}/.claude" ]; then found=1; claude_server; fi
if command -v codex >/dev/null 2>&1 || [ -e "${CODEX_DIR}" ]; then found=1; codex_server "${CODEX_DIR}/config.toml"; fi
if command -v gemini >/dev/null 2>&1 || [ -f "${GEMINI}/settings.json" ]; then
  found=1
  json_server gemini-cli "${GEMINI}/settings.json"
fi
if [ -e "${GEMINI}/config" ] || [ -e "${GEMINI}/antigravity" ] || [ -e "${GEMINI}/antigravity-cli" ]; then
  found=1
  json_server antigravity "${GEMINI}/config/mcp_config.json"
fi
[ "${found}" = 1 ] || echo "docker-agents: no agent found (Claude Code, Codex, Gemini CLI, Antigravity): nothing to do" >&2

if [ "${#RESULTS[@]}" -gt 0 ]; then
  { echo "AGENT|TARGET|STATUS"; printf '%s\n' "${RESULTS[@]}"; } | if command -v column >/dev/null 2>&1; then column -t -s '|'; else tr '|' '\t'; fi
fi

if [ "${REMOVE}" = 0 ] && [ -n "${DOCKER}" ]; then
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
  fi
fi
if [ "${DRY_RUN}" = 0 ] && printf '%s\n' "${RESULTS[@]:-}" | grep -Eq '\|(registered|replaced|removed)'; then
  echo "Restart each agent to load the change."
fi
