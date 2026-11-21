---
skill_hash: c7404392318fed1a
reviewed: 2026-11-21
---
# fix-ci-and-releases: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| Pin to full SHA, least-privilege token | [GitHub: secure use](https://docs.github.com/en/actions/reference/security/secure-use) |
| Script injection, env indirection | [GitHub: script injections](https://docs.github.com/en/actions/concepts/security/script-injections) |
| pull_request_target risk | [GitHub Security Lab: pwn requests](https://securitylab.github.com/resources/github-actions-preventing-pwn-requests/) |
| Token permissions | [GITHUB_TOKEN permissions](https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/controlling-permissions-for-github_token) |
| Cache keys and misses | [Dependency caching](https://docs.github.com/en/actions/concepts/workflows-and-actions/dependency-caching) |
| Provenance | [Artifact attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds) |
| `--log-failed` | [gh run view](https://cli.github.com/manual/gh_run_view) |

## Field evidence
- A Rust release broke on both targets after a native-library bump (glibc 2.38 needed vs 2.35 on ubuntu-22.04; dynamic vs static CRT on Windows). Basis for failing-run step 3.
