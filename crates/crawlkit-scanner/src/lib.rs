//! Hosted free-scan scanner service (ADR-012).
//!
//! Public surface: the hardened target guard ([`guard`]), ADR-012 bounds and
//! politeness budget ([`bounds`]), the bounded scan engine ([`scan`]), and
//! result tokens ([`token`]). The binary (`main.rs`) exposes an axum HTTP
//! API: submit a URL, poll for the result by unguessable token.

// Test code may unwrap/expect/panic per workspace convention (mirrors
// crates/crawlkit-api and crates/crawlkit).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod api;
pub mod bounds;
pub mod fetcher;
pub mod guard;
pub mod scan;
pub mod token;

pub use bounds::{BudgetError, BudgetStore, InMemoryBudgetStore, MAX_PAGES, RESULT_RETENTION};
pub use fetcher::PinnedFetcher;
pub use guard::{validate_target, GuardError};
pub use scan::{run_scan, Fetch, ScanDeps, ScanOutcome};
pub use token::{new_result_token, TokenError};
