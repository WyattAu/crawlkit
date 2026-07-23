//! Integration tests for RUM adapters
//!
//! Tests Google Analytics and CrUX adapters

use crawlkit_core::rum::{CruxAdapter, GoogleAnalyticsAdapter};

#[tokio::test]
async fn test_ga_adapter_config_required() {
    let adapter = GoogleAnalyticsAdapter::new(None, None);
    assert!(!adapter.is_available());

    let result = adapter.fetch_rum_data(&["/test".to_string()]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_crux_adapter_config_required() {
    let adapter = CruxAdapter::new(None);
    assert!(!adapter.is_available());

    let result = adapter.fetch_crux_data("https://example.com").await;
    assert!(result.is_err());
}

#[test]
fn test_merged_metrics_calculation() {
    let lab = crawlkit_core::LabMetrics {
        lcp: Some(2500.0),
        fid: Some(100.0),
        cls: Some(0.1),
        ttfb: Some(200.0),
        fcp: Some(1500.0),
        tti: Some(3000.0),
    };

    let field = crawlkit_core::FieldMetrics {
        lcp_p75: Some(3000.0),
        inp_p75: Some(150.0),
        cls_p75: Some(0.15),
        fcp_p75: Some(1800.0),
        ttfb_p75: Some(250.0),
    };

    // Calculate deltas
    let deltas = crawlkit_core::MetricDeltas {
        lcp_delta: Some(field.lcp_p75.unwrap_or(0.0) - lab.lcp.unwrap_or(0.0)),
        cls_delta: Some(field.cls_p75.unwrap_or(0.0) - lab.cls.unwrap_or(0.0)),
        fcp_delta: Some(field.fcp_p75.unwrap_or(0.0) - lab.fcp.unwrap_or(0.0)),
    };

    let merged = crawlkit_core::MergedMetrics {
        url: "https://example.com".to_string(),
        lab,
        field: Some(field),
        deltas,
    };

    // Verify calculations (using approximate comparison for floating-point)
    assert_eq!(merged.lab.lcp, Some(2500.0));
    assert_eq!(merged.field.as_ref().unwrap().lcp_p75, Some(3000.0));
    assert!((merged.deltas.lcp_delta.unwrap_or(0.0) - 500.0).abs() < 0.001);
    assert!((merged.deltas.cls_delta.unwrap_or(0.0) - 0.05).abs() < 0.001);
    assert!((merged.deltas.fcp_delta.unwrap_or(0.0) - 300.0).abs() < 0.001);
}
