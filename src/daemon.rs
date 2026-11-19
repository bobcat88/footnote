//! One shared, resident daemon per corpus, reachable by every agent.
//!
//! Transport is a local socket: a named pipe on Windows (`\\.\pipe\...`, local
//! only, owner-scoped) or a Unix domain socket elsewhere. There is no TCP port.
//! The first line on every connection is a token handshake read from a per-user
//! file, so a local process that guesses the endpoint still cannot talk to it.
//!
//! Agents never manage this process: their MCP entry points at `footnote
//! mcp`, which connects to the daemon and spawns it on demand. That is what makes
//! one warm corpus shared by Claude Code, Codex and Gemini at the same time.
use crate::mcp::{Shared, session_loop};
use anyhow::{Context, Result};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// Sessions served at once. A handful of agents is normal; this only stops floods.
const MAX_SESSIONS: usize = 64;
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// When the last session ended, in seconds since `STARTED` (idle exit).
static LAST_END: AtomicU64 = AtomicU64::new(0);
static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Endpoint name derived from the corpus path, so separate databases never share
/// a daemon and a moved corpus gets its own.
pub fn endpoint_name(db: &Path) -> String {
    let canonical = std::fs::canonicalize(db).unwrap_or_else(|_| db.to_path_buf());
    let digest = crate::app::hash(canonical.to_string_lossy().as_bytes());
    format!("footnote-{}", &digest[..16])
}

fn endpoint_of(db: &Path) -> String {
    endpoint_name(db)
}

/// Run the daemon: accept many concurrent agents, one thread each.
pub fn run(shared: Shared, idle: Option<Duration>) -> Result<()> {
    let endpoint = endpoint_of(&shared.db);
    let name = endpoint.as_str().to_ns_name::<GenericNamespaced>()?;
    let options = ListenerOptions::new().name(name);
    // Owner and SYSTEM only (#208): Windows' default pipe access also lets Everyone and Anonymous open it read-only.
    #[cfg(windows)]
    let options = {
        use interprocess::os::windows::{
            local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
        };
        let sddl = widestring::U16CString::from_str("D:P(A;;GA;;;SY)(A;;GA;;;OW)")?;
        options.security_descriptor(SecurityDescriptor::deserialize(&sddl)?)
    };
    let listener = options.create_sync().with_context(|| {
        format!(
            "cannot create a daemon endpoint for {} (another daemon may already serve it)",
            shared.db.display()
        )
    })?;
    // Record the pid so operators (and tests) can stop an owned daemon without
    // killing every other footnote process on the machine.
    if let Ok(state) = crate::state::state_dir() {
        let _ = crate::state::ensure_private_dir(&state);
        if let Err(e) = crate::state::fix_private_dir(&state) {
            eprintln!("footnote: the state folder may be readable by others: {e:#}");
        }
        let _ = std::fs::write(state.join("daemon.pid"), std::process::id().to_string());
    }
    if let Some(idle) = idle {
        // Started on demand (the memory hook): leave once no session has been open for `idle`, so nothing stays resident
        // that nobody uses. Agents' MCP sessions stay open, so the daemon lives as long as one of them does.
        let started = *STARTED.get_or_init(Instant::now);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(5).min(idle));
                let quiet = started.elapsed().as_secs() - LAST_END.load(Ordering::SeqCst);
                if ACTIVE.load(Ordering::SeqCst) == 0 && quiet >= idle.as_secs() {
                    std::process::exit(0);
                }
            }
        });
    }
    let shared = Arc::new(shared);
    for connection in listener.incoming() {
        let stream = match connection {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("footnote daemon: accept failed: {e}");
                continue;
            }
        };
        // Bound the number of concurrent sessions so a flood of connections
        // cannot exhaust threads or memory. Real use is a handful of agents.
        if ACTIVE.fetch_add(1, Ordering::SeqCst) >= MAX_SESSIONS {
            ACTIVE.fetch_sub(1, Ordering::SeqCst);
            eprintln!("footnote daemon: refused a connection (limit {MAX_SESSIONS})");
            continue;
        }
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(&stream);
            let mut out = &stream;
            if let Err(e) = session_loop(&mut reader, &mut out, &shared, true) {
                // A rejected handshake or a dropped agent is routine, not fatal.
                eprintln!("footnote daemon: session ended: {e:#}");
            }
            ACTIVE.fetch_sub(1, Ordering::SeqCst);
            if let Some(started) = STARTED.get() {
                LAST_END.store(started.elapsed().as_secs(), Ordering::SeqCst);
            }
        });
    }
    Ok(())
}

/// Connect to this corpus's daemon. Never spawns a child process: a daemon
/// started by an agent would inherit the agent's stdout pipe on Windows and
/// keep it open after this process exits, hanging the agent. `install` registers
/// the daemon at logon instead; when it is absent, the caller serves stdio
/// in-process.
pub fn connect(db: &Path) -> Result<Stream> {
    let endpoint = endpoint_of(db);
    Ok(Stream::connect(
        endpoint.as_str().to_ns_name::<GenericNamespaced>()?,
    )?)
}

#[cfg(feature = "memory")]
/// One tool call on the running daemon (the memory hook's recall). Fails when no daemon runs: the caller falls back.
pub fn call(db: &Path, tool: &str, arguments: &serde_json::Value) -> Result<serde_json::Value> {
    let stream = connect(db).context("no resident daemon")?;
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;
    crate::mcp::client_handshake(
        &mut reader,
        &mut writer,
        &crate::state::load_or_create_token()?,
        &crate::state::random_hex(32)?,
    )?;
    let request = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}});
    writer.write_all(
        format!(
            "{request}
"
        )
        .as_bytes(),
    )?;
    writer.flush()?;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            anyhow::bail!("daemon closed the connection");
        }
        let reply: serde_json::Value =
            serde_json::from_str(&line).context("daemon reply: not JSON")?;
        if reply["id"] != 1 {
            continue; // a notification
        }
        let result = &reply["result"];
        let text = result["content"][0]["text"].as_str().unwrap_or_default();
        if result["isError"] == true || !reply["error"].is_null() {
            anyhow::bail!(
                "daemon: {}",
                if text.is_empty() {
                    reply["error"].to_string()
                } else {
                    text.to_string()
                }
            );
        }
        return serde_json::from_str(text).context("daemon result: not JSON");
    }
}

/// Stdio front-end for agents: use the shared daemon when it is running,
/// otherwise serve this session in-process.
///
/// The agent always speaks plain MCP on its stdin/stdout; the socket and the
/// token are invisible to it and to the user.
pub fn proxy(shared: &Shared) -> Result<()> {
    match connect(&shared.db) {
        Ok(stream) => proxy_over_stream(stream, shared),
        Err(_) => {
            // No resident daemon: stay functional, just not shared.
            eprintln!(
                "footnote: no shared daemon for this corpus; serving in-process \
                 (run `footnote --db <DB> serve --mcp --daemon` or re-run install to share one)"
            );
            let stdin = std::io::stdin();
            let mut reader = BufReader::new(stdin.lock());
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            session_loop(&mut reader, &mut out, shared, false)
        }
    }
}

fn proxy_over_stream(stream: Stream, shared: &Shared) -> Result<()> {
    let token = crate::state::load_or_create_token()?;
    {
        // Mutual challenge-response: the token is never sent, and nothing is
        // forwarded to a peer that cannot prove it holds the token.
        let mut reader = BufReader::new(&stream);
        let mut writer = &stream;
        crate::mcp::client_handshake(
            &mut reader,
            &mut writer,
            &token,
            &crate::state::random_hex(32)?,
        )?;
    }
    let stream = Arc::new(stream);
    let requests = Arc::new(AtomicUsize::new(0));
    let responses = Arc::new(AtomicUsize::new(0));

    // Daemon -> agent.
    let reader_stream = Arc::clone(&stream);
    let sent_responses = Arc::clone(&responses);
    let pump_out = std::thread::spawn(move || -> Result<()> {
        let mut reader = BufReader::new(&*reader_stream);
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                break;
            }
            out.write_all(&line)?;
            out.flush()?;
            sent_responses.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    });

    // Agent -> daemon.
    let writer_stream = Arc::clone(&stream);
    let sent_requests = Arc::clone(&requests);
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = &*writer_stream;
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        // Count requests (messages carrying an id) so we know when the last
        // response has been delivered before exiting on stdin EOF.
        if String::from_utf8_lossy(&line).contains("\"id\"") {
            sent_requests.fetch_add(1, Ordering::Relaxed);
        }
        if writer
            .write_all(&line)
            .and_then(|()| writer.flush())
            .is_err()
        {
            // The daemon went away (stopped, crashed, or replaced by an install): keep this agent's tools working
            // in-process instead of closing its MCP connection, starting with the request that just failed.
            // shortcut: a request already forwarded when the daemon died gets no answer (the agent sees a timeout for
            // that one call); replaying unanswered requests would need per-id tracking, worth it only if this shows up.
            eprintln!(
                "footnote: the shared daemon closed; serving in-process for the rest of this session"
            );
            let _ = pump_out.join(); // it ends on the closed socket and releases stdout
            let mut rest = std::io::Cursor::new(std::mem::take(&mut line)).chain(reader);
            return session_loop(&mut rest, &mut std::io::stdout().lock(), shared, false);
        }
    }

    // Drain the last reply(s) so a closing agent does not lose its answer. The
    // daemon keeps the socket open until we go away, so this thread is not
    // joined: process exit ends it, which is also what releases the connection.
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline
        && responses.load(Ordering::Relaxed) < requests.load(Ordering::Relaxed)
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    use crate::mcp::{client_handshake, proof, server_handshake};

    const SN: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const CN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn auth_line(token: &str, client_nonce: &str) -> String {
        format!(
            "{{\"footnote\":{{\"auth\":{{\"nonce\":\"{client_nonce}\",\"proof\":\"{}\"}}}}}}\n",
            proof(token, "c", SN, client_nonce)
        )
    }

    fn serve(input: &str, token: &str) -> (Result<()>, String) {
        let mut reader = BufReader::new(input.as_bytes());
        let mut out: Vec<u8> = Vec::new();
        let result = server_handshake(&mut reader, &mut out, token, SN);
        (result, String::from_utf8_lossy(&out).to_string())
    }

    #[test]
    fn a_correct_proof_is_accepted_and_the_server_proves_itself_back() {
        let token = "a".repeat(64);
        let (result, out) = serve(&auth_line(&token, CN), &token);
        assert!(result.is_ok(), "{result:?}");
        assert!(out.contains("\"hello\""), "{out}");
        assert!(out.contains(&proof(&token, "s", SN, CN)), "{out}");
        assert!(!out.contains(&token), "the token must never be sent");
    }

    #[test]
    fn a_wrong_token_is_rejected_and_gets_no_server_proof() {
        let token = "a".repeat(64);
        let (result, out) = serve(&auth_line(&"b".repeat(64), CN), &token);
        assert!(result.is_err(), "wrong token accepted");
        assert!(
            !out.contains("\"ready\""),
            "server proved itself to a stranger"
        );
    }

    #[test]
    fn a_proof_for_another_nonce_cannot_be_replayed() {
        let token = "a".repeat(64);
        let other = "3333333333333333333333333333333333333333333333333333333333333333";
        let recorded = format!(
            "{{\"footnote\":{{\"auth\":{{\"nonce\":\"{CN}\",\"proof\":\"{}\"}}}}}}\n",
            proof(&token, "c", other, CN)
        );
        assert!(serve(&recorded, &token).0.is_err(), "replay accepted");
    }

    #[test]
    fn skipping_oversized_or_malformed_handshakes_is_rejected() {
        let token = "a".repeat(64);
        let mcp = format!("{}\n", r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        assert!(
            serve(&mcp, &token).0.is_err(),
            "unauthenticated request accepted"
        );
        assert!(serve(&auth_line(&token, "short"), &token).0.is_err());
        assert!(
            serve(&"x".repeat(10_000), &token).0.is_err(),
            "oversized line accepted"
        );
        assert!(serve("", &token).0.is_err(), "closed connection accepted");
    }

    #[test]
    fn a_daemon_without_a_token_fails_closed() {
        let dir = std::env::temp_dir().join(format!("footnote-notoken-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let shared = Shared::new(&dir.join("corpus.db"), None, None);
        let mut reader = BufReader::new(&b""[..]);
        let mut out: Vec<u8> = Vec::new();
        assert!(session_loop(&mut reader, &mut out, &shared, true).is_err());
    }

    #[test]
    fn the_client_refuses_a_server_that_does_not_know_the_token() {
        let token = "a".repeat(64);
        let hello = format!("{{\"footnote\":{{\"hello\":{{\"nonce\":\"{SN}\"}}}}}}\n");
        let ready = |t: &str| {
            format!(
                "{{\"footnote\":{{\"ready\":{{\"proof\":\"{}\"}}}}}}\n",
                proof(t, "s", SN, CN)
            )
        };
        let run = |t: &str| {
            let input = format!("{hello}{}", ready(t));
            let mut reader = BufReader::new(input.as_bytes());
            let mut out: Vec<u8> = Vec::new();
            let result = client_handshake(&mut reader, &mut out, &token, CN);
            (result, String::from_utf8_lossy(&out).to_string())
        };
        let (good, sent) = run(&token);
        assert!(good.is_ok(), "{good:?}");
        assert!(!sent.contains(&token), "the client leaked the token");
        assert!(run(&"b".repeat(64)).0.is_err(), "impostor server accepted");
    }
}
