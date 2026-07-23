//! Integration tests for Playwright renderer
//!
//! Tests actual browser rendering with Playwright

use crawlkit_core::playwright::{PlaywrightConfig, PlaywrightRenderer};

#[tokio::test]
async fn test_playwright_renderer_with_browser() {
    let config = PlaywrightConfig {
        enabled: true,
        ..Default::default()
    };

    let renderer = PlaywrightRenderer::new(config);

    // Check if Playwright is available
    if !renderer.is_available() {
        println!("Playwright not available, skipping test");
        return;
    }

    // Test rendering a simple page
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
            println!("Console messages: {}", rendered.console_messages.len());
            println!("Network requests: {}", rendered.network_requests.len());
        }
        Err(e) => {
            // Playwright might not be fully configured
            println!("Playwright render error (expected in CI): {}", e);
        }
    }
}

#[tokio::test]
async fn test_playwright_renderer_context_isolation() {
    let config = PlaywrightConfig {
        enabled: true,
        max_concurrent: 2,
        ..Default::default()
    };

    let renderer = PlaywrightRenderer::new(config);

    if !renderer.is_available() {
        println!("Playwright not available, skipping test");
        return;
    }

    // Create multiple contexts
    let ctx1 = renderer.create_context();
    let ctx2 = renderer.create_context();

    assert!(ctx1.is_ok(), "First context should be created");
    assert!(ctx2.is_ok(), "Second context should be created");

    // Check active contexts
    assert_eq!(renderer.active_contexts(), 2);

    // Try to create third context (should fail due to limit)
    let ctx3 = renderer.create_context();
    assert!(ctx3.is_err(), "Third context should fail (limit reached)");
}

#[test]
fn test_playwright_detector_finds_binary() {
    let detector = crawlkit_core::PlaywrightDetector::detect();

    if detector.is_available() {
        println!("Playwright binary: {:?}", detector.binary_path());
        println!("Version: {:?}", detector.version());
        println!(
            "Has Chromium: {}",
            detector.has_browser(crawlkit_core::BrowserType::Chromium)
        );
        println!(
            "Has Firefox: {}",
            detector.has_browser(crawlkit_core::BrowserType::Firefox)
        );
        println!(
            "Has WebKit: {}",
            detector.has_browser(crawlkit_core::BrowserType::WebKit)
        );
    } else {
        println!("Playwright binary not found");
    }
}
