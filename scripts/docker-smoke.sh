#!/usr/bin/env bash
# End-to-end smoke verification for the official Footnote Docker image (#392).
# Runs completely isolated with a temporary named Docker volume and cleans up on exit.
# Linux, macOS, and Git Bash on Windows.
set -euo pipefail

IMAGE="${1:-footnote:local}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR=/state
WORKSPACE_DIR=/workspace
WORKSPACES_DIR=/workspaces
DB_PATH="${STATE_DIR}/memory.db"
EXPECTED_UID="${FOOTNOTE_UID:-10001}"
HOST_PREFIX='C:\work\repos'
HOST_REPO="${HOST_PREFIX}\\footnote"
SUFFIX="$$-$(date +%s)"
MARKER="docker-smoke-${SUFFIX}"
TODAY="$(date -u +%Y-%m-%d)"
STATE_VOL="footnote-smoke-state-${SUFFIX}"
DAEMON_NAME="footnote-smoke-daemon-${SUFFIX}"
SCRATCH="$(mktemp -d)"
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}'
TOOLS_LIST='{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# Git Bash: Docker needs Windows paths for bind mounts, and MSYS must not rewrite container paths (/var/lib/...).
host_path() { if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else printf '%s' "$1"; fi; }
if command -v cygpath >/dev/null 2>&1; then export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'; fi
REPO_MOUNT="$(host_path "${REPO_ROOT}")"

cleanup() {
  docker rm -f "${DAEMON_NAME}" >/dev/null 2>&1 || true
  docker volume rm -f "${STATE_VOL}" >/dev/null 2>&1 || true
  rm -rf -- "${SCRATCH:?}"
}
trap cleanup EXIT

HARDENED_ARGS=(
  --network none
  --read-only
  --tmpfs /tmp:rw,noexec,nosuid,size=64m
  --cap-drop=ALL
  --security-opt=no-new-privileges:true
)

echo "[1/9] Verify non-root user (UID ${EXPECTED_UID}), git, scoped git trust, and system CA bundle"
uid="$(docker run --rm --entrypoint id "${IMAGE}" -u)"
[ "${uid}" = "${EXPECTED_UID}" ] || { echo "Expected UID ${EXPECTED_UID}, got ${uid}" >&2; exit 1; }
docker run --rm --entrypoint git "${IMAGE}" --version >/dev/null
docker run --rm --entrypoint test "${IMAGE}" -s /etc/ssl/certs/ca-certificates.crt
if docker run --rm --entrypoint sh "${IMAGE}" -c 'command -v curl >/dev/null 2>&1'; then
  echo "Unexpected curl found in minimal runtime image" >&2
  exit 1
fi
if docker run --rm --entrypoint git "${IMAGE}" config --system --get-all safe.directory | grep -qx '\*'; then
  echo "git trusts every directory (safe.directory=*): only the workspace mounts may be trusted" >&2
  exit 1
fi

echo "[2/9] Verify one-shot CLI under hardened flags (--network none --read-only)"
docker run --rm "${HARDENED_ARGS[@]}" "${IMAGE}" --version
docker run --rm "${HARDENED_ARGS[@]}" \
  -v "${STATE_VOL}:${STATE_DIR}" \
  "${IMAGE}" status >/dev/null

echo "[3/9] Verify read-only workspace code-index (tree-sitter) and grep"
docker run --rm "${HARDENED_ARGS[@]}" \
  -v "${STATE_VOL}:${STATE_DIR}" \
  -v "${REPO_MOUNT}:${WORKSPACE_DIR}:ro" \
  "${IMAGE}" code-index "${WORKSPACE_DIR}" >/dev/null
docker run --rm "${HARDENED_ARGS[@]}" \
  -v "${STATE_VOL}:${STATE_DIR}" \
  -v "${REPO_MOUNT}:${WORKSPACE_DIR}:ro" \
  "${IMAGE}" grep "map_host_path" "${WORKSPACE_DIR}" --glob "*.rs" >/dev/null

echo "[4/9] Verify MCP tools/list byte budget (<= 5,000 B, 8 tools) over serve --mcp"
mcp_out="$(printf '%s\n%s\n' "${INIT}" "${TOOLS_LIST}" \
  | docker run --rm -i "${HARDENED_ARGS[@]}" \
      -v "${STATE_VOL}:${STATE_DIR}" \
      "${IMAGE}" --db "${DB_PATH}" serve --mcp)"
replies="$(printf '%s\n' "${mcp_out}" | grep -c .)"
[ "${replies}" = 2 ] || { echo "expected 2 JSON-RPC replies, got ${replies}" >&2; exit 1; }
tools_reply="$(printf '%s\n' "${mcp_out}" | grep . | sed -n 2p)"
tools="$(printf '%s' "${tools_reply}" | grep -o '"inputSchema"' | wc -l | tr -d ' ')"
# The reply is compact JSON: the result is the line minus the envelope's keys and the closing brace.
envelope='{"jsonrpc":"2.0","id":2,"result":'
size=$(($(printf '%s' "${tools_reply}" | LC_ALL=C wc -c | tr -d ' ') - ${#envelope} - 1))
[ "${tools}" = 8 ] || { echo "expected 8 tools, got ${tools}" >&2; exit 1; }
[ "${size}" -le 5000 ] || { echo "tools/list exceeded 5000 B: ${size}" >&2; exit 1; }
echo "MCP tools/list OK: ${tools} tools, ${size} B <= 5000 B"

echo "[5/9] Verify cross-container memory persistence (remember -> recall)"
printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"footnote_remember","arguments":{"memories":[{"kind":"fact","scope":"footnote","text":"Docker smoke persistence marker %s","observed_at":"%s"}]}}}\n' "${MARKER}" "${TODAY}" \
  | docker run --rm -i "${HARDENED_ARGS[@]}" \
      -v "${STATE_VOL}:${STATE_DIR}" \
      "${IMAGE}" --db "${DB_PATH}" serve --mcp >/dev/null

recall_out="$(printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"footnote_recall","arguments":{"query":"%s","mode":"lexical"}}}\n' "${MARKER}" \
  | docker run --rm -i "${HARDENED_ARGS[@]}" \
      -v "${STATE_VOL}:${STATE_DIR}" \
      "${IMAGE}" --db "${DB_PATH}" serve --mcp)"
echo "${recall_out}" | grep -q "${MARKER}" || {
  echo "Persistence recall failed: ${recall_out}" >&2
  exit 1
}

echo "[6/9] Verify FOOTNOTE_PATH_MAP Windows host path translation for hook + footnote_code"
host_repo_json="${HOST_REPO//\\/\\\\}"
printf '{"hook_event_name":"SessionStart","session_id":"smoke-map","cwd":"%s"}\n' "${host_repo_json}" \
  | docker run --rm -i "${HARDENED_ARGS[@]}" \
      -e "FOOTNOTE_PATH_MAP=${HOST_PREFIX}=${WORKSPACES_DIR}" \
      -v "${STATE_VOL}:${STATE_DIR}" \
      -v "${REPO_MOUNT}:${WORKSPACES_DIR}/footnote:ro" \
      "${IMAGE}" hook >/dev/null

docker run --rm "${HARDENED_ARGS[@]}" \
  -v "${STATE_VOL}:${STATE_DIR}" \
  -v "${REPO_MOUNT}:${WORKSPACES_DIR}/footnote:ro" \
  "${IMAGE}" code-index "${WORKSPACES_DIR}/footnote" >/dev/null

code_out="$(printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"footnote_code","arguments":{"root":"%s","name":"map_host_path"}}}\n' "${host_repo_json}" \
  | docker run --rm -i "${HARDENED_ARGS[@]}" \
      -e "FOOTNOTE_PATH_MAP=${HOST_PREFIX}=${WORKSPACES_DIR}" \
      -v "${STATE_VOL}:${STATE_DIR}" \
      -v "${REPO_MOUNT}:${WORKSPACES_DIR}/footnote:ro" \
      "${IMAGE}" --db "${DB_PATH}" serve --mcp)"
echo "${code_out}" | grep -q "map_host_path" || {
  echo "Mapped footnote_code lookup failed: ${code_out}" >&2
  exit 1
}

echo "[7/9] Verify a mounted repository's .git/config cannot start a program (fsmonitor, filter driver)"
evil="${SCRATCH}/evil"
mkdir -p "${evil}"
evil_git="$(host_path "${evil}")" # host git is a Windows program under Git Bash
git -C "${evil_git}" init -q
printf 'print(1)\n' >"${evil}/app.py"
printf '* filter=smoke\n' >"${evil}/.gitattributes"
git -C "${evil_git}" -c user.name=smoke -c user.email=smoke@example.invalid add app.py .gitattributes
git -C "${evil_git}" -c user.name=smoke -c user.email=smoke@example.invalid commit -qm init
printf 'print(2)\n' >"${evil}/app.py" # stat-dirty: `status` would hash it through the filter
# Armed last: no git command runs on the host in this repository from here on.
git -C "${evil_git}" config core.fsmonitor "sh -c 'id > /tmp/footnote-smoke-ran' #"
git -C "${evil_git}" config filter.smoke.clean "sh -c 'id > /tmp/footnote-smoke-ran; cat' #"
printf '{"hook_event_name":"SessionStart","session_id":"smoke-evil","cwd":"%s/evil"}\n' "${WORKSPACES_DIR}" \
  | docker run --rm -i "${HARDENED_ARGS[@]}" \
      -v "${STATE_VOL}:${STATE_DIR}" \
      -v "${evil_git}:${WORKSPACES_DIR}/evil:ro" \
      --entrypoint sh "${IMAGE}" -c '
        footnote hook >/dev/null 2>&1
        footnote code-index /workspaces/evil >/dev/null 2>&1
        footnote projects >/dev/null 2>&1
        if [ -e /tmp/footnote-smoke-ran ]; then echo "repository config ran: $(cat /tmp/footnote-smoke-ran)" >&2; exit 1; fi' || {
  echo "A mounted repository started a program inside the container" >&2
  exit 1
}

echo "[8/9] Verify long-lived daemon mode + docker exec -i mcp proxy"
docker run -d --init --name "${DAEMON_NAME}" "${HARDENED_ARGS[@]}" \
  -v "${STATE_VOL}:${STATE_DIR}" \
  -v "${REPO_MOUNT}:${WORKSPACES_DIR}/footnote:ro" \
  "${IMAGE}" --db "${DB_PATH}" serve --mcp --daemon >/dev/null

ready=0
for _ in $(seq 1 30); do
  if docker exec "${DAEMON_NAME}" sh -c 'for s in "$FOOTNOTE_STATE_DIR"/footnote-*.sock; do [ -S "$s" ] && exit 0; done; exit 1'; then
    ready=1
    break
  fi
  sleep 0.2
done
[ "${ready}" = "1" ] || {
  echo "Daemon socket did not appear in time" >&2
  docker logs "${DAEMON_NAME}" >&2 || true
  exit 1
}

daemon_status="$(docker exec "${DAEMON_NAME}" footnote status)"
echo "${daemon_status}" | grep -q '"daemon_running":true' || {
  echo "Expected daemon_running:true in status: ${daemon_status}" >&2
  exit 1
}

exec_out="$(printf '%s\n{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"footnote_recall","arguments":{"query":"%s","mode":"lexical"}}}\n' \
  "${INIT}" "${MARKER}" \
  | docker exec -i "${DAEMON_NAME}" footnote --db "${DB_PATH}" mcp)"
echo "${exec_out}" | grep -q "${MARKER}" || {
  echo "Long-lived daemon exec proxy recall failed: ${exec_out}" >&2
  exit 1
}

echo "[9/9] All Docker smoke checks passed."
