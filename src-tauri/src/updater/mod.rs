//! Auto-update orchestration: scheduler, events, install.
//!
//! The actual download + signature verification + install is handled by
//! `tauri-plugin-updater`; this module owns the policy (when to check,
//! what to emit, how to persist last-checked time) and the two Tauri
//! commands the frontend invokes.

mod persistence;
mod scheduler;
mod version;

use serde::Serialize;
use specta::Type;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UpdatePhase {
    Check,
    Download,
    Verify,
    Install,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum UpdateOutcome {
    UpToDate,
    Ready { version: String },
    Failed { phase: UpdatePhase, message: String },
}
