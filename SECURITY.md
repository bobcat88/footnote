# Security policy

Footnote is a local tool that serves your own documents to your own agents. It opens outbound network connections only in commands you run yourself (`footnote models install`, `footnote crawl <url>`, `footnote docs add`, `footnote docs refresh`, `footnote docs install <url>`) and in `footnote find`'s fetch on miss, which reads up to 3 pages of an already cached docs site when no cached passage matches well and never sends your query (`FOOTNOTE_NO_LIVE_FETCH=1` turns it off; see the README's security model), and its MCP tools cannot write to your documents; the only write tools add or retire agent memories in a separate memory database. Agent memories are written by the agent you already use, into a local file; no other model receives your conversation.
The model that describes what it defends against is in the README under [Security model](README.md#security-model).

## Reporting a vulnerability

Please do not open a public issue for a security problem. Use GitHub's private reporting:
**Security → Report a vulnerability** on this repository.

Include what you did, what you expected, what happened, and the version (`footnote --version`) and operating system.

## In scope

- Anything that lets retrieved text act as an instruction to the server or to the agent beyond being labelled data.
- Any way for an MCP client to write to the corpus, read outside the indexed roots, or bypass the audit log.
- Weaknesses in the daemon's local socket authentication, the audit log's hash chain or MAC, or the private state folder. (Known limit, not a vulnerability: a hash chain cannot show that its newest entries were cut off; keep the last verified hash elsewhere.)
- A way to make the embedding encoder load a model that does not match its pinned manifest.
- Resource exhaustion through a query, a document or a helper response that the documented bounds should have stopped.
- In the official Docker setup (`Dockerfile`, `compose.yaml`): any path-traversal bypass in `FOOTNOTE_PATH_MAP`, privilege escalation out of the non-root container (`UID 10001`, `--network none`, `--read-only`, read-only workspace mounts), or a mounted repository (its files or its `.git/config`) that makes Footnote start a program inside the container.

## Container security boundary (`Dockerfile`, `compose.yaml`)

When running Footnote inside Docker (for example on Windows hosts where Application Control blocks local unsigned builds), the container preserves the same local-only, read-only threat model:

- Runs as dedicated non-root user `footnote` (`10001:10001`) with `ENTRYPOINT ["footnote"]`, `cap_drop: [ALL]`, `no-new-privileges`, and `--read-only` rootfs (writable `/tmp` tmpfs only, `noexec`).
- Uses `--network none` for offline CLI, MCP, and daemon workloads. Only the explicit network commands (`models install`, `docs add`, `crawl`) use a bridged network, never `--network host`. The Compose `footnote-net` service that runs them mounts no workspace, so the one container that can reach the network sees no repository.
- Stores secrets, databases, and daemon sockets inside a named Docker volume at `/state` (`0700`), never on a host bind-mount.
- Mounts user repositories read-only (`:ro`). Host-to-container path translation (`FOOTNOTE_PATH_MAP`) is operator-configured via environment variable only, matches full path-segment boundaries, and fails closed on `..` traversal segments before `known_project()` and `no_links()` run.
- Git trusts mounted repositories for ownership (`safe.directory`) only at `/workspace` and `/workspaces`, never `*`. Every git query Footnote runs turns `core.fsmonitor` off on the command line. `git status` is skipped for a repository that defines its own filter driver. So a repository's `.git/config` cannot start a program next to the state volume.
- The Compose daemon is bounded (`pids_limit`, `mem_limit`, rotated logs), so one runaway agent cannot exhaust the host.
- Base images are pinned by `@sha256` digest. The ONNX Runtime archive that `ort-sys` downloads at build time is checked against the hashes pinned in that crate, which `Cargo.lock` pins in turn. Dependabot proposes refreshed digests every week, so the runtime image picks up Debian security fixes.

### One person per container

Docker support is for one person: their agents, sessions and MCP hosts sharing one memory, as on a host. **Anyone who can use the Docker engine can read, change and delete every memory in the state volume** (through `docker exec`, `docker cp`, or another container on the volume), so do not share a stack between people.

- **Backups** (`footnote backup`) hold every memory in clear text. Store them with the same access rules as the volume.

## Out of scope

- An attacker who already has write access to your user account, the database file or the state folder: they can change anything Footnote can read.
- Quality of retrieval results (use a normal issue, with the benchmark protocol in `bench/`).
- Vulnerabilities in third-party models or libraries, unless Footnote's use of them is the cause.
