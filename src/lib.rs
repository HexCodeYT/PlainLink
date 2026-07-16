//! PlainLink's reusable core.
//!
//! The crate is intentionally small for the MVP: the URL cleaning engine is
//! platform-independent, while clipboard watching lives behind a thin adapter.

pub mod cleaner;
pub mod clipboard;
pub mod rules;
pub mod state;

pub use cleaner::{CleanResult, RemovedParam, clean_url};
pub use clipboard::{WatchOptions, watch_clipboard, write_clipboard_text};
pub use rules::{DomainRule, ParamPattern, RuleError, RuleSet};
pub use state::{LastCleaned, read_last_cleaned, save_last_cleaned};
