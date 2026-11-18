# Security policy

Footnote is a local tool that serves your own documents to your own agents. It opens no outbound network connection except in two commands you run yourself, `footnote models install` and `footnote crawl <url>` (see the README's security model for their limits), and its MCP tools cannot write to your documents; the only write tools add or retire agent memories in a separate memory database. Agent memories are written by the agent you already use, into a local file; no other model receives your conversation.
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

## Out of scope

- An attacker who already has write access to your user account, the database file or the state folder: they can change anything Footnote can read.
- Quality of retrieval results (use a normal issue, with the benchmark protocol in `bench/`).
- Vulnerabilities in third-party models or libraries, unless Footnote's use of them is the cause.
