//! Integration tests for backlink adapters
//!
//! Tests Ahrefs, Majestic, and GSC adapters
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crawlkit_core::backlink_adapters::{
    AhrefsAdapter, BacklinkAdapter, GscAdapter, MajesticAdapter,
};

#[tokio::test]
async fn test_ahrefs_adapter_api_key_required() {
    let adapter = AhrefsAdapter::new(None);
    assert!(!adapter.is_available());

    let result = adapter.fetch_backlinks("example.com", 10).await;
    assert!(result.is_err());

    let result = adapter.get_domain_rating("example.com").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_majestic_adapter_api_key_required() {
    let adapter = MajesticAdapter::new(None);
    assert!(!adapter.is_available());

    let result = adapter.fetch_backlinks("example.com", 10).await;
    assert!(result.is_err());

    let result = adapter.get_domain_rating("example.com").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_gsc_adapter_token_required() {
    let adapter = GscAdapter::new(None);
    assert!(!adapter.is_available());

    let result = adapter.fetch_backlinks("example.com", 10).await;
    assert!(result.is_err());

    let result = adapter.get_domain_rating("example.com").await;
    assert!(result.is_err());
}

#[test]
fn test_backlink_registry() {
    let registry = crawlkit_core::BacklinkAdapterRegistry::with_defaults();

    // Check that all adapters are registered
    assert!(registry.get("ahrefs").is_some());
    assert!(registry.get("majestic").is_some());
    assert!(registry.get("google_search_console").is_some());

    // Check that unavailable adapters are not in available list
    let available = registry.available();
    // All adapters require API keys, so none should be available
    assert!(available.is_empty());
}
