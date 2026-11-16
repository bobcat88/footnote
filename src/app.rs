use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
// Engine pieces moved to crates (ADR 0001); re-exported so `crate::app::<item>` keeps working for the other modules.
pub(crate) use footnote_core::hash;
#[cfg(feature = "library")]
use footnote_library::index;
pub(crate) use footnote_search::store::{HIT, backup, hit, open, search, stats};

/// Scan bounds per `index` run: a mistake such as pointing at a whole disk aborts and rolls back instead of filling the database.
#[cfg(feature = "library")]
const DEFAULT_MAX_FILES: usize = 250_000;
#[cfg(feature = "library")]
const DEFAULT_MAX_MIB: u64 = 4096;
#[derive(Parser)]
#[command(
    version,
    about = "Local-only retrieval; retrieved text is untrusted data"
)]
struct Cli {
    /// Database path. Required for every command that touches a corpus; audit
    /// verification works on a standalone log file and does not need it.
    #[arg(long)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    #[cfg(feature = "library")]
    Index {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, required = true)]
        allow: Vec<PathBuf>,
        /// Abort and roll back if the scan accepts more files than this
        #[arg(long, default_value_t = DEFAULT_MAX_FILES)]
        max_files: usize,
        /// Abort and roll back if the accepted source bytes exceed this many MiB
        #[arg(long, default_value_t = DEFAULT_MAX_MIB)]
        max_mib: u64,
    },
    RebuildEmbeddings {
        /// Local embedding model directory; repeat for an ensemble (1 to 3 models, blended equally)
        #[arg(long, required = true)]
        model: Vec<PathBuf>,
    },
    Search {
        query: String,
        /// Default: hybrid when the corpus has vectors from models installed in Footnote's models folder, otherwise lexical
        #[arg(long, value_parser = ["lexical", "hybrid"])]
        mode: Option<String>,
        /// Local embedding model directory, hybrid mode only; repeat for an ensemble (default: the corpus's installed models)
        #[arg(long)]
        model: Vec<PathBuf>,
        #[arg(long, default_value_t = 8)]
        limit: usize,
        #[arg(long)]
        technology: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        tested_version: Option<String>,
        #[arg(long)]
        status: Option<String>,
        /// At most this many chunks of one source file in the results
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=20))]
        per_source: Option<u8>,
    },
    Get {
        id: i64,
    },
    Propose {
        #[arg(long)]
        file: PathBuf,
        #[arg(long, required = true)]
        allow: Vec<PathBuf>,
    },
    Validate {
        id: i64,
        #[arg(long)]
        revision: i64,
        #[arg(long)]
        evidence: PathBuf,
        #[arg(long, required = true)]
        allow: Vec<PathBuf>,
    },
    GetRecord {
        id: i64,
    },
    Backup {
        #[arg(long)]
        output: PathBuf,
    },
    Stats,
    #[cfg(feature = "memory")]
    /// Store agent memories read as a JSON array on stdin (agent memory database only).
    Remember {
        /// Local embedding model directory; repeat for an ensemble. Without it, new memories are lexical-only until rebuild-embeddings.
        #[arg(long)]
        model: Vec<PathBuf>,
    },
    #[cfg(feature = "memory")]
    /// Remove an agent memory from recall (kept as history).
    Forget {
        id: i64,
    },
    #[cfg(feature = "memory")]
    /// Add or remove tags on an agent memory (at most 10; lowercase words such as rust, windows, bugfix, resolved).
    MemoryTag {
        id: i64,
        #[arg(long)]
        add: Vec<String>,
        #[arg(long)]
        remove: Vec<String>,
    },
    #[cfg(feature = "memory")]
    /// Rename a project scope of the agent memories (for example after a project rename).
    MemoryRescope {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    #[cfg(feature = "memory")]
    /// Session-start pack: global preferences, then this project's memories, within a character budget.
    MemoryPack {
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 3000)]
        max_chars: usize,
    },
    #[cfg(feature = "memory")]
    /// Agent memories relevant to a prompt: global ones and the given project's.
    MemoryRecall {
        query: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=20))]
        limit: u8,
        #[arg(long)]
        model: Vec<PathBuf>,
        /// Ask the resident daemon (models already loaded) instead of searching here; fails when none runs.
        #[arg(long)]
        resident: bool,
        /// Only memories sharing one of these tags, plus untagged ones (repeatable); narrows before ranking.
        #[arg(long)]
        tag: Vec<String>,
        /// Take the tags from the query's own words (the tag engine the hook uses when `infer_tags` is on).
        #[arg(long)]
        infer_tags: bool,
    },
    /// Serve the knowledge base to an agent; --mcp selects the MCP stdio transport.
    Serve {
        #[arg(long)]
        mcp: bool,
        /// Stay resident and serve every agent over a local socket instead of
        /// stdin/stdout. The `mcp` front-end spawns this automatically.
        #[arg(long)]
        daemon: bool,
        /// Append-only hash-chained log of every tool call. Refuses to serve if
        /// an existing log does not verify. Daemon mode defaults to a managed log.
        #[arg(long)]
        audit: Option<PathBuf>,
        /// Key for keyed HMAC integrity in the audit log (64 hex chars or >=32
        /// raw bytes). Requires --audit. Daemon mode manages this automatically.
        #[arg(long)]
        mac_key: Option<PathBuf>,
        /// Daemon mode: do not record an audit log.
        #[arg(long)]
        no_audit: bool,
        /// Daemon mode: exit after this many seconds with no session open (a daemon started on demand).
        #[arg(long)]
        idle_seconds: Option<u64>,
    },
    /// Stdio front-end for agents: use the shared daemon when it runs, otherwise
    /// serve in-process. Audit logging, MAC key and token are automatic.
    Mcp,
    /// Register this corpus as an MCP server in every detected agent.
    Install {
        #[arg(long)]
        dry_run: bool,
        /// Set up agent memory instead of a corpus: the memory database, its MCP server, the memory hook and
        /// /remember in Claude Code, Codex, Grok and DeepSeek Harness (no --db needed).
        #[arg(long)]
        memory: bool,
    },
    /// Remove Footnote Memory from every agent (hooks, MCP server, /remember). Memories and settings are kept.
    #[cfg(feature = "memory")]
    Uninstall {
        #[arg(long)]
        dry_run: bool,
        /// Required: what to remove (only Footnote Memory can be uninstalled this way today).
        #[arg(long, required = true)]
        memory: bool,
    },
    #[cfg(feature = "memory")]
    /// Agent memory hook: read an event as JSON on stdin, without Python (settings: <state dir>/memory.json).
    Hook {
        /// Background step: encode new memories with the configured models.
        #[arg(hide = true)]
        step: Option<HookStep>,
    },
    /// Vectors for a JSON array of texts on stdin, with the shipped recipe (benchmarks and checks; not needed for normal use).
    #[command(hide = true)]
    Embed {
        #[arg(long)]
        model: PathBuf,
        #[arg(long, value_parser = ["query", "passage"])]
        kind: String,
    },
    /// Summarize the metrics file: time per step, failures, steps that never finished (builds with --features metrics only).
    #[cfg(feature = "metrics")]
    Metrics {
        #[arg(long, default_value_t = 30)]
        top: usize,
    },
    /// Crawl a documentation site into Markdown files for `index` (llms.txt, sitemap, then links under the start URL).
    #[cfg(feature = "library")]
    Crawl {
        url: String,
        /// Folder for the pages (`<n>.md`) and `urls.json`.
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 1000)]
        max_pages: usize,
        /// Skip pages whose URL matches this regular expression (for example translated mirrors).
        #[arg(long)]
        exclude: Option<String>,
        /// Requests in flight at the start; by default the level this host reached last time, else 2. It then adapts.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=8))]
        concurrency: Option<u8>,
    },
    /// What is installed, on, registered and running, and what memory did this week.
    Status,
    /// Running and recent long steps (downloads, indexing) with their progress, whoever started them.
    Jobs,
    /// What memory did, day by day: memories saved, sessions and prompts given memories, the memories used most.
    #[cfg(feature = "memory")]
    Activity {
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u16).range(1..=90))]
        days: u16,
    },
    /// Browse agent memories, newest first (ids are those `forget` takes).
    #[cfg(feature = "memory")]
    MemoryList {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    /// Embedding models: list, install, use (for Footnote Memory), remove, recommend from this machine and corpus.
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },
    /// Every option with its value, where it comes from (default, you, company policy), and what it gains and costs.
    Options,
    /// Read or change an option (`footnote options` lists them).
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Footnote Alias (not released): personal-data identifiers in a JSON array of texts on stdin, by rules with check digits.
    #[cfg(feature = "alias")]
    AliasScan,
    /// Verify an append-only audit log (no database needed).
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
}
#[cfg(feature = "memory")]
#[derive(Clone, clap::ValueEnum)]
enum HookStep {
    Embed,
}
#[derive(Subcommand)]
enum ModelsAction {
    /// Known models, which are installed, which Footnote Memory uses.
    List,
    /// Download models (the only command that uses the network): pinned revisions, checked hashes.
    Install { names: Vec<String> },
    /// Make Footnote Memory use these installed models (one to three).
    Use { names: Vec<String> },
    /// Delete an installed model that nothing uses.
    Remove { name: String },
    /// Recommend models from this machine (CPU, RAM, GPU) and, with --db, the corpus language.
    Recommend,
}
#[derive(Subcommand)]
enum ConfigAction {
    /// One option's value, or every option without a key.
    Get { key: Option<String> },
    /// Set an option, for example `footnote config set memory.enabled false`.
    Set { key: String, value: String },
}
#[derive(Subcommand)]
pub enum AuditAction {
    /// Recompute the hash chain and report the first break, if any.
    Verify {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        mac_key: Option<PathBuf>,
        /// Pinned head hash. Detects tail truncation, which a bare hash chain
        /// cannot: without an out-of-band head, dropping the last line verifies.
        #[arg(long)]
        expect_last: Option<String>,
    },
}
fn require_db(db: &Option<PathBuf>) -> Result<&Path> {
    db.as_deref()
        .ok_or_else(|| anyhow::anyhow!("--db is required for this command"))
}

/// Daemon security defaults: an audit log and a managed MAC key, created on
/// demand, unless the operator opts out. No flag or key file has to be supplied.
fn managed_audit(
    disabled: bool,
    audit: &Option<PathBuf>,
    mac_key: &Option<PathBuf>,
) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    if disabled {
        return Ok((None, None));
    }
    match audit {
        Some(path) => Ok((Some(path.clone()), mac_key.clone())),
        None => {
            let key = crate::state::load_or_create_key()?;
            Ok((Some(crate::state::audit_path()?), Some(key)))
        }
    }
}
fn audit_command(action: &AuditAction) -> Result<Value> {
    match action {
        AuditAction::Verify {
            file,
            mac_key,
            expect_last,
        } => {
            let key = match mac_key.as_deref() {
                Some(path) => Some(crate::audit::load_key(path)?),
                // Default: this machine's managed key, so MACs are checked without a flag.
                None => match crate::state::key_path() {
                    Ok(path) if path.is_file() => Some(crate::audit::load_key(&path)?),
                    _ => None,
                },
            };
            let (entries, last_hash) = crate::audit::verify(file, key.as_deref())?;
            let head = match expect_last.as_deref() {
                Some(expected) if expected != last_hash => {
                    bail!(
                        "audit head mismatch: expected {expected}, found {last_hash} \
                         (log was truncated or extended)"
                    );
                }
                Some(_) => "matched",
                None => "not_checked",
            };
            Ok(json!({
                "entries": entries,
                "ok": true,
                "last_hash": last_hash,
                "mac": if key.is_some() { "verified" } else { "not_checked" },
                "head": head,
            }))
        }
    }
}
fn execute(cli: Cli) -> Result<Value> {
    // MCP stdio opens the database lazily, read-only, per request, so a missing
    // or not-yet-migrated database is reported as a tool error instead of
    // aborting the server before the client can call tools/list. Audit-log
    // verification is a pure file operation and also needs no database.
    match &cli.command {
        Action::Serve {
            mcp,
            daemon,
            audit,
            mac_key,
            no_audit,
            idle_seconds,
        } => {
            let db = require_db(&cli.db)?;
            if *daemon {
                let (audit_path, key_path) = managed_audit(*no_audit, audit, mac_key)?;
                let shared = crate::mcp::Shared::new(
                    db,
                    crate::mcp::open_audit(audit_path.as_deref(), key_path.as_deref())?,
                    Some(crate::state::load_or_create_token()?),
                );
                crate::daemon::run(shared, idle_seconds.map(std::time::Duration::from_secs))?;
                return Ok(Value::Null);
            }
            return crate::mcp::serve(db, *mcp, audit.as_deref(), mac_key.as_deref());
        }
        Action::Mcp => {
            let db = require_db(&cli.db)?;
            let (audit_path, key_path) = managed_audit(false, &None, &None)?;
            let shared = crate::mcp::Shared::new(
                db,
                crate::mcp::open_audit(audit_path.as_deref(), key_path.as_deref())?,
                None,
            );
            crate::daemon::proxy(&shared)?;
            return Ok(Value::Null);
        }
        Action::Install { dry_run, memory } => {
            return if *memory {
                #[cfg(feature = "memory")]
                return crate::install::memory(*dry_run);
                #[cfg(not(feature = "memory"))]
                bail!("this build has no Footnote Memory");
            } else {
                crate::install::run(&cli.db, *dry_run)
            };
        }
        Action::Audit { action } => return audit_command(action),
        #[cfg(feature = "alias")]
        Action::AliasScan => {
            let texts: Vec<String> = serde_json::from_reader(std::io::stdin().lock())?;
            let results: Vec<_> = texts.iter().map(|t| footnote_alias::scan(t)).collect();
            return Ok(json!({ "results": results }));
        }
        Action::Status => return crate::status::status(),
        #[cfg(feature = "library")]
        Action::Crawl {
            url,
            out,
            max_pages,
            exclude,
            concurrency,
        } => {
            // Each host's last concurrency level, so a site that slowed us down starts low next time.
            let hosts_file = crate::state::state_dir()?.join("crawl-hosts.json");
            let mut hosts: serde_json::Map<String, Value> = std::fs::read_to_string(&hosts_file)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let host = url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_string))
                .unwrap_or_default();
            let remembered = hosts.get(&host).and_then(Value::as_u64).map(|n| n as usize);
            let options = footnote_library::crawl::Options {
                start: url,
                max_pages: *max_pages,
                exclude: exclude.as_deref().map(regex::Regex::new).transpose()?,
                concurrency: concurrency.map(usize::from).or(remembered).unwrap_or(2),
            };
            let mut job = footnote_core::jobs::Job::start(
                &crate::state::state_dir()?,
                "library.crawl",
                url,
                "pages",
            )?;
            let done = footnote_library::crawl::crawl(out, &options, &mut |pages, _| {
                job.progress(pages, Some(*max_pages as u64))
            });
            job.finish(&done);
            if let Ok(v) = &done
                && let Some(level) = v["concurrency"].as_u64()
                && !host.is_empty()
            {
                hosts.insert(host, json!(level));
                std::fs::write(&hosts_file, serde_json::to_string_pretty(&hosts)?)?;
            }
            return done;
        }
        Action::Jobs => return footnote_core::jobs::list(&crate::state::state_dir()?),
        #[cfg(feature = "memory")]
        Action::Activity { days } => return crate::activity::activity(*days),
        Action::Options => return footnote_core::config::options(&crate::state::state_dir()?),
        Action::Models { action } => {
            let state = crate::state::state_dir()?;
            return match action {
                ModelsAction::List => crate::models::list(&state),
                ModelsAction::Install { names } => crate::models::install(&state, names),
                ModelsAction::Use { names } => crate::models::use_models(&state, names),
                ModelsAction::Remove { name } => crate::models::remove(&state, name),
                ModelsAction::Recommend => crate::models::recommend(cli.db.as_deref()),
            };
        }
        #[cfg(feature = "memory")]
        Action::Uninstall { dry_run, .. } => return crate::install::uninstall_memory(*dry_run),
        Action::Config { action } => {
            let state = crate::state::state_dir()?;
            return match action {
                ConfigAction::Get { key } => footnote_core::config::get(&state, key.as_deref()),
                ConfigAction::Set { key, value } => footnote_core::config::set(&state, key, value),
            };
        }
        Action::Embed { model, kind } => {
            let mut input = String::new();
            std::io::Read::read_to_string(
                &mut std::io::Read::take(std::io::stdin(), 64_000_000),
                &mut input,
            )?;
            let texts: Vec<String> = serde_json::from_str(&input)?;
            return crate::semantic::embed(model, &texts, kind);
        }
        #[cfg(feature = "metrics")]
        Action::Metrics { top } => return crate::metrics::summary(*top),
        _ => {}
    }
    let db = require_db(&cli.db)?;
    let mut c = open(db, writes(&cli.command))?;
    match cli.command {
        #[cfg(feature = "library")]
        Action::Index {
            root,
            allow,
            max_files,
            max_mib,
        } => {
            let mut job = footnote_core::jobs::Job::start(
                &crate::state::state_dir()?,
                "library.index",
                &root.to_string_lossy(),
                "files",
            )?;
            let indexed = index(
                &mut c,
                &root,
                &allow,
                max_files,
                max_mib,
                &mut |files, _| job.progress(files, None),
            );
            job.finish(&indexed);
            let mut report = indexed?;
            // Build the lexical index at index time; a failure only means searches fall back to FTS5.
            report["lexical_index"] =
                match crate::metrics::timed!("lexical.sync", crate::lexical::sync(&c)) {
                    Ok(Some(_)) => json!("ready"),
                    Ok(None) => json!("not_applicable"),
                    Err(e) => json!(format!("unavailable: {e:#}")),
                };
            Ok(report)
        }
        Action::RebuildEmbeddings { model } => crate::semantic::rebuild(&mut c, &model),
        Action::Search {
            query,
            mode,
            model,
            limit,
            technology,
            project,
            tested_version,
            status,
            per_source,
        } => {
            let filters = crate::records::Filters {
                technology,
                project,
                tested_version,
                status,
                per_source: per_source.map(usize::from),
                only: None,
            };
            let models = search_models(&c, mode.as_deref(), model)?;
            if models.is_empty() {
                search(&c, &query, limit, &filters)
            } else {
                crate::semantic::search(&c, &query, limit, &filters, &models)
            }
        }
        Action::Propose { file, allow } => crate::records::propose(&mut c, &file, &allow),
        Action::Validate {
            id,
            revision,
            evidence,
            allow,
        } => crate::records::validate(&mut c, id, revision, &evidence, &allow),
        Action::GetRecord { id } => crate::records::get(&c, id, true),
        Action::Get { id } => {
            Ok(c.query_row(&format!("{HIT} WHERE c.id=?1"), [id], |r| hit(r, true))?)
        }
        Action::Backup { output } => backup(&c, &output),
        Action::Stats => stats(&c),
        #[cfg(feature = "memory")]
        Action::Remember { model } => {
            let mut input = String::new();
            std::io::Read::read_to_string(
                &mut std::io::Read::take(std::io::stdin(), 512 * 1024),
                &mut input,
            )?;
            crate::memory::remember(&mut c, &serde_json::from_str(&input)?, &model)
        }
        #[cfg(feature = "memory")]
        Action::Forget { id } => crate::memory::forget(&mut c, id),
        #[cfg(feature = "memory")]
        Action::MemoryTag { id, add, remove } => crate::memory::retag(&mut c, id, &add, &remove),
        #[cfg(feature = "memory")]
        Action::MemoryRescope { from, to } => crate::memory::rescope(&mut c, &from, &to),
        #[cfg(feature = "memory")]
        Action::MemoryList {
            project,
            kind,
            limit,
            offset,
        } => crate::memory::list(&c, project.as_deref(), kind.as_deref(), limit, offset),
        #[cfg(feature = "memory")]
        Action::MemoryPack { project, max_chars } => {
            crate::memory::pack(&c, project.as_deref(), max_chars)
        }
        #[cfg(feature = "memory")]
        Action::MemoryRecall {
            query,
            project,
            limit,
            model,
            resident,
            mut tag,
            infer_tags,
        } => {
            if resident {
                let models: Vec<String> = model
                    .iter()
                    .map(|m| m.to_string_lossy().into_owned())
                    .collect();
                crate::daemon::call(
                    db,
                    "footnote_memory_recall",
                    &json!({"query": query, "project": project, "limit": limit, "tags": tag, "infer_tags": infer_tags, "model": if models.is_empty() { Value::Null } else { json!(models) }}),
                )
            } else {
                if infer_tags && tag.is_empty() {
                    tag = crate::memory::prompt_tags(&c, &query)?;
                }
                crate::memory::recall(&c, &query, project.as_deref(), limit.into(), &model, &tag)
            }
        }
        // Dispatched above, before the database is opened; MCP serves its
        // read-only tools by opening the database lazily per request.
        Action::Serve { .. } => unreachable!("serve is dispatched before the database is opened"),
        Action::Mcp => unreachable!("mcp is dispatched before the database is opened"),
        #[cfg(feature = "memory")]
        Action::Uninstall { .. } | Action::Activity { .. } => {
            unreachable!("dispatched before the database is opened")
        }
        #[cfg(feature = "library")]
        Action::Crawl { .. } => unreachable!("dispatched before the database is opened"),
        #[cfg(feature = "alias")]
        Action::AliasScan => unreachable!("dispatched before the database is opened"),
        Action::Audit { .. }
        | Action::Options
        | Action::Config { .. }
        | Action::Models { .. }
        | Action::Status
        | Action::Jobs => {
            unreachable!("dispatched before the database is opened")
        }
        #[cfg(feature = "memory")]
        Action::Hook { .. } => unreachable!("the hook is dispatched before the database is opened"),
        Action::Install { .. } | Action::Embed { .. } => {
            unreachable!("dispatched before the database is opened")
        }
        #[cfg(feature = "metrics")]
        Action::Metrics { .. } => {
            unreachable!("metrics is dispatched before the database is opened")
        }
    }
}
/// Commands that open the database for writing (and migrate it).
fn writes(a: &Action) -> bool {
    match a {
        #[cfg(feature = "library")]
        Action::Index { .. } => true,
        #[cfg(feature = "memory")]
        Action::Remember { .. }
        | Action::Forget { .. }
        | Action::MemoryTag { .. }
        | Action::MemoryRescope { .. } => true,
        Action::Propose { .. } | Action::Validate { .. } | Action::RebuildEmbeddings { .. } => true,
        _ => false,
    }
}
/// Step name of a command in the metrics file.
#[cfg(feature = "metrics")]
fn action_name(a: &Action) -> &'static str {
    match a {
        #[cfg(feature = "library")]
        Action::Index { .. } => "index",
        Action::RebuildEmbeddings { .. } => "rebuild-embeddings",
        Action::Search { mode, .. } => match mode.as_deref() {
            Some("hybrid") => "search-hybrid",
            Some(_) => "search-lexical",
            None => "search",
        },
        Action::Get { .. } => "get",
        Action::Propose { .. } => "propose",
        Action::Validate { .. } => "validate",
        Action::GetRecord { .. } => "get-record",
        Action::Backup { .. } => "backup",
        Action::Stats => "stats",
        #[cfg(feature = "memory")]
        Action::Remember { .. } => "remember",
        #[cfg(feature = "memory")]
        Action::Forget { .. } => "forget",
        #[cfg(feature = "memory")]
        Action::MemoryTag { .. } => "memory-tag",
        #[cfg(feature = "memory")]
        Action::MemoryRescope { .. } => "memory-rescope",
        #[cfg(feature = "memory")]
        Action::MemoryPack { .. } => "memory-pack",
        #[cfg(feature = "memory")]
        Action::MemoryRecall { .. } => "memory-recall",
        Action::Serve { .. } => "serve",
        Action::Mcp => "mcp",
        Action::Install { .. } => "install",
        #[cfg(feature = "memory")]
        Action::Hook { .. } => "hook",
        Action::Audit { .. } => "audit",
        #[cfg(feature = "alias")]
        Action::AliasScan => "alias-scan",
        Action::Options => "options",
        Action::Status => "status",
        #[cfg(feature = "library")]
        Action::Crawl { .. } => "crawl",
        Action::Jobs => "jobs",
        #[cfg(feature = "memory")]
        Action::Activity { .. } => "activity",
        #[cfg(feature = "memory")]
        Action::MemoryList { .. } => "memory-list",
        Action::Models { .. } => "models",
        #[cfg(feature = "memory")]
        Action::Uninstall { .. } => "uninstall",
        Action::Config { .. } => "config",
        Action::Embed { .. } => "embed",
        Action::Metrics { .. } => "metrics",
    }
}
/// Models for a search; empty means lexical. Hybrid is the default whenever the corpus has vectors from installed models
/// (it scored higher on every documentation and memory benchmark), so agents need not know model paths.
pub fn search_models(
    c: &rusqlite::Connection,
    mode: Option<&str>,
    model: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    let installed = || -> Vec<PathBuf> {
        let dir = crate::state::state_dir().map(|s| crate::models::models_dir(&s));
        let mut dirs: Vec<PathBuf> = dir
            .ok()
            .and_then(|d| std::fs::read_dir(d).ok())
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.join("manifest.json").is_file())
            .collect();
        dirs.sort();
        crate::semantic::embedded_with(c, dirs)
    };
    match (mode, model.is_empty()) {
        (Some("lexical"), false) => bail!("--model requires --mode hybrid"),
        (Some("lexical"), true) => Ok(vec![]),
        (Some("hybrid"), true) => {
            let found = installed();
            if found.is_empty() {
                bail!(
                    "hybrid needs this corpus's embedding models: pass --model, or install them with `footnote models install`"
                );
            }
            Ok(found)
        }
        (_, false) => Ok(model),
        (_, true) => Ok(installed()),
    }
}

pub fn run() {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e)
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            print!("{e}");
            return;
        }
        Err(e) => {
            println!("{}", json!({"error":e.to_string()}));
            std::process::exit(2);
        }
    };
    // Hook stdout belongs to the agent's event protocol. Failures must never block a session.
    #[cfg(feature = "memory")]
    if let Action::Hook { step } = &cli.command {
        crate::hook::run(step.is_some());
        return;
    }
    // In MCP mode stdout belongs to the JSON-RPC transport: a serve-level
    // failure is reported on stderr instead of the CLI's JSON envelope.
    let serving = matches!(cli.command, Action::Serve { .. } | Action::Mcp);
    let vectors = matches!(cli.command, Action::Embed { .. }); // bulk output by design: no response cap
    crate::metrics::started!(format!("cli.{}", action_name(&cli.command)));
    match crate::metrics::timed!(format!("cli.{}", action_name(&cli.command)), execute(cli)) {
        Ok(v) => {
            if serving {
                return;
            }
            let encoded = v.to_string();
            if encoded.len() > 256_000 && !vectors {
                println!(
                    "{}",
                    json!({"error":"response exceeds 256k byte limit; narrow search"})
                );
                std::process::exit(1);
            }
            println!("{encoded}");
        }
        Err(e) => {
            if serving {
                eprintln!("{}", json!({"error":e.to_string()}));
                std::process::exit(1);
            }
            println!("{}", json!({"error":e.to_string()}));
            std::process::exit(1);
        }
    }
}
