//! PlainLink's reusable core.
//!
//! The crate is intentionally small for the MVP: the URL cleaning engine is
//! platform-independent, while clipboard watching lives behind a thin adapter.

pub mod agent;
pub mod cleaner;
pub mod clipboard;
pub mod install;
pub mod rules;
pub mod state;

pub use agent::{
    AgentInstallOptions, AgentStatus, agent_status, install_agent, restart_agent, uninstall_agent,
};
pub use cleaner::{CleanResult, RemovedParam, clean_url};
pub use clipboard::{WatchOptions, watch_clipboard, write_clipboard_text};
pub use install::{
    DoctorCheck, DoctorStatus, InstallOptions, InstallReport, UninstallOptions, UninstallReport,
    install_plainlink, run_doctor, uninstall_plainlink,
};
pub use rules::{DomainRule, ParamPattern, RuleError, RuleSet};
pub use state::{LastCleaned, read_last_cleaned, save_last_cleaned};
