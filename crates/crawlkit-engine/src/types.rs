//! Core types shared across all feature gates.
//!
//! `Severity`, `IssueCategory`, and `Finding` are re-exported from
//! `crawlkit-types` so existing import paths continue to work.

pub use crawlkit_types::{Finding, IssueCategory, Severity};
