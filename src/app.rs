use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
// Engine pieces moved to crates (ADR 0001); re-exported so `crate::app::<item>` keeps working for the other modules.
pub(crate) use footnote_core::hash;
#[cfg(feature = "library")]
use footnote_library::index;
pub(crate) use footnote_search::store::{HIT, backup, hit, search, stats};

/// The personal store a database belongs to: a `memory.db` inside a folder that `footnote personal init` completed (its `ready`
/// marker and key id beside it). Every command opens through `open`, so the hook, MCP, daemon, status, review and find all read
/// an encrypted store the same way, with the key from the operating system's store (#128).
pub(crate) fn personal_dir(path: &Path) -> Option<&Path> {
    let dir = path.parent()?;
    (path.file_name().is_some_and(|n| n == "memory.db")
        && dir.join("ready").is_file()
        && dir.join("key.id").is_file())
    .then_some(dir)
}

/// A process that opens an encrypted store holds decrypted memory in its heap: Windows Error Reporting must leave the heap out of
/// any crash report it writes for this process (#173). Set once, for the life of the process; other processes are not affected.
#[cfg(all(feature = "encryption", windows))]
pub(crate) fn keep_heap_out_of_crash_reports() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        use windows_sys::Win32::System::ErrorReporting::{
            WER_FAULT_REPORTING_FLAG_NOHEAP, WerSetFlags,
        };
        // SAFETY: plain Win32 call with a constant flag; it affects only this process's future crash reports.
        unsafe { WerSetFlags(WER_FAULT_REPORTING_FLAG_NOHEAP) };
    });
}

/// Open a database: an encrypted personal store through its key, anything else as before.
pub fn open(path: &Path, write: bool) -> Result<rusqlite::Connection> {
    if let Some(dir) = personal_dir(path) {
        #[cfg(all(feature = "encryption", windows))]
        {
            keep_heap_out_of_crash_reports();
            return footnote_memory::personal_store::open_existing(dir, write);
        }
        #[cfg(not(all(feature = "encryption", windows)))]
        bail!(
            "{} is an encrypted personal store; this build of Footnote cannot open it (needs the encryption feature, Windows for now)",
            dir.display()
        );
    }
    footnote_search::store::open(path, write)
}

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
#[cfg(feature = "code")]
#[derive(Subcommand)]
enum StackWrite {
    /// Create the dependency file a project found only through its imports lacks. Shows the content and writes nothing
    /// unless `--yes` is given; never overwrites an existing file; no network.
    Write {
        /// Project folder.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Confirm: write the file.
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum Action {
    /// Experimental encrypted personal-store setup, selection and recovery.
    #[cfg(all(feature = "encryption", windows))]
    Personal {
        #[command(subcommand)]
        action: crate::personal::PersonalAction,
    },
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
    /// Bring back an agent memory retired by `forget` or replaced through `supersedes`.
    MemoryRestore {
        id: i64,
    },
    #[cfg(feature = "memory")]
    /// Memories worth a second look: near-duplicates, old session summaries, memories no prompt used. Changes nothing.
    MemoryReview {
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=200))]
        limit: u16,
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
        /// Install Footnote Lean PreToolUse hooks (`footnote hook lean`) across detected agents (#351).
        #[arg(long)]
        lean: bool,
        /// Disconnect external RTK hooks and MCP entries from detected agent configs (#351).
        #[arg(long)]
        remove_rtk: bool,
        /// Add the folder containing this `footnote` binary to the user's `PATH` (Windows User PATH or POSIX shell profile).
        #[arg(long)]
        add_to_path: bool,
    },
    /// Replace this binary with a newer one; the current one is kept beside it so `rollback` can put it back. Data in the state
    /// folder is not touched.
    Update {
        /// The new footnote binary (checked with `--version` before anything moves).
        #[arg(long)]
        from: PathBuf,
    },
    /// Put back the binary that the last `update` replaced.
    Rollback,
    /// Remove Footnote hooks/integrations from detected agents (`--memory` and/or `--lean`). Data and settings are kept.
    Uninstall {
        #[arg(long)]
        dry_run: bool,
        /// Remove Footnote Memory (hooks, MCP server, /remember).
        #[arg(long)]
        memory: bool,
        /// Remove Footnote Lean PreToolUse hooks only (leaves Memory untouched).
        #[arg(long)]
        lean: bool,
    },
    /// Agent hook processor: memory events (default) or Lean PreToolUse shell rewriter (`lean`).
    Hook {
        /// Sub-step: `embed` (background memory embedding) or `lean` (PreToolUse command rewriter).
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
    /// One shared documentation index per site and version on this machine (`<state>/docs/<site>/<version>.db`), found by
    /// Search. `add` crawls a site (or indexes a folder with `--from`) unless that site and version is already there.
    #[cfg(feature = "library")]
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },
    /// What is installed, on, registered and running, and what memory did this week.
    Status,
    /// Running and recent long steps (downloads, indexing) with their progress, whoever started them.
    Jobs,
    /// Projects agents worked in (recorded automatically at each session start): memories, code graph, uncommitted and
    /// unpushed git work.
    #[cfg(feature = "memory")]
    Projects,
    /// What memory did, day by day: memories saved, sessions and prompts given memories, the memories used most.
    #[cfg(feature = "memory")]
    Activity {
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u16).range(1..=90))]
        days: u16,
    },
    /// The Library corpora Search uses: list them, or add or remove one by its database path (`index` and `install` add
    /// theirs automatically; removing never deletes the database).
    Corpora {
        #[command(subcommand)]
        action: Option<CorporaAction>,
    },
    /// Search every product that is in this build and switched on (Memory, each Library corpus, every code graph); each
    /// product answers on its own, with its results or why it has none.
    Find {
        query: String,
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u16).range(1..=20))]
        limit: u16,
    },
    /// Tokens per day, agent and model from Claude Code and Codex session files on this computer, with what they would
    /// cost at list API prices. Counts only: no prompt or answer text is kept.
    Usage {
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u16).range(1..=365))]
        days: u16,
        /// Price one Claude Code or Codex session file instead (tokens per category, list price, per model).
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Footnote Lean savings ledger: shell output bytes saved, handle re-retrieval miss rate, MCP trimming, optional token attribution, and uncovered command discovery (#351, #381).
    Gain {
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u16).range(1..=365))]
        days: u16,
        /// Include session token and cache-bucket attribution from local agent transcripts.
        #[arg(long)]
        attrib: bool,
        /// Scan local agent transcripts (prefixes and byte counts only) to rank uncovered shell commands by raw output bytes (#381).
        #[arg(long)]
        missed: bool,
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
    /// Footnote Code: build the code graph of a project (Rust, TypeScript/JavaScript, Python, others by parser). Warning:
    /// for Rust, `rust-analyzer scip` builds the project, which runs its build scripts and procedural macros, so index
    /// only code you trust; without the compiler indexer, the parser graph runs no project code.
    #[cfg(feature = "code")]
    CodeIndex {
        /// Project folder.
        #[arg(default_value = ".")]
        root: PathBuf,
        /// Compiler-grade graph: check that the project's indexer (scip-typescript, scip-python, rust-analyzer) is installed and
        /// say how to install it if not. Footnote installs nothing unless you confirm (or pass --yes); a failing or oversized
        /// index is reported, and the parser graph is built instead.
        #[arg(long)]
        deep: bool,
        /// With --deep: run the install command without asking (for scripts).
        #[arg(long, requires = "deep", conflicts_with = "no_install")]
        yes: bool,
        /// With --deep: only report what is missing, never ask or install (the default when there is no terminal).
        #[arg(long, requires = "deep")]
        no_install: bool,
    },
    /// Footnote Code: the architecture map, blocks (folders) with symbol counts, links between them counted in references,
    /// and the outside packages each block uses.
    #[cfg(feature = "code")]
    CodeMap {
        /// Project folder indexed with `code-index`.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// How many folders deep a block goes (`crates/code` is 2).
        #[arg(long, default_value_t = 2)]
        depth: usize,
    },
    /// Footnote Code: a symbol's signature, place, callers and callees, from the compiler's index.
    #[cfg(feature = "code")]
    Code {
        name: String,
        /// Project folder indexed with `code-index`.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value_t = 30)]
        limit: usize,
    },
    /// Footnote Code: calls the graph could not link, in a file (its path) or inside a symbol, each with the project
    /// definitions sharing the callee's name (where to grep) or none (the callee is outside the project).
    #[cfg(feature = "code")]
    CodeUnresolved {
        /// A file path relative to the root, or the name of the function holding the calls.
        target: String,
        /// Project folder indexed with `code-index`.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Project framing: a readiness score (0-100) from weighted checks (stack, dependency file, cached docs, README or goal,
    /// test command, git) with the one next action for each, the questions still open and a verdict (ready 80+, almost 50-79,
    /// not yet below 50). `--set goal="..."` stores an answer in project memory; it counts next time. Local, no model.
    #[cfg(feature = "code")]
    Frame {
        /// Project folder.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// An answer to a framing question, `goal`, `users`, `constraints` or `tests`: `--set goal="a CLI that ..."`. Repeatable.
        #[arg(long = "set", value_name = "KEY=VALUE")]
        set: Vec<String>,
    },
    /// Footnote Code: the libraries a project uses, from its manifests and lockfiles, else from its imports, else one question.
    /// The result is recorded in Footnote Memory with exact versions where known; a re-run reports only the difference. Reads
    /// files only: no network. When the stack came only from imports the result suggests the missing dependency file;
    /// `footnote stack write` creates it.
    #[cfg(feature = "code")]
    #[command(args_conflicts_with_subcommands = true)]
    Stack {
        /// Project folder.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// What the project uses, when detection found nothing (repeatable): `cargo:serde@1`, `npm:react`, `pypi:flask`, `go:github.com/gin-gonic/gin`.
        #[arg(long)]
        library: Vec<String>,
        #[command(subcommand)]
        write: Option<StackWrite>,
    },
    /// Footnote Code: the impact radius of changing a symbol, its callers up to `depth` levels, the tests that reach it, the
    /// files and blocks affected, and the memories anchored to them.
    #[cfg(feature = "code")]
    CodeImpact {
        name: String,
        /// Project folder indexed with `code-index`.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value_t = 3)]
        depth: usize,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Fast native ripgrep-engine search (.gitignore-aware, symbol-annotated via Footnote Code, bounded output) (#382).
    Grep {
        /// Regular expression pattern (or literal with `--fixed-strings`).
        pattern: String,
        /// Folder or file to search (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Optional glob filter (e.g. `*.rs`, `crates/**/*.rs`).
        #[arg(short = 'g', long)]
        glob: Option<String>,
        /// Case-insensitive matching.
        #[arg(short = 'i', long)]
        ignore_case: bool,
        /// Treat pattern as a literal string instead of a regular expression.
        #[arg(short = 'F', long)]
        fixed_strings: bool,
        /// Max matching lines inline before spilling full output to `footnote output <id>`.
        #[arg(long, default_value_t = 80)]
        limit: usize,
        /// Emit structured JSON instead of compact file-grouped text.
        #[arg(long)]
        json: bool,
    },
    /// Verify an append-only audit log (no database needed).
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
    /// Measure Turn-1 fixed prompt overhead across instruction files, skills and MCP servers (#380).
    AuditPrefix {
        /// Project folder to inspect for repo-level instruction files (default: current directory).
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Limit scan to one agent (`all`, `claude`, `codex`, `gemini`, `antigravity`).
        #[arg(long, default_value = "all", value_parser = ["all", "claude", "codex", "gemini", "antigravity"])]
        agent: String,
        /// Opt-in: probe each configured stdio MCP server (`initialize` + `tools/list`) for exact schema bytes.
        #[arg(long)]
        probe_mcp: bool,
        /// Per-server probe timeout in seconds when `--probe-mcp` is set.
        #[arg(long, default_value_t = 5)]
        timeout_secs: u64,
    },
    /// Run a shell command with preservation-first output filtering and reversible local handle storage (#243).
    Run {
        /// Pass raw output through unchanged (bypass filtering and handle storage).
        #[arg(long)]
        raw: bool,
        /// Do not persist full raw output to the local handle store even when shortened.
        #[arg(long)]
        no_store: bool,
        /// Filter text read from stdin instead of spawning a command (for testing and benchmarks).
        #[arg(long, hide = true)]
        stdin: bool,
        /// Check whether a raw shell command is safely rewritable to `footnote run -- <cmd>` (exits 0 with rewritten command, else 1).
        #[arg(long)]
        rewrite: Option<String>,
        /// Override automatic command-family detection (`cargo-test`, `cargo-build`, `tests`, `git`, `search`, `linter`, `generic`).
        #[arg(long, value_parser = ["cargo-test", "cargo-build", "tests", "git", "search", "linter", "generic"])]
        family: Option<String>,
        /// Command and arguments to execute (after `--`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Inspect or search the full raw output stored by `footnote run` under a local handle id (#243).
    Output {
        /// Output handle id (e.g. `h-a1b2c3d4e5f6`).
        id: String,
        /// Keep only lines matching this regular expression (or substring).
        #[arg(long)]
        grep: Option<String>,
        /// Return at most the first N matching lines.
        #[arg(long, conflicts_with = "tail")]
        head: Option<usize>,
        /// Return at most the last N matching lines.
        #[arg(long, conflicts_with = "head")]
        tail: Option<usize>,
        /// 1-based inclusive line range `start:end` (or `start-end`).
        #[arg(long)]
        lines: Option<String>,
    },
}
#[derive(Clone, clap::ValueEnum)]
enum HookStep {
    #[cfg(feature = "memory")]
    Embed,
    Lean,
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
#[cfg(feature = "library")]
#[derive(Subcommand)]
enum DocsAction {
    /// Index a site's docs once per machine: reuse the cached database for that site and version, else crawl and index.
    Add {
        /// Documentation start page (omit with `--from`).
        url: Option<String>,
        /// Index this folder of Markdown files instead of crawling.
        #[arg(long)]
        from: Option<PathBuf>,
        /// Cache name; defaults to the start URL's host.
        #[arg(long)]
        site: Option<String>,
        /// Library version; defaults to `latest`. A new version is a new database; old ones are kept until `prune`.
        #[arg(long)]
        version: Option<String>,
        #[arg(long, default_value_t = 1000)]
        max_pages: usize,
        /// Embedding model folder for the background step; repeat for an ensemble (default: the models Footnote Memory uses).
        #[arg(long)]
        model: Vec<PathBuf>,
        /// Keyword index only: skip the background embedding.
        #[arg(long)]
        no_embed: bool,
    },
    /// Several libraries: each is searchable by keywords once indexed; embeddings follow in the background, largest last.
    Ingest {
        #[arg(required = true)]
        urls: Vec<String>,
        /// Project folder: libraries it imports most often are indexed and embedded first.
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long, default_value_t = 1000)]
        max_pages: usize,
        #[arg(long)]
        model: Vec<PathBuf>,
        #[arg(long)]
        no_embed: bool,
    },
    /// The background embedding worker that `add` and `ingest` start (one job per database).
    #[command(hide = true)]
    Embed {
        #[arg(long, required = true)]
        model: Vec<PathBuf>,
        dbs: Vec<PathBuf>,
    },
    /// Re-check cached documentation without crawling it again: a conditional request per page, then only changed pages are
    /// re-indexed (keyword index first, embeddings in the background), removed pages dropped, new pages added.
    Refresh {
        /// One library (site); default every cached entry.
        library: Option<String>,
        #[arg(long)]
        model: Vec<PathBuf>,
        /// Keyword index only: skip the background embedding.
        #[arg(long)]
        no_embed: bool,
    },
    /// Cached documentation: site, version, keyword-only / embedding n/m / ready, pages, size and date.
    List,
    /// Delete every version older than the newest of its site.
    Prune,
    /// Record the source licence of a cached library; `bundle` needs it.
    Licence {
        library: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        name: String,
        /// Whether the licence allows redistributing the indexed text (`true` or `false`).
        #[arg(long, action = clap::ArgAction::Set)]
        redistributable: bool,
        /// The docs site the pages came from.
        #[arg(long)]
        url: String,
    },
    /// Make a signing key for bundles (private key file, never overwritten); prints the public key.
    Keygen { file: PathBuf },
    /// Build a signed bundle of a cached library (refused without a recorded, redistributable licence).
    Bundle {
        library: String,
        #[arg(long)]
        version: Option<String>,
        /// Private key file from `docs keygen`.
        #[arg(long)]
        key: PathBuf,
        /// Output file; default `<library>-<version>.fnbundle`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Install a bundle (file or https URL) after checking its signature and hash.
    Install {
        source: String,
        /// Also trust this public key (hex), besides the key built into Footnote.
        #[arg(long)]
        trust_key: Vec<String>,
    },
}

#[derive(Subcommand)]
enum CorporaAction {
    /// Add a corpus database to the list.
    Add { db: PathBuf },
    /// Drop a corpus database from the list (the database itself stays).
    Remove { db: PathBuf },
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
        #[cfg(all(feature = "encryption", windows))]
        Action::Personal { .. } => unreachable!("personal actions are handled before JSON output"),
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
        Action::Install {
            dry_run,
            memory,
            lean,
            remove_rtk,
            add_to_path,
        } => {
            if *add_to_path && !*memory && !*lean && !*remove_rtk && cli.db.is_none() {
                return crate::install::add_to_path(*dry_run);
            }
            let mut out = if *memory && *lean {
                #[cfg(feature = "memory")]
                {
                    let mut m = crate::install::memory(*dry_run)?;
                    m["lean"] = crate::install::lean(*dry_run, *remove_rtk)?;
                    m
                }
                #[cfg(not(feature = "memory"))]
                bail!("this build has no Footnote Memory")
            } else if *lean {
                crate::install::lean(*dry_run, *remove_rtk)?
            } else if *remove_rtk && !*memory {
                crate::install::remove_rtk(*dry_run)?
            } else if *memory {
                #[cfg(feature = "memory")]
                {
                    crate::install::memory(*dry_run)?
                }
                #[cfg(not(feature = "memory"))]
                bail!("this build has no Footnote Memory")
            } else {
                crate::install::run(&cli.db, *dry_run)?
            };
            if *add_to_path {
                out["path"] = crate::install::add_to_path(*dry_run)?;
            }
            return Ok(out);
        }
        Action::Audit { action } => return audit_command(action),
        Action::Run { .. } => unreachable!("run is dispatched before JSON output"),
        Action::Output {
            id,
            grep,
            head,
            tail,
            lines,
        } => {
            return crate::run::output_cli(&crate::run::OutputQuery {
                id: id.clone(),
                grep: grep.clone(),
                head: *head,
                tail: *tail,
                lines: lines.clone(),
            });
        }
        #[cfg(feature = "code")]
        Action::CodeIndex {
            root,
            deep,
            yes,
            no_install,
        } => {
            use std::io::IsTerminal;
            let (abs, mut report) = (canonical(root)?, vec![]);
            if *deep {
                let mode = if *yes {
                    footnote_code::deep::Mode::Yes
                } else if *no_install || !std::io::stdin().is_terminal() {
                    footnote_code::deep::Mode::Report
                } else {
                    footnote_code::deep::Mode::Ask
                };
                report = footnote_code::deep::prepare(&abs, mode, &mut footnote_code::deep::System);
            }
            let mut out = footnote_code::index(&abs, &code_db(root)?)?;
            if *deep {
                out["deep"] = serde_json::json!({"indexers": report, "compiler_graph": out["origin"] == "compiler"});
            }
            return Ok(out);
        }
        #[cfg(feature = "code")]
        Action::CodeMap { root, depth } => {
            let mut map = footnote_code::map(&code_db(root)?, (*depth).max(1))?;
            crate::find::add_block_memories(&mut map);
            return Ok(map);
        }
        #[cfg(feature = "code")]
        Action::Stack {
            write: Some(StackWrite::Write { path, yes }),
            ..
        } => return crate::stack::write(path, *yes),
        #[cfg(feature = "code")]
        Action::Frame { path, set } => return crate::frame::run(path, set, true),
        #[cfg(feature = "code")]
        Action::Stack { path, library, .. } => return crate::stack::run(path, library, true),
        #[cfg(feature = "code")]
        Action::CodeImpact {
            name,
            root,
            depth,
            limit,
        } => return code_impact(root, name, *depth, *limit),
        #[cfg(feature = "code")]
        Action::CodeUnresolved {
            target,
            root,
            limit,
        } => return footnote_code::unresolved(&code_db(root)?, target, *limit),
        #[cfg(feature = "code")]
        Action::Code { name, root, limit } => {
            return code_lookup(root, name, *limit, false);
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
        Action::Projects => return crate::projects::list(&crate::state::state_dir()?),
        #[cfg(feature = "memory")]
        Action::Activity { days } => return crate::activity::activity(*days),
        #[cfg(feature = "memory")]
        Action::MemoryReview { limit } => return crate::review::review(usize::from(*limit)),
        Action::Usage { days, file } => {
            return match file {
                Some(f) => crate::usage::session(f),
                None => crate::usage::usage(*days),
            };
        }
        Action::Gain {
            days,
            attrib,
            missed,
        } => return crate::usage::gain(*days, *attrib, *missed),
        Action::Grep {
            pattern,
            path,
            glob,
            ignore_case,
            fixed_strings,
            limit,
            json,
        } => {
            return crate::grep::run(&crate::grep::Options {
                pattern,
                path,
                glob: glob.as_deref(),
                ignore_case: *ignore_case,
                fixed_strings: *fixed_strings,
                limit: *limit,
                json: *json,
            });
        }
        Action::AuditPrefix {
            root,
            agent,
            probe_mcp,
            timeout_secs,
        } => return crate::audit_prefix::run(root, agent, *probe_mcp, *timeout_secs),
        #[cfg(feature = "library")]
        Action::Docs { action } => {
            let state = crate::state::state_dir()?;
            return match action {
                DocsAction::Add {
                    url,
                    from,
                    site,
                    version,
                    max_pages,
                    model,
                    no_embed,
                } => crate::docs::add(
                    &state,
                    &crate::docs::Add {
                        url: url.as_deref(),
                        from: from.as_deref(),
                        site: site.as_deref(),
                        version: version.as_deref(),
                        max_pages: *max_pages,
                        models: model,
                        no_embed: *no_embed,
                    },
                ),
                DocsAction::Ingest {
                    urls,
                    project,
                    max_pages,
                    model,
                    no_embed,
                } => crate::docs::ingest(
                    &state,
                    urls,
                    &crate::docs::Add {
                        url: None,
                        from: None,
                        site: None,
                        version: None,
                        max_pages: *max_pages,
                        models: model,
                        no_embed: *no_embed,
                    },
                    project.as_deref(),
                ),
                DocsAction::Refresh {
                    library,
                    model,
                    no_embed,
                } => crate::refresh::run(
                    &state,
                    library.as_deref(),
                    &crate::refresh::TokenDiff::default(),
                    &crate::docs::Add {
                        url: None,
                        from: None,
                        site: None,
                        version: None,
                        max_pages: 0,
                        models: model,
                        no_embed: *no_embed,
                    },
                ),
                DocsAction::Embed { model, dbs } => crate::docs::embed(&state, dbs, model),
                DocsAction::List => crate::docs::list(&state),
                DocsAction::Prune => crate::docs::prune(&state),
                DocsAction::Licence {
                    library,
                    version,
                    name,
                    redistributable,
                    url,
                } => crate::bundle::licence(
                    &state,
                    library,
                    version.as_deref(),
                    name,
                    *redistributable,
                    url,
                ),
                DocsAction::Keygen { file } => crate::bundle::keygen(file),
                DocsAction::Bundle {
                    library,
                    version,
                    key,
                    out,
                } => {
                    crate::bundle::bundle(&state, library, version.as_deref(), key, out.as_deref())
                }
                DocsAction::Install { source, trust_key } => {
                    crate::bundle::install(&state, source, trust_key)
                }
            };
        }
        Action::Find { query, limit } => return crate::find::find(query, usize::from(*limit)),
        Action::Corpora { action } => {
            return crate::find::corpora_command(match action {
                None => None,
                Some(CorporaAction::Add { db }) => Some(("add", db.as_path())),
                Some(CorporaAction::Remove { db }) => Some(("remove", db.as_path())),
            });
        }
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
        Action::Uninstall {
            dry_run,
            memory,
            lean,
        } => {
            if !*memory && !*lean {
                bail!("pass `--memory` and/or `--lean` to `footnote uninstall`");
            }
            if *lean && !*memory {
                return crate::install::uninstall_lean(*dry_run);
            }
            if *memory && !*lean {
                #[cfg(feature = "memory")]
                return crate::install::uninstall_memory(*dry_run);
                #[cfg(not(feature = "memory"))]
                bail!("this build has no Footnote Memory");
            }
            let lean_out = crate::install::uninstall_lean(*dry_run)?;
            #[cfg(feature = "memory")]
            let mem_out = crate::install::uninstall_memory(*dry_run)?;
            #[cfg(not(feature = "memory"))]
            let mem_out = json!({"error": "this build has no Footnote Memory"});
            return Ok(json!({"dry_run": dry_run, "memory": mem_out, "lean": lean_out}));
        }
        Action::Update { from } => return crate::update::update(from),
        Action::Rollback => return crate::update::rollback(),
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
        #[cfg(all(feature = "encryption", windows))]
        Action::Personal { .. } => unreachable!("dispatched before the database is opened"),
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
            crate::find::record_corpus(db);
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
            crate::memory::remember(&mut c, &serde_json::from_str(&input)?, &model, "cli")
        }
        #[cfg(feature = "memory")]
        Action::Forget { id } => crate::memory::forget(&mut c, id),
        #[cfg(feature = "memory")]
        Action::MemoryRestore { id } => crate::memory::restore(&mut c, id),
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
        Action::Activity { .. } | Action::MemoryReview { .. } => {
            unreachable!("dispatched before the database is opened")
        }
        Action::Usage { .. } | Action::Find { .. } | Action::Corpora { .. } => {
            unreachable!("dispatched before the database is opened")
        }
        #[cfg(feature = "library")]
        Action::Crawl { .. } => unreachable!("dispatched before the database is opened"),
        #[cfg(feature = "library")]
        Action::Docs { .. } => unreachable!("dispatched before the database is opened"),
        #[cfg(feature = "code")]
        Action::CodeIndex { .. }
        | Action::Code { .. }
        | Action::CodeMap { .. }
        | Action::CodeImpact { .. }
        | Action::CodeUnresolved { .. }
        | Action::Stack { .. }
        | Action::Frame { .. } => {
            unreachable!("dispatched before the database is opened")
        }
        Action::Audit { .. }
        | Action::AuditPrefix { .. }
        | Action::Grep { .. }
        | Action::Options
        | Action::Config { .. }
        | Action::Models { .. }
        | Action::Status
        | Action::Jobs => {
            unreachable!("dispatched before the database is opened")
        }
        #[cfg(feature = "memory")]
        Action::Projects => unreachable!("dispatched before the database is opened"),
        Action::Hook { .. } => unreachable!("the hook is dispatched before the database is opened"),
        Action::Install { .. }
        | Action::Uninstall { .. }
        | Action::Gain { .. }
        | Action::Embed { .. }
        | Action::Update { .. }
        | Action::Rollback
        | Action::Run { .. }
        | Action::Output { .. } => {
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
        | Action::MemoryRestore { .. }
        | Action::MemoryTag { .. }
        | Action::MemoryRescope { .. } => true,
        Action::Propose { .. } | Action::Validate { .. } | Action::RebuildEmbeddings { .. } => true,
        _ => false,
    }
}
#[cfg(feature = "code")]
fn canonical(root: &Path) -> Result<PathBuf> {
    std::path::absolute(root).map_err(|e| anyhow::anyhow!("project folder {}: {e}", root.display()))
}

/// A symbol lookup plus what Memory and Library hold about it (three of each), when the symbol exists.
#[cfg(feature = "code")]
pub(crate) fn code_lookup(root: &Path, name: &str, limit: usize, body: bool) -> Result<Value> {
    let mut v = footnote_code::lookup(&code_db(root)?, name, limit)?;
    // `body`: each definition's own lines, so an agent reads one function instead of its whole file (#240).
    if body && let Some(list) = v["symbols"].as_array_mut() {
        for s in list {
            let (Some(path), Some(a), Some(b)) = (
                s["path"].as_str(),
                s["lines"][0].as_u64(),
                s["lines"][1].as_u64(),
            ) else {
                continue;
            };
            if let Ok(text) = std::fs::read_to_string(root.join(path)) {
                const MAX: usize = 150;
                let lines: Vec<&str> = text
                    .lines()
                    .skip(a as usize - 1)
                    .take((b - a + 1) as usize)
                    .collect();
                s["body"] = json!(
                    lines
                        .iter()
                        .take(MAX)
                        .copied()
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                if lines.len() > MAX {
                    s["body_cut_at"] = json!(MAX);
                }
            }
        }
    }
    if v["symbols"].as_array().is_some_and(|s| !s.is_empty()) {
        let files: Vec<String> = v["symbols"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s["path"].as_str().map(String::from))
            .collect();
        v["related"] = crate::find::related(name, &files, 3);
    }
    Ok(v)
}

/// The impact radius plus the memories anchored to the files it reaches ("last time this broke...").
#[cfg(feature = "code")]
pub(crate) fn code_impact(root: &Path, name: &str, depth: usize, limit: usize) -> Result<Value> {
    let mut v = footnote_code::impact(&code_db(root)?, name, depth, limit)?;
    let files: Vec<String> = v["found"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f["path"].as_str().map(String::from))
        .chain(
            v["files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f.as_str().map(String::from)),
        )
        .collect();
    v["memories"] = crate::find::related(name, &files, 5)["memory"].clone();
    Ok(v)
}

/// One graph per project, in the state folder (never inside the repository), named after the project's canonical path.
#[cfg(feature = "code")]
pub(crate) fn code_db(root: &Path) -> Result<PathBuf> {
    let key = footnote_core::hash(canonical(root)?.to_string_lossy().as_bytes());
    Ok(crate::state::state_dir()?
        .join("code")
        .join(format!("{}.db", &key[..16])))
}

/// Step name of a command in the metrics file.
#[cfg(feature = "metrics")]
fn action_name(a: &Action) -> &'static str {
    match a {
        #[cfg(all(feature = "encryption", windows))]
        Action::Personal { .. } => "personal",
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
        Action::MemoryRestore { .. } => "memory-restore",
        #[cfg(feature = "memory")]
        Action::MemoryReview { .. } => "memory-review",
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
        Action::Hook { .. } => "hook",
        Action::Audit { .. } => "audit",
        Action::Run { .. } => "run",
        Action::Output { .. } => "output",
        #[cfg(feature = "code")]
        Action::CodeIndex { .. } => "code-index",
        #[cfg(feature = "code")]
        Action::Code { .. } => "code",
        #[cfg(feature = "code")]
        Action::CodeImpact { .. } => "code-impact",
        #[cfg(feature = "code")]
        Action::CodeUnresolved { .. } => "code-unresolved",
        #[cfg(feature = "code")]
        Action::Stack { .. } => "stack",
        #[cfg(feature = "code")]
        Action::Frame { .. } => "frame",
        #[cfg(feature = "code")]
        Action::CodeMap { .. } => "code-map",
        Action::Options => "options",
        Action::Status => "status",
        #[cfg(feature = "library")]
        Action::Crawl { .. } => "crawl",
        Action::Jobs => "jobs",
        #[cfg(feature = "memory")]
        Action::Projects => "projects",
        #[cfg(feature = "memory")]
        Action::Activity { .. } => "activity",
        Action::Usage { .. } => "usage",
        Action::Gain { .. } => "gain",
        Action::Find { .. } => "find",
        Action::Corpora { .. } => "corpora",
        #[cfg(feature = "library")]
        Action::Docs { .. } => "docs",
        #[cfg(feature = "memory")]
        Action::MemoryList { .. } => "memory-list",
        Action::Models { .. } => "models",
        Action::Uninstall { .. } => "uninstall",
        Action::Config { .. } => "config",
        Action::Update { .. } => "update",
        Action::Rollback => "rollback",
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
    #[cfg(all(feature = "encryption", windows))]
    if let Action::Personal { action } = &cli.command {
        if let Err(e) = crate::personal::run(action) {
            eprintln!("{e:#}");
            std::process::exit(1);
        }
        return;
    }
    // Hook stdout belongs to the agent's event protocol. Failures must never block a session.
    if let Action::Hook { step } = &cli.command {
        if matches!(step, Some(HookStep::Lean)) {
            crate::run::run_lean_hook();
            return;
        }
        #[cfg(feature = "memory")]
        {
            crate::hook::run(matches!(step, Some(HookStep::Embed)));
        }
        return;
    }
    // `footnote run` emits the filtered/raw command output directly and preserves the exact child exit code.
    if let Action::Run {
        raw,
        no_store,
        stdin,
        rewrite,
        family,
        command,
    } = &cli.command
    {
        let code = crate::run::run_cli(&crate::run::RunOptions {
            raw: *raw,
            no_store: *no_store,
            stdin_mode: *stdin,
            rewrite: rewrite.clone(),
            family_override: family
                .as_deref()
                .and_then(crate::run::CommandFamily::parse_override),
            command: command.clone(),
        });
        if code != 0 {
            std::process::exit(code);
        }
        return;
    }
    // In MCP mode stdout belongs to the JSON-RPC transport: a serve-level
    // failure is reported on stderr instead of the CLI's JSON envelope.
    let serving = matches!(cli.command, Action::Serve { .. } | Action::Mcp);
    let raw_stdout = serving || matches!(cli.command, Action::Grep { json: false, .. });
    let vectors = matches!(cli.command, Action::Embed { .. }); // bulk output by design: no response cap
    crate::metrics::started!(format!("cli.{}", action_name(&cli.command)));
    match crate::metrics::timed!(format!("cli.{}", action_name(&cli.command)), execute(cli)) {
        Ok(v) => {
            if raw_stdout {
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

#[cfg(all(test, feature = "encryption", windows))]
mod crash_report_tests {
    #[test]
    fn opening_an_encrypted_store_keeps_the_heap_out_of_crash_reports() {
        use windows_sys::Win32::System::ErrorReporting::{
            WER_FAULT_REPORTING_FLAG_NOHEAP, WerGetFlags,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        super::keep_heap_out_of_crash_reports();
        let mut flags = 0;
        // SAFETY: reads this process's own WER flags into a local.
        let hr = unsafe { WerGetFlags(GetCurrentProcess(), &mut flags) };
        assert_eq!(hr, 0);
        assert_ne!(flags & WER_FAULT_REPORTING_FLAG_NOHEAP, 0);
    }
}
