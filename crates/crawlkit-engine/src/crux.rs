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

/// Default base URL for the CrUX API.
pub const CRUX_API_BASE: &str = "https://chromeuxreport.googleapis.com/v1";

/// Client for the Chrome User Experience Report (CrUX) API.
///
/// Uses the `records:queryRecord` endpoint to fetch real-world Core Web
/// Vitals field data for origins. The base URL is injectable
/// ([`CruxClient::with_base_url`]) so contract tests run against a hermetic
/// HTTP stub with no Google dependency in CI.
///
/// Token hygiene: the API key is sent in the `x-goog-api-key` header, not
/// the URL query string — reqwest error messages embed request URLs, so a
/// query-string key would leak into logs and error reports. A test pins
/// this property.
pub struct CruxClient {
    api_key: String,
    base_url: String,
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
        Self::with_base_url(api_key, CRUX_API_BASE.to_string())
    }

    /// Create a client against an alternate base URL (testing).
    #[must_use]
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
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

        let request_url = format!("{}/records:queryRecord", self.base_url);

        let body = serde_json::json!({
            "origin": origin,
        });

        let response = self
            .http_client
            .post(&request_url)
            .header("x-goog-api-key", &self.api_key)
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

    // ---- Error-path + token-hygiene contract (GSC promotion pattern) ----
    // A raw TcpListener stub serves one scripted HTTP response; no mock
    // crate, no Google dependency.

    /// Serve one HTTP response on 127.0.0.1:0 and return the base URL plus
    /// the captured request bytes.
    async fn serve_one(
        response: &'static str,
    ) -> (String, tokio::sync::oneshot::Receiver<Vec<u8>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let _ = tx.send(buf[..n].to_vec());
            sock.write_all(response.as_bytes()).await.ok();
        });
        (format!("http://{addr}/v1"), rx)
    }

    fn ok_response(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    const CRUX_OK_BODY: &str = r#"{"record":{"key":{"origin":"https://example.com"},"metrics":{"largest_contentful_paint":{"percentile":2500.0},"cumulative_layout_shift":{"percentile":0.05}}}}"#;

    #[tokio::test]
    async fn happy_path_parses_field_data() {
        let (base, _rx) = serve_one(Box::leak(ok_response(CRUX_OK_BODY).into_boxed_str())).await;
        let client = CruxClient::with_base_url("k-test".into(), base);
        let field = client
            .get_field_data("https://example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(field.lcp_p75, Some(2500.0));
        assert_eq!(field.cls_p75, Some(0.05));
    }

    #[tokio::test]
    async fn http_error_surfaces_status_and_body() {
        let (base, _rx) = serve_one(Box::leak(
            "HTTP/1.1 403 Forbidden\r\nContent-Length: 14\r\nConnection: close\r\n\r\npermission-den".to_string().into_boxed_str(),
        )).await;
        let client = CruxClient::with_base_url("k-test".into(), base);
        let err = client
            .get_field_data("https://example.com")
            .await
            .unwrap_err();
        let CruxError::ApiError { status, body } = err else {
            panic!("expected ApiError, got {err:?}");
        };
        assert_eq!(status, 403);
        assert!(body.contains("permission"));
    }

    #[tokio::test]
    async fn server_error_is_request_failed_not_panic() {
        let (base, _rx) = serve_one(Box::leak(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 3\r\nConnection: close\r\n\r\noops".to_string().into_boxed_str(),
        )).await;
        let client = CruxClient::with_base_url("k-test".into(), base);
        assert!(matches!(
            client.get_field_data("https://example.com").await,
            Err(CruxError::ApiError { status: 500, .. })
        ));
    }

    #[tokio::test]
    async fn malformed_json_is_invalid_response() {
        let (base, _rx) = serve_one(Box::leak(
            "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nnot json"
                .to_string()
                .into_boxed_str(),
        ))
        .await;
        let client = CruxClient::with_base_url("k-test".into(), base);
        assert!(matches!(
            client.get_field_data("https://example.com").await,
            Err(CruxError::InvalidResponse(_))
        ));
    }

    #[tokio::test]
    async fn connection_refused_maps_to_request_failed() {
        // Bind and immediately drop: the port is closed for the client.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let client = CruxClient::with_base_url("k-test".into(), format!("http://{addr}/v1"));
        assert!(matches!(
            client.get_field_data("https://example.com").await,
            Err(CruxError::RequestFailed(_))
        ));
    }

    #[tokio::test]
    async fn api_key_never_in_url_and_error_hides_it() {
        let key = "dummy-key";
        let (base, rx) = serve_one(Box::leak(ok_response(CRUX_OK_BODY).into_boxed_str())).await;
        let client = CruxClient::with_base_url(key.to_string(), base);
        let _ = client.get_field_data("https://example.com").await;
        let request = rx.await.unwrap();
        let request = String::from_utf8_lossy(&request);

        // Key travels in the header, never the request line.
        assert!(
            request.contains("x-goog-api-key: dummy-key"),
            "key must be sent as header"
        );
        assert!(
            !request.lines().next().unwrap().contains(key),
            "key leaked into request line"
        );

        // A transport error string embeds the URL; assert the key is absent
        // from the error type's Display for that path too.
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let caddr = closed.local_addr().unwrap();
        drop(closed);
        let c2 = CruxClient::with_base_url(key.to_string(), format!("http://{caddr}/v1"));
        if let Err(e) = c2.get_field_data("https://example.com").await {
            assert!(!e.to_string().contains(key), "key leaked into error: {e}");
        }
    }
}
