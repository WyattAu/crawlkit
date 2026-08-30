use serde::{Deserialize, Serialize};

/// Errors from the CrUX API client.
#[derive(Debug, thiserror::Error)]
pub enum CruxError {
    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("no data available for origin: {0}")]
    NoData(String),

    #[error("API error ({status}): {body}")]
    ApiError { status: u16, body: String },
}

/// Client for the Chrome User Experience Report (CrUX) API.
///
/// Uses the `https://chromeuxreport.googleapis.com/v1/records:queryRecord`
/// endpoint to fetch real-world Core Web Vitals field data for origins.
pub struct CruxClient {
    api_key: String,
    http_client: reqwest::Client,
}

/// CrUX field data for a single origin.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CruxFieldData {
    pub lcp_p75: Option<f64>,
    pub cls_p75: Option<f64>,
    pub inp_p75: Option<f64>,
    pub fcp_p75: Option<f64>,
    pub ttfb_p75: Option<f64>,
}

impl CruxClient {
    /// Create a new CrUX client with the given API key.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Create a client from the `CRUX_API_KEY` environment variable.
    /// Returns `None` if the env var is not set.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("CRUX_API_KEY").ok()?;
        if key.is_empty() {
            return None;
        }
        Some(Self::new(key))
    }

    /// Check if the client is configured with a valid API key.
    #[must_use]
    pub fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Fetch CrUX field data for an origin.
    ///
    /// Calls the CrUX API `queryRecord` endpoint with the origin
    /// (e.g. `https://example.com`). Returns `None` if no data is
    /// available for the origin.
    pub async fn get_field_data(&self, origin: &str) -> Result<Option<CruxFieldData>, CruxError> {
        if !self.is_available() {
            return Err(CruxError::RequestFailed(
                "no API key configured".to_string(),
            ));
        }

        let request_url = format!(
            "https://chromeuxreport.googleapis.com/v1/records:queryRecord?key={}",
            self.api_key,
        );

        let body = serde_json::json!({
            "origin": origin,
        });

        let response = self
            .http_client
            .post(&request_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CruxError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(CruxError::ApiError {
                status: status.as_u16(),
                body: text,
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CruxError::InvalidResponse(e.to_string()))?;

        Self::parse_field_data(&data)
    }

    /// Parse the CrUX API response into [`CruxFieldData`].
    fn parse_field_data(data: &serde_json::Value) -> Result<Option<CruxFieldData>, CruxError> {
        let record = match data.get("record") {
            Some(r) => r,
            None => return Ok(None),
        };

        let metrics = match record.get("metrics") {
            Some(m) => m,
            None => return Ok(None),
        };

        let extract_p75 = |metric_name: &str| -> Option<f64> {
            metrics.get(metric_name)?.get("percentile")?.as_f64()
        };

        let field_data = CruxFieldData {
            lcp_p75: extract_p75("largest_contentful_paint"),
            cls_p75: extract_p75("cumulative_layout_shift"),
            inp_p75: extract_p75("interaction_to_next_paint"),
            fcp_p75: extract_p75("first_contentful_paint"),
            ttfb_p75: extract_p75("experimental_time_to_first_byte"),
        };

        if field_data.lcp_p75.is_none()
            && field_data.cls_p75.is_none()
            && field_data.inp_p75.is_none()
        {
            return Ok(None);
        }

        Ok(Some(field_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crux_client_from_env_missing() {
        std::env::remove_var("CRUX_API_KEY");
        assert!(CruxClient::from_env().is_none());
    }

    #[test]
    fn test_crux_client_from_env_empty() {
        std::env::set_var("CRUX_API_KEY", "");
        assert!(CruxClient::from_env().is_none());
    }

    #[test]
    fn test_crux_client_from_env_present() {
        std::env::set_var("CRUX_API_KEY", "test-key-123");
        let client = CruxClient::from_env();
        assert!(client.is_some());
        assert!(client.unwrap().is_available());
    }

    #[test]
    fn test_crux_client_new() {
        let client = CruxClient::new("my-key".to_string());
        assert!(client.is_available());
    }

    #[test]
    fn test_parse_field_data_full() {
        let data = serde_json::json!({
            "record": {
                "key": { "origin": "https://example.com" },
                "metrics": {
                    "largest_contentful_paint": { "percentile": 2500.0 },
                    "cumulative_layout_shift": { "percentile": 0.05 },
                    "interaction_to_next_paint": { "percentile": 150.0 },
                    "first_contentful_paint": { "percentile": 800.0 },
                    "experimental_time_to_first_byte": { "percentile": 100.0 }
                }
            }
        });

        let result = CruxClient::parse_field_data(&data).unwrap();
        let field = result.unwrap();
        assert_eq!(field.lcp_p75, Some(2500.0));
        assert_eq!(field.cls_p75, Some(0.05));
        assert_eq!(field.inp_p75, Some(150.0));
        assert_eq!(field.fcp_p75, Some(800.0));
        assert_eq!(field.ttfb_p75, Some(100.0));
    }

    #[test]
    fn test_parse_field_data_partial() {
        let data = serde_json::json!({
            "record": {
                "metrics": {
                    "largest_contentful_paint": { "percentile": 3000.0 },
                    "cumulative_layout_shift": { "percentile": 0.1 }
                }
            }
        });

        let result = CruxClient::parse_field_data(&data).unwrap();
        let field = result.unwrap();
        assert_eq!(field.lcp_p75, Some(3000.0));
        assert_eq!(field.cls_p75, Some(0.1));
        assert!(field.inp_p75.is_none());
    }

    #[test]
    fn test_parse_field_data_no_record() {
        let data = serde_json::json!({});
        let result = CruxClient::parse_field_data(&data).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_field_data_no_metrics() {
        let data = serde_json::json!({
            "record": { "key": {} }
        });
        let result = CruxClient::parse_field_data(&data).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_field_data_empty_metrics() {
        let data = serde_json::json!({
            "record": {
                "metrics": {}
            }
        });
        let result = CruxClient::parse_field_data(&data).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_crux_field_data_default() {
        let data = CruxFieldData::default();
        assert!(data.lcp_p75.is_none());
        assert!(data.cls_p75.is_none());
        assert!(data.inp_p75.is_none());
    }

    #[test]
    fn test_crux_field_data_serialization() {
        let data = CruxFieldData {
            lcp_p75: Some(2000.0),
            cls_p75: Some(0.03),
            inp_p75: Some(100.0),
            fcp_p75: Some(500.0),
            ttfb_p75: Some(80.0),
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: CruxFieldData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.lcp_p75, Some(2000.0));
        assert_eq!(parsed.cls_p75, Some(0.03));
    }
}
