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
    pub async fn fetch_rum_data(&self, paths: &[String]) -> Result<Vec<RumDataPoint>, RumError> {
        if !self.is_available() {
            return Err(RumError::NotConfigured);
        }

        let property_id = self.property_id.as_ref().unwrap();
        let api_key = self.api_key.as_ref().unwrap();
        let client = reqwest::Client::new();

        let mut results = Vec::new();

        for path in paths {
            let url = format!(
                "https://analyticsdata.googleapis.com/v1beta/properties/{}:runReport?key={}",
                property_id, api_key
            );

            let body = serde_json::json!({
                "dateRanges": [{ "startDate": "30daysAgo", "endDate": "today" }],
                "dimensions": [{ "name": "pagePath" }],
                "metrics": [
                    { "name": "averageLargestContentfulPaint" },
                    { "name": "averageCumulativeLayoutShift" },
                    { "name": "averageFirstContentfulPaint" },
                    { "name": "averageTimeToFirstByte" },
                    { "name": "screenPageViews" }
                ],
                "dimensionFilter": {
                    "filter": {
                        "fieldName": "pagePath",
                        "stringFilter": { "matchType": "EXACT", "value": path }
                    }
                }
            });

            let response = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| RumError::RequestFailed(e.to_string()))?;

            if !response.status().is_success() {
                return Err(RumError::RequestFailed(format!(
                    "HTTP {}",
                    response.status()
                )));
            }

            let data: serde_json::Value = response
                .json()
                .await
                .map_err(|e| RumError::RequestFailed(e.to_string()))?;

            if let Some(rows) = data["rows"].as_array() {
                for row in rows {
                    let empty_vec = vec![];
                    let metric_values = row["metricValues"].as_array().unwrap_or(&empty_vec);
                    results.push(RumDataPoint {
                        path: path.clone(),
                        lcp: metric_values
                            .get(0)
                            .and_then(|v| v["value"].as_str())
                            .and_then(|s| s.parse().ok()),
                        inp: None,
                        cls: metric_values
                            .get(1)
                            .and_then(|v| v["value"].as_str())
                            .and_then(|s| s.parse().ok()),
                        fcp: metric_values
                            .get(2)
                            .and_then(|v| v["value"].as_str())
                            .and_then(|s| s.parse().ok()),
                        ttfb: metric_values
                            .get(3)
                            .and_then(|v| v["value"].as_str())
                            .and_then(|s| s.parse().ok()),
                        page_views: metric_values
                            .get(4)
                            .and_then(|v| v["value"].as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0),
                        period: "30d".to_string(),
                    });
                }
            }
        }

        Ok(results)
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
    pub async fn fetch_crux_data(&self, url: &str) -> Result<Option<CruxData>, RumError> {
        if !self.is_available() {
            return Err(RumError::NotConfigured);
        }

        let api_key = self.api_key.as_ref().unwrap();
        let client = reqwest::Client::new();

        let request_url = format!(
            "https://www.googleapis.com/pagespeedonline/v5/runPagespeed?url={}&key={}&strategy=mobile",
            urlencoding::encode(url),
            api_key
        );

        let response = client
            .get(&request_url)
            .send()
            .await
            .map_err(|e| RumError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RumError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RumError::RequestFailed(e.to_string()))?;

        // Extract CrUX data from lighthouse result
        let lighthouse = &data["lighthouseResult"]["audits"];

        let crux_data = CruxData {
            url: url.to_string(),
            lcp_p75: lighthouse["largest-contentful-paint"]["numericValue"].as_f64(),
            inp_p75: None, // Not directly available in PSI
            cls_p75: lighthouse["cumulative-layout-shift"]["numericValue"].as_f64(),
            fcp_p75: lighthouse["first-contentful-paint"]["numericValue"].as_f64(),
            ttfb_p75: lighthouse["server-response-time"]["numericValue"].as_f64(),
        };

        Ok(Some(crux_data))
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
