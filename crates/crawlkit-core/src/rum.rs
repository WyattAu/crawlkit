use serde::{Deserialize, Serialize};

/// RUM (Real User Monitoring) data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RumDataPoint {
    /// URL path.
    pub path: String,
    /// Largest Contentful Paint (ms).
    pub lcp: Option<f64>,
    /// Interaction to Next Paint (ms).
    pub inp: Option<f64>,
    /// Cumulative Layout Shift.
    pub cls: Option<f64>,
    /// First Contentful Paint (ms).
    pub fcp: Option<f64>,
    /// Time to First Byte (ms).
    pub ttfb: Option<f64>,
    /// Number of page views.
    pub page_views: u64,
    /// Collection period.
    pub period: String,
}

/// Google Analytics RUM adapter.
pub struct GoogleAnalyticsAdapter {
    /// GA4 property ID.
    property_id: Option<String>,
    /// API key for GA4 Data API.
    api_key: Option<String>,
}

impl GoogleAnalyticsAdapter {
    /// Create new GA adapter.
    #[must_use]
    pub fn new(property_id: Option<String>, api_key: Option<String>) -> Self {
        Self {
            property_id,
            api_key,
        }
    }

    /// Create from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            property_id: std::env::var("GA4_PROPERTY_ID").ok(),
            api_key: std::env::var("GA4_API_KEY").ok(),
        }
    }

    /// Check if adapter is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.property_id.is_some() && self.api_key.is_some()
    }

    /// Fetch RUM data for paths.
    ///
    /// # Errors
    /// Returns error if API call fails.
    pub async fn fetch_rum_data(&self, _paths: &[String]) -> Result<Vec<RumDataPoint>, RumError> {
        if !self.is_available() {
            return Err(RumError::NotConfigured);
        }
        // Placeholder: actual API call
        Ok(Vec::new())
    }
}

impl Default for GoogleAnalyticsAdapter {
    fn default() -> Self {
        Self::from_env()
    }
}

/// CrUX (Chrome User Experience Report) adapter.
pub struct CruxAdapter {
    /// API key for PageSpeed Insights API.
    api_key: Option<String>,
}

impl CruxAdapter {
    /// Create new CrUX adapter.
    #[must_use]
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    /// Create from environment variable.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("PAGESPEED_API_KEY").ok(),
        }
    }

    /// Check if adapter is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    /// Fetch CrUX data for a URL.
    ///
    /// # Errors
    /// Returns error if API call fails.
    pub async fn fetch_crux_data(&self, _url: &str) -> Result<Option<CruxData>, RumError> {
        if !self.is_available() {
            return Err(RumError::NotConfigured);
        }
        Ok(None)
    }
}

impl Default for CruxAdapter {
    fn default() -> Self {
        Self::from_env()
    }
}

/// CrUX data for a single URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CruxData {
    pub url: String,
    pub lcp_p75: Option<f64>,
    pub inp_p75: Option<f64>,
    pub cls_p75: Option<f64>,
    pub fcp_p75: Option<f64>,
    pub ttfb_p75: Option<f64>,
}

/// Merged lab + field data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedMetrics {
    pub url: String,
    pub lab: LabMetrics,
    pub field: Option<FieldMetrics>,
    pub deltas: MetricDeltas,
}

/// Lab (synthetic) metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LabMetrics {
    pub lcp: Option<f64>,
    pub fid: Option<f64>,
    pub cls: Option<f64>,
    pub ttfb: Option<f64>,
    pub fcp: Option<f64>,
    pub tti: Option<f64>,
}

/// Field (real user) metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldMetrics {
    pub lcp_p75: Option<f64>,
    pub inp_p75: Option<f64>,
    pub cls_p75: Option<f64>,
    pub fcp_p75: Option<f64>,
    pub ttfb_p75: Option<f64>,
}

/// Deltas between lab and field metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricDeltas {
    pub lcp_delta: Option<f64>,
    pub cls_delta: Option<f64>,
    pub fcp_delta: Option<f64>,
}

/// RUM errors.
#[derive(Debug, thiserror::Error)]
pub enum RumError {
    #[error("RUM not configured")]
    NotConfigured,

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("No data available for URL: {0}")]
    NoData(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ga_adapter_not_available() {
        let adapter = GoogleAnalyticsAdapter::new(None, None);
        assert!(!adapter.is_available());
    }

    #[test]
    fn test_crux_adapter_not_available() {
        let adapter = CruxAdapter::new(None);
        assert!(!adapter.is_available());
    }

    #[test]
    fn test_merged_metrics() {
        let metrics = MergedMetrics {
            url: "https://example.com".to_string(),
            lab: LabMetrics {
                lcp: Some(2500.0),
                ..Default::default()
            },
            field: Some(FieldMetrics {
                lcp_p75: Some(3000.0),
                ..Default::default()
            }),
            deltas: MetricDeltas {
                lcp_delta: Some(500.0),
                ..Default::default()
            },
        };
        assert_eq!(metrics.lab.lcp, Some(2500.0));
        assert_eq!(metrics.deltas.lcp_delta, Some(500.0));
    }
}
