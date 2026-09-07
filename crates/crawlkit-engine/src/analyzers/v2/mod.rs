//! Category-based modules for the V2 analyzer set.

//! This module is a pure structural split of the former `v2_analyzers.rs`;
//! every type is re-exported so the public API is unchanged.

pub mod accessibility;
pub mod content;
pub mod performance;
pub mod schema;
pub mod scoring;
pub mod security;
pub mod seo;

pub use accessibility::*;
pub use content::*;
pub use performance::*;
pub use schema::*;
pub use scoring::*;
pub use security::*;
pub use seo::*;
