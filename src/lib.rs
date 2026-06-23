//! PlainLink's reusable core.
//!
//! The crate is intentionally small for the MVP: the URL cleaning engine is
//! platform-independent, while clipboard watching lives behind a thin adapter.

pub mod cleaner;
pub mod clipboard;
pub mod rules;

pub use cleaner::{CleanResult, RemovedParam, clean_url};
pub use clipboard::{WatchOptions, watch_clipboard};
pub use rules::{DomainRule, ParamPattern, RuleError, RuleSet};
