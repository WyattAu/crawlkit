//! Category-based modules for the V2 analyzer set.

//! This module is a pure structural split of the former `v2_analyzers.rs`;
//! every type is re-exported so the public API is unchanged.

pub mod content;
pub mod security;
pub mod accessibility;
pub mod seo;
pub mod schema;
pub mod performance;
pub mod scoring;

pub use content::*;
pub use security::*;
pub use accessibility::*;
pub use seo::*;
pub use schema::*;
pub use performance::*;
pub use scoring::*;
