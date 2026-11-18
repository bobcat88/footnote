# Security audit, 2026-11-18

Two passes over `main` (first at `bb66e72`, second at `aba87af`). Read-only review of the Rust engine, the MCP server, the daemon,
the state folder, model downloads, the Tauri launcher and CI. HTML design files were out of scope.

**Verdict:** no critical, high or medium issue open. Two medium issues found in the first pass are fixed.

## Fixed

| Finding | Fix |
|---|---|
| MEDIUM: an MCP client could pass any folder as `model`; the encoder checks files against the manifest in that same folder, so the client chose its own pins | #203: only installed models, folders under the models folder, or folders the database already has vectors from; one refusal message for all cases |
| MEDIUM: `e5-small` and `minilm` had no pinned sha256 (trust on first download) | #203: model and tokenizer hashes pinned |
| Design work folders polluting the repository | #202: `.design-studio/`, `design/qloop/`, `design/demos/` untracked and ignored |

## Open

| Severity | Finding | Where | Suggested fix |
|---|---|---|---|
| LOW | Agent memories persist injected text: an agent steered by a hostile document can `footnote_remember` instructions that the hook then recalls on later prompts, across sessions (labelled as data, but labels are not a guarantee) | `src/hook.rs:227`, `footnote_remember` | Record memory origin (hook/MCP, session) and show it on recall; project scope by default for MCP writes; let the user review new memories |
| LOW | Daemon endpoint on Linux is an abstract socket with a predictable name: another local user can squat it (denial of sharing only; the handshake blocks impersonation) | `src/daemon.rs:33` | Socket file inside the private state folder |
| LOW | A pre-existing state folder is never tightened to 0700 / user-only (secret files themselves are 0600) | `crates/core/src/state.rs:83` | Check and fix permissions when the folder exists |
| LOW | Docs say "no network" / "model downloads are the only network access", but `footnote crawl` fetches websites | `README.md`, `SECURITY.md`, `Cargo.toml`, `crates/library/src/crawl.rs` | Document `crawl` |
| LOW | `code-index` runs `rust-analyzer scip`, which runs the project's build scripts and proc macros: indexing an untrusted repository runs its code (explicit CLI action only) | `crates/code/src/lib.rs:97` | Document; consider a warning |
| LOW | `footnote_code` MCP tool accepts any `root` (only reads graphs already built) | `src/mcp.rs` | Restrict to known projects |
| INFO | The audit hash chain cannot detect the newest entries being cut off (needs write access to the state folder, out of scope) | `crates/core/src/audit.rs:232` | Say so in the security model, or keep a MAC'd checkpoint elsewhere |
| INFO | CI actions referenced by version tag, not commit SHA; release installs tools with `curl \| sh` (pinned versions, HTTPS) | `.github/workflows/` | Pin to SHAs |
| INFO | Dependencies: `paste` unmaintained (already allowed); `picomatch` denial-of-service issue via Vite, build time only | `Cargo.lock`, `apps/launcher` | Update Vite when convenient |

## Checked and clean

- **Access control:** mutual HMAC handshake on the daemon (token never sent, no replay, fails closed without a token); agents cannot write to a corpus; read tools never create a database; strict launcher command allowlist.
- **Injection:** SQL is fixed text with bound values; no shell anywhere, argument lists only, Python helper run with `-I` and a clean environment; file names sanitised; no symlink following; JSON only, with size limits; no `innerHTML`/`eval` in the launcher, strict CSP.
- **Secrets and crypto:** no credentials in tracked files; state files 0600 / user-only; constant-time HMAC comparison; OS randomness; personal store encrypted with SQLCipher and a zeroized 32-byte key.
- **Supply chain / CI:** read-only workflow token, no `pull_request_target`, PR title passed through an environment variable; all model downloads hash-pinned.
- **LLM surface:** retrieved text and memories labelled untrusted; no model output executed or used for a security decision.

## Not covered

- Python helpers in `tools/` and `scripts/` (beyond how they are started); `crates/alias`; `windows_keystore.rs` (needs Windows).
- Memory safety of the third-party PDF, ONNX and GGUF parsers.
- Whether the Windows named pipe really gets an owner-only security descriptor.
- 13 tests could not run in the review container (no Python venv, models or indexers) and fail the same way on `main` there: run `cargo test --all-features` locally, especially after #203.

## Next

1. Memory provenance and project scope by default.
2. Full local test run.
3. Document `crawl` and the audit-chain limit.
