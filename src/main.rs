#[cfg(feature = "memory")]
mod activity;
mod app;
mod audit_prefix;
#[cfg(feature = "library")]
mod bundle;
mod daemon;
#[cfg(feature = "library")]
mod docs;
mod find;
#[cfg(feature = "code")]
mod frame;
mod grep;
mod hardware;
#[cfg(feature = "memory")]
mod hook;
mod install;
mod mcp;
#[cfg(feature = "library")]
mod miss;
mod models;
#[cfg(all(feature = "encryption", windows))]
mod personal;
#[cfg(feature = "memory")]
mod projects;
#[cfg(feature = "library")]
mod refresh;
#[cfg(feature = "memory")]
mod review;
mod run;
// Written by the Memory hook, read by `footnote skills stats` (Library).
#[cfg_attr(not(all(feature = "memory", feature = "library")), allow(dead_code))]
mod skill_outcomes;
#[cfg(feature = "library")]
// The prompt hook that suggests skills is Memory's; a Library-only build keeps just the index commands.
#[cfg_attr(not(feature = "memory"), allow(dead_code))]
mod skills;
#[cfg(feature = "code")]
mod stack;
mod status;
mod update;
mod usage;

// Moved to library crates (ADR 0001); imported here so `crate::<module>` paths keep working.
use footnote_core::{audit, metrics, state};
#[cfg(feature = "memory")]
use footnote_memory as memory;
#[cfg(feature = "library")]
use footnote_search::lexical;
use footnote_search::{records, semantic};

fn main() {
    // Windows gives the main thread 1 MB, which a debug build with every feature overflows (#265): the command runs on a
    // thread with 8 MB, the default on Linux. process::exit inside it still ends the process with its code.
    let worker = std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(app::run)
        .expect("start the command thread");
    if worker.join().is_err() {
        std::process::exit(101); // a panic, already reported by the default hook
    }
}
