# Remote access from phones and tablets: how T3 Code does it

16 November 2026. Read from T3 Code's own documentation ([pingdotgg/t3code](https://github.com/pingdotgg/t3code), MIT):
`docs/user/remote-access.md`, `docs/internals/remote.md`, `t3-connect.md`, `environment-auth.md`, `connection-runtime.md` and
`infra/relay/README.md`. Johan uses it daily from his phone and finds it reliable.

## How it works

**One server per machine.** The machine running the agents (the "environment") holds the threads, files and state. Phones,
browsers and other desktops are clients over HTTP and WebSocket. Every way of reaching the machine is just a route to the same
server; none adds a second execution model.

**Two ways in:**

| Route | How | Cost |
|---|---|---|
| Direct (LAN, Tailscale, VPN) | `t3 pair` prints a one-time pairing link as a QR code. The phone scans it, trades the one-time token for its own session, and reconnects with that session afterwards. | Nothing hosted. The listener only accepts private, loopback and tailnet addresses, so a VPS's public address stays closed (not a firewall: a rewriting proxy looks private). |
| T3 Connect (anywhere) | Sign in with the same account (Clerk) on both sides. A hosted relay (Cloudflare Worker) creates a managed tunnel to the machine and brokers a one-time credential; after that, traffic goes through the tunnel straight to the machine, not through the relay. | A hosted service, an identity provider, tunnels and DNS. 3 machines free, more on paid plans. |

**Security choices worth copying:**

- Pairing grants scopes, and a session can only narrow them. Pairing secrets are shown once and never readable afterwards.
- Tokens are bound to the device's key (DPoP), so a stolen token is useless without the key; WebSocket upgrades use short-lived
  tickets, so long-lived tokens never appear in URLs.
- Optional device approval: a new device waits with a code (`K7QM-2XFD`) until an already-trusted device approves it, so a
  hijacked cloud account alone cannot reach the machine.
- Outside agents get a separate, narrow OAuth audience on `/mcp` (read-only or a mode ceiling), never the full session.

**Why it feels reliable:** one connection owner per machine on each client; retries with jittered backoff capped at five minutes;
on foregrounding or a network change it probes the live socket and replaces it only when the probe fails; cached data stays
readable offline without pretending to be live; a machine keeps an ordered list of routes (LAN first, then tailnet, then relay)
and quietly moves to a faster one when it answers.

## What this means for Footnote

**Footnote is already reachable from the phone, through T3 Code.** The agents T3 runs on this machine call Footnote's MCP and
hook, so recall and saves work from the phone today. No Footnote work is needed for that.

What a phone or tablet would add on its own: reading and correcting memories, searching a Library, seeing activity (the
visibility principle), approving a proposed memory. All of this is a small read-mostly API, not agent execution.

Recommended path, cheapest first:

1. **Now: nothing.** Use T3 Code as the remote front end.
2. **When the launcher exists:** let it serve its screens to other devices on the LAN or tailnet, with T3's pairing model: a
   one-time QR code, a per-device session with narrow scopes (read memories; write needs its own scope), private addresses
   only, sessions listed and revocable in the launcher. No account, no cloud. Built once in Rust, so Windows, Linux and macOS
   behave the same.
3. **A hosted relay only if a customer asks for it**, and probably never for companies: they already have a VPN, and a relay
   is one more processor for the DPO and IT security to review. Personal users can use Tailscale.

Not taken from T3: its code (TypeScript and Effect, not reusable in Rust), its relay and its account system.
