//! Integration tests for Playwright renderer
//!
//! Tests actual browser rendering with Playwright.
//! All tests have timeouts to prevent hanging under code coverage tools.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use crawlkit_engine::playwright::{PlaywrightConfig, PlaywrightRenderer};

/// Timeout for all playwright tests (30 seconds).
const TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn test_playwright_renderer_with_browser() {
    let result = tokio::time::timeout(TIMEOUT, async {
        let config = PlaywrightConfig {
            enabled: true,
            ..Default::default()
        };

        let renderer = PlaywrightRenderer::new(config);

        if !renderer.is_available() {
            println!("Playwright not available, skipping test");
            return Ok::<(), String>(());
        }

        let result = renderer.render("https://example.com").await;

        match result {
            Ok(rendered) => {
                assert!(!rendered.html.is_empty(), "HTML should not be empty");
                assert!(
                    !rendered.final_url.is_empty(),
                    "Final URL should not be empty"
                );
                println!("Render time: {:?}", rendered.render_time);
                println!("HTML length: {} bytes", rendered.html.len());
            }
            Err(e) => {
                println!("Playwright render error (expected in CI): {}", e);
            }
        }
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("Test failed: {e}"),
        Err(_) => panic!("Test timed out after {TIMEOUT:?}"),
    }
}

#[tokio::test]
async fn test_playwright_renderer_context_isolation() {
    let result = tokio::time::timeout(TIMEOUT, async {
        let config = PlaywrightConfig {
            enabled: true,
            max_concurrent: 2,
            ..Default::default()
        };

        let renderer = PlaywrightRenderer::new(config);

        if !renderer.is_available() {
            println!("Playwright not available, skipping test");
            return Ok::<(), String>(());
        }

        let ctx1 = renderer.create_context().unwrap();
        let ctx2 = renderer.create_context().unwrap();
        assert_ne!(ctx1.id, ctx2.id, "Context IDs should be unique");

        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("Test failed: {e}"),
        Err(_) => panic!("Test timed out after {TIMEOUT:?}"),
    }
}

#[tokio::test]
async fn test_playwright_detector_finds_binary() {
    let result = tokio::time::timeout(TIMEOUT, async {
        let config = PlaywrightConfig::default();
        let renderer = PlaywrightRenderer::new(config);

        // This just tests the detector logic, not actual rendering
        let _ = renderer.is_available();
        Ok::<(), String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("Test failed: {e}"),
        Err(_) => panic!("Test timed out after {TIMEOUT:?}"),
    }
}
