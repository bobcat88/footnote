#[cfg(feature = "memory")]
mod activity;
mod app;
mod daemon;
mod hardware;
#[cfg(feature = "memory")]
mod hook;
mod install;
mod mcp;
mod models;
#[cfg(all(feature = "encryption", windows))]
mod personal;
#[cfg(feature = "memory")]
mod projects;
mod status;
mod usage;

// Moved to library crates (ADR 0001); imported here so `crate::<module>` paths keep working.
use footnote_core::{audit, metrics, state};
#[cfg(feature = "memory")]
use footnote_memory as memory;
#[cfg(feature = "library")]
use footnote_search::lexical;
use footnote_search::{records, semantic};

fn main() {
    app::run();
}
