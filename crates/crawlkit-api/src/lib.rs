//! crawlkit REST API server library.
//!
//! The crate is structured as a library plus a thin `crawlkit-api` binary so
//! that the router, middleware, and handlers can be exercised end-to-end by
//! integration tests without spawning a process.
//!
//! Interactive OpenAPI documentation is served at `GET /api/v1/docs` with
//! the raw document at `GET /api/v1/openapi.json`; both are public by
//! default and can be disabled with `DOCS_PUBLIC=false`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod auth;
pub mod auth_mw;
pub mod handlers;
pub mod middleware;
pub mod oidc;
pub mod openapi;
pub mod persistence;
pub mod router;
pub mod types;
