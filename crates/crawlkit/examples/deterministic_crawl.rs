//! Example: Deterministic crawl with seed-based reproducibility.
//!
//! Run with: cargo run --example deterministic_crawl

use crawlkit_core::determinism::DeterminismController;

fn main() {
    let seed = 42;

    let ctrl1 = DeterminismController::new(seed);
    let ctrl2 = DeterminismController::new(seed);

    println!("Seed: {}", seed);
    println!("Controller 1 seed: {}", ctrl1.seed());
    println!("Controller 2 seed: {}", ctrl2.seed());

    // Content hashing is deterministic
    let hash1 = DeterminismController::content_hash("https://example.com/page1");
    let hash2 = DeterminismController::content_hash("https://example.com/page1");
    assert_eq!(hash1, hash2);
    println!("Content hash deterministic: {}", hash1);

    // Different content produces different hashes
    let hash3 = DeterminismController::content_hash("https://example.com/page2");
    assert_ne!(hash1, hash3);
    println!("Different content, different hash: {}", hash3);

    // URL hashing
    let url_hash = DeterminismController::url_hash("https://example.com");
    println!("URL hash: {}", url_hash);

    // Derive seeds for different contexts (each call is unique due to counter)
    let seed1 = ctrl1.derive_seed("page1");
    let seed2 = ctrl1.derive_seed("page2");
    assert_ne!(seed1, seed2);
    println!("Derived seeds are unique: {} != {}", seed1, seed2);
}
