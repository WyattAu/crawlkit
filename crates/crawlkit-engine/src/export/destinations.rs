//! Warehouse destination clients (6.0.0-alpha.1) — upload the ADR-016
//! export artifacts to BigQuery, Snowflake, or S3.
//!
//! Design constraints, per ADR-016 §2.4 ("manifests are data; contract gaps
//! are build bugs") and the 5.3.0 connector posture:
//!
//! 1. **One transport seam.** Every HTTP call goes through [`Transport`], so
//!    contract tests observe exact request bytes (SigV4 canonical forms, load
//!    bodies) without network access, and live tests swap in a real
//!    transport. The clients never construct a TLS stack themselves.
//! 2. **Idempotency over the wire.** Re-running an upload of the same
//!    export overwrites the same destination keys (S3 PUT is idempotent; BQ
//!    loads default to `WRITE_TRUNCATE` per table; Snowflake `COPY` into a
//!    keyed table is re-runnable) — matching the byte-identical re-export
//!    guarantee of the exporter.
//! 3. **Credentials are operator material.** They exist only as strings
//!    handed to the signer; they are never logged, never embedded in errors,
//!    never echoed in metrics (5.3.0 §1 hardening posture, reused here).
//! 4. **Failures are classified, not swallowed.** Transport errors and
//!    5xx/429 are retryable; other 4xx are fatal — the same classification
//!    the alert-channel and webhook delivery pipelines use, so a shared
//!    failure-metric surface can key on it later.
//!
//! What this module is *not*: a general-purpose cloud SDK. Only the load
//! path the alpha.1 exit criteria need is implemented.

use std::fmt;

// ---------------------------------------------------------------------------
// Transport seam
// ---------------------------------------------------------------------------

/// One outbound HTTP request, destination-agnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    /// Header names lowercased. `host` and `content-type` are set by the
    /// client, never by the caller.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// One HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    /// Header names lowercased by the transport (HTTP/2 canonical form).
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Delivery-outcome classification shared with the alert/webhook
    /// pipelines: transport errors and 5xx/429 retryable, other 4xx fatal.
    pub fn outcome(&self) -> Result<(), LoadError> {
        match self.status {
            200..=299 => Ok(()),
            408 | 429 => Err(LoadError::Retryable {
                status: self.status,
            }),
            400..=499 => Err(LoadError::Fatal {
                status: self.status,
                detail: String::from_utf8_lossy(&self.body).into_owned(),
            }),
            _ => Err(LoadError::Retryable {
                status: self.status,
            }),
        }
    }
}

/// Errors from destination loads.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    /// 5xx / 429 / 408 — safe to retry the same request.
    Retryable { status: u16 },
    /// 4xx (other) — a contract or credential problem; retrying cannot fix it.
    Fatal { status: u16, detail: String },
    /// The destination client rejected the input before any HTTP call.
    InvalidInput(String),
    /// The transport itself failed (DNS, TLS, timeout).
    Transport(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Retryable { status } => write!(f, "retryable load failure (HTTP {status})"),
            LoadError::Fatal { status, detail } => {
                write!(f, "load failed (HTTP {status}): {detail}")
            }
            LoadError::InvalidInput(msg) => write!(f, "invalid load input: {msg}"),
            LoadError::Transport(msg) => write!(f, "transport error: {msg}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// The HTTP seam. Production wires [`reqwest_transport`]; tests wire an
/// in-memory recorder.
pub trait Transport: Send + Sync {
    fn send(
        &self,
        req: HttpRequest,
    ) -> impl std::future::Future<Output = Result<HttpResponse, LoadError>> + Send;
}

/// reqwest-backed transport (production). Feature-gated at the call site in
/// the CLI; the engine itself does not require `reqwest` for tests.
pub mod reqwest_transport {
    use super::{HttpRequest, HttpResponse, LoadError, Transport};

    /// Uses a client configured without default features (rustls) as the
    /// rest of the workspace does. Build once and pass with [`Self::with_client`]
    /// in production; the default constructor creates a fresh client per
    /// transport (acceptable for CLI one-shot uploads).
    #[derive(Clone, Default)]
    pub struct ReqwestTransport {
        client: Option<reqwest::Client>,
    }

    impl ReqwestTransport {
        pub fn new() -> Self {
            Self { client: None }
        }

        fn client(&self) -> Result<reqwest::Client, LoadError> {
            match &self.client {
                Some(c) => Ok(c.clone()),
                None => reqwest::Client::builder()
                    .build()
                    .map_err(|e| LoadError::Transport(e.to_string())),
            }
        }

        /// Provide a shared client (production path).
        pub fn with_client(client: reqwest::Client) -> Self {
            Self {
                client: Some(client),
            }
        }
    }

    impl Transport for ReqwestTransport {
        async fn send(&self, req: HttpRequest) -> Result<HttpResponse, LoadError> {
            let client = self.client()?;
            let method = match req.method {
                "GET" => reqwest::Method::GET,
                "POST" => reqwest::Method::POST,
                "PUT" => reqwest::Method::PUT,
                _ => return Err(LoadError::InvalidInput("unsupported method".into())),
            };
            let mut out = client.request(method, &req.url);
            for (k, v) in &req.headers {
                out = out.header(k, v);
            }
            let resp = out
                .body(req.body)
                .send()
                .await
                .map_err(|e| LoadError::Transport(e.to_string()))?;
            let status = resp.status().as_u16();
            let headers = resp
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_ascii_lowercase(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();
            let body = resp
                .bytes()
                .await
                .map_err(|e| LoadError::Transport(e.to_string()))?
                .to_vec();
            Ok(HttpResponse {
                status,
                headers,
                body,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Percent-encode a URI path component per RFC 3986 (unreserved = A–Z a–z
/// 0–9 `- . _ ~`). SigV4 and `s3://` parsing both need this; the encoder is
/// strict about `/` (kept in keys, encoded in query components by callers).
pub fn uri_encode(value: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');
        if keep || (byte == b'/' && !encode_slash) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, LoadError> {
    use hmac::Mac;
    let mut mac = <hmac::Hmac<sha2::Sha256> as hmac::Mac>::new_from_slice(key)
        .map_err(|e| LoadError::InvalidInput(format!("HMAC key rejected by signer: {e}")))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(data);
    hex_lower(&digest)
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// ISO-8601 basic format (YYYYMMDDTHHMMSSZ) required by SigV4.
fn amz_date(now: &std::time::SystemTime) -> (String, String) {
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let dt =
        chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0).unwrap_or_else(chrono::Utc::now);
    let full = dt.format("%Y%m%dT%H%M%SZ").to_string();
    let day = dt.format("%Y%m%d").to_string();
    (full, day)
}

// ---------------------------------------------------------------------------
// S3 client (SigV4, PUT + multipart)
// ---------------------------------------------------------------------------

/// S3 endpoint parameters. `region` applies to both AWS and S3-compatible
/// stores (MinIO uses any non-empty region, conventionally `us-east-1`).
#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    /// `https://s3.amazonaws.com` for AWS; `http://127.0.0.1:9000` for MinIO.
    pub endpoint: String,
    /// Access key id; operator material (never logged).
    pub access_key: String,
    /// Secret key; operator material (never logged).
    pub secret_key: String,
    /// Optional session token (STS); operator material.
    pub session_token: Option<String>,
    /// Force path-style addressing (`http://host/bucket/key`) — required for
    /// MinIO and most S3-compatible stores in tests.
    pub path_style: bool,
}

/// S3 upload client with SigV4 request signing (service `s3`, unsigned
/// payload *not* used — payloads are signed so the store verifies content).
pub struct S3Client<'a, T: Transport> {
    pub config: S3Config,
    pub transport: &'a T,
}

impl<'a, T: Transport> S3Client<'a, T> {
    pub fn new(config: S3Config, transport: &'a T) -> Self {
        Self { config, transport }
    }

    fn host(&self) -> String {
        self.config
            .endpoint
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/')
            .to_string()
    }

    /// Sign per the SigV4 spec for service `s3` (bucket-scoped signing scope,
    /// path-style or virtual-host addressing per config). Fallible only on
    /// signer-input problems (HMAC key length), which is a config error.
    fn sign(
        &self,
        method: &'static str,
        key: &str,
        query: &[(&str, &str)],
        payload_hash: &str,
        now: &std::time::SystemTime,
    ) -> Result<HttpRequest, LoadError> {
        let (amzdate, datestamp) = amz_date(now);
        let host = self.host();
        let (key_path, _bucket_scoped) = if self.config.path_style {
            (format!("/{}/{}", self.config.bucket, key), true)
        } else {
            (format!("/{}", key), false)
        };
        let canonical_uri = {
            let raw = key_path;
            // Encode each path segment but keep '/' separators.
            raw.split('/')
                .map(|seg| uri_encode(seg, false))
                .collect::<Vec<_>>()
                .join("/")
        };
        let canonical_query = if query.is_empty() {
            String::new()
        } else {
            let mut pairs: Vec<(String, String)> = query
                .iter()
                .map(|(k, v)| (uri_encode(k, true), uri_encode(v, true)))
                .collect();
            pairs.sort();
            pairs
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        };

        let host_header = if self.config.path_style {
            host
        } else {
            format!("{}.{}", self.config.bucket, host)
        };

        let mut canonical_headers = format!("host:{host_header}\n");
        let mut signed_headers = "host;x-amz-content-sha256;x-amz-date".to_string();
        if let Some(tok) = &self.config.session_token {
            canonical_headers.push_str(&format!("x-amz-security-token:{}\n", tok));
            signed_headers =
                "host;x-amz-content-sha256;x-amz-date;x-amz-security-token".to_string();
        }
        canonical_headers.push_str(&format!(
            "x-amz-content-sha256:{payload_hash}\nx-amz-date:{amzdate}\n"
        ));

        let canonical_request = format!(
            "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        );
        let scope = format!("{}/{}/s3/aws4_request", datestamp, self.config.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amzdate}\n{scope}\n{}",
            sha256_hex(canonical_request.as_bytes())
        );
        let k_date = hmac_sha256(
            format!("AWS4{}", self.config.secret_key).as_bytes(),
            datestamp.as_bytes(),
        )?;
        let k_region = hmac_sha256(&k_date, self.config.region.as_bytes())?;
        let k_service = hmac_sha256(&k_region, b"s3")?;
        let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
        let signature = hex_lower(&hmac_sha256(&k_signing, string_to_sign.as_bytes())?);

        let mut headers = vec![
            ("host".to_string(), host_header.clone()),
            ("x-amz-content-sha256".to_string(), payload_hash.to_string()),
            ("x-amz-date".to_string(), amzdate),
            (
                "authorization".to_string(),
                format!(
                    "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={signature}",
                    self.config.access_key, scope, signed_headers
                ),
            ),
        ];
        if let Some(tok) = &self.config.session_token {
            headers.push(("x-amz-security-token".to_string(), tok.clone()));
        }

        let url = if self.config.path_style {
            format!(
                "{}{canonical_uri}",
                self.config.endpoint.trim_end_matches('/')
            )
        } else {
            format!(
                "{}://{host_header}{canonical_uri}",
                if self.config.endpoint.starts_with("https") {
                    "https"
                } else {
                    "http"
                }
            )
        };
        // The query is part of both the signature and the wire request.
        let url = if canonical_query.is_empty() {
            url
        } else {
            format!("{url}?{canonical_query}")
        };

        Ok(HttpRequest {
            method,
            url,
            headers,
            // Body is attached by the caller (needs the payload hash first).
            body: Vec::new(),
        })
    }

    /// PUT an object in full (single request; fine up to ~5 GB).
    pub async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<(), LoadError> {
        let payload_hash = sha256_hex(&body);
        let mut req = self.sign(
            "PUT",
            key,
            &[],
            &payload_hash,
            &std::time::SystemTime::now(),
        )?;
        req.body = body;
        let resp = self
            .transport
            .send(req)
            .await
            .map_err(|e| LoadError::Transport(e.to_string()))?;
        resp.outcome()
    }

    /// GET an object (used by the round-trip contract test to verify the
    /// uploaded bytes; also lets pipelines diff re-exports before loading).
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, LoadError> {
        let req = self.sign(
            "GET",
            key,
            &[],
            &sha256_hex(b""),
            &std::time::SystemTime::now(),
        )?;
        let resp = self
            .transport
            .send(req)
            .await
            .map_err(|e| LoadError::Transport(e.to_string()))?;
        resp.outcome()?;
        Ok(resp.body)
    }

    /// Multipart upload of `body` in `part_size` chunks (5 MiB minimum per
    /// S3 rules except the last part). Returns the assembled object's key.
    pub async fn multipart_put_object(
        &self,
        key: &str,
        body: Vec<u8>,
        part_size: usize,
    ) -> Result<String, LoadError> {
        if part_size < 5 * 1024 * 1024 {
            return Err(LoadError::InvalidInput(
                "part_size must be at least 5 MiB (S3 multipart rule)".into(),
            ));
        }
        // initiate
        let init_req = self.sign(
            "POST",
            key,
            &[("uploads", "")],
            &sha256_hex(b""),
            &std::time::SystemTime::now(),
        )?;
        let init_resp = self.transport.send(init_req).await?;
        init_resp.outcome()?;
        let upload_id = parse_xml_tag(&init_resp.body, "UploadId")
            .ok_or_else(|| LoadError::Fatal {
                status: init_resp.status,
                detail: "InitiateMultipartUploadResponse missing UploadId".into(),
            })?
            .to_string();

        // upload parts (1-indexed)
        let mut etags: Vec<(usize, String)> = Vec::new();
        for (idx, chunk) in body.chunks(part_size).enumerate() {
            let part_number = (idx + 1).to_string();
            let payload_hash = sha256_hex(chunk);
            let mut req = self.sign(
                "PUT",
                key,
                &[
                    ("partNumber", part_number.as_str()),
                    ("uploadId", upload_id.as_str()),
                ],
                &payload_hash,
                &std::time::SystemTime::now(),
            )?;
            req.body = chunk.to_vec();
            let resp = self.transport.send(req).await?;
            resp.outcome()?;
            let etag = extract_header(&resp, "etag")
                .ok_or_else(|| LoadError::Fatal {
                    status: resp.status,
                    detail: "part response missing ETag".into(),
                })?
                .to_string();
            etags.push((idx + 1, etag));
        }

        // complete
        let xml = {
            let mut s = String::from("<CompleteMultipartUpload>");
            for (n, etag) in &etags {
                s.push_str(&format!(
                    "<Part><PartNumber>{n}</PartNumber><ETag>{etag}</ETag></Part>"
                ));
            }
            s.push_str("</CompleteMultipartUpload>");
            s
        };
        let payload_hash = sha256_hex(xml.as_bytes());
        let mut req = self.sign(
            "POST",
            key,
            &[("uploadId", upload_id.as_str())],
            &payload_hash,
            &std::time::SystemTime::now(),
        )?;
        req.body = xml.into_bytes();
        let resp = self.transport.send(req).await?;
        resp.outcome()?;
        Ok(key.to_string())
    }
}

fn extract_header(resp: &HttpResponse, name: &str) -> Option<String> {
    let want = name.to_ascii_lowercase();
    resp.headers
        .iter()
        .find(|(k, _)| *k == want)
        .map(|(_, v)| v.clone())
}

/// Minimal XML tag extraction for the two S3 responses this client reads
/// (`UploadId`, and errors). Not an XML parser — the S3 responses here are
/// machine-generated and structurally simple.
fn parse_xml_tag(body: &[u8], tag: &str) -> Option<String> {
    let text = String::from_utf8_lossy(body);
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].to_string())
}

// ---------------------------------------------------------------------------
// BigQuery load (jobs.insert via simple upload; multipart/related)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BigQueryConfig {
    pub project_id: String,
    pub dataset: String,
    /// OAuth2 bearer token; operator material (never logged).
    pub bearer_token: String,
    /// Defaults to `https://bigquery.googleapis.com`.
    pub endpoint: Option<String>,
}

pub struct BigQueryClient<'a, T: Transport> {
    pub config: BigQueryConfig,
    pub transport: &'a T,
}

impl<'a, T: Transport> BigQueryClient<'a, T> {
    pub fn new(config: BigQueryConfig, transport: &'a T) -> Self {
        Self { config, transport }
    }

    /// Load one NDJSON table file with `WRITE_TRUNCATE` disposition and the
    /// `_schema` manifest as the explicit schema (BigQuery accepts a JSON
    /// Schema object in the job resource). Uses the JSON API simple-upload
    /// path (`uploadType=media` on a job resource POST would lose metadata,
    /// so the multipart/related body carries both).
    pub async fn load_jsonl(
        &self,
        table: &str,
        jsonl: Vec<u8>,
        schema_manifest: &[u8],
    ) -> Result<(), LoadError> {
        let endpoint = self
            .config
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://bigquery.googleapis.com".to_string());
        let job = serde_json::json!({
            "configuration": {
                "load": {
                    "sourceFormat": "NEWLINE_DELIMITED_JSON",
                    "writeDisposition": "WRITE_TRUNCATE",
                    "destinationTable": {
                        "projectId": self.config.project_id,
                        "datasetId": self.config.dataset,
                        "tableId": table,
                    },
                    // The `_schema` manifest is the column contract; pass it
                    // through so the load fails loudly on drift rather than
                    // coercing silently.
                    "schemaInline": serde_json::from_slice::<serde_json::Value>(schema_manifest)
                        .map_err(|e| LoadError::InvalidInput(format!("schema manifest is not JSON: {e}")))?,
                    "schemaInlineFormat": "RECORD_COLUMNS",
                }
            }
        });
        let boundary = "crawlkit_bq_boundary_7f3a";
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(
            format!("--{boundary}\r\ncontent-type: application/json; charset=UTF-8\r\n\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(job.to_string().as_bytes());
        body.extend_from_slice(
            format!("\r\n--{boundary}\r\ncontent-type: application/octet-stream\r\n\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(&jsonl);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let url = format!(
            "{}/upload/bigquery/v2/projects/{}/jobs?uploadType=multipart",
            endpoint.trim_end_matches('/'),
            uri_encode(&self.config.project_id, false)
        );
        let req = HttpRequest {
            method: "POST",
            url,
            headers: vec![
                (
                    "authorization".to_string(),
                    format!("Bearer {}", self.config.bearer_token),
                ),
                (
                    "content-type".to_string(),
                    format!("multipart/related; boundary={boundary}"),
                ),
            ],
            body,
        };
        let resp = self.transport.send(req).await?;
        resp.outcome()
    }
}

// ---------------------------------------------------------------------------
// Snowflake load (statements API → COPY INTO)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SnowflakeConfig {
    pub account: String,
    pub warehouse: String,
    pub database: String,
    pub schema: String,
    /// OAuth access token; operator material (never logged).
    pub bearer_token: String,
    /// Defaults to `https://{account}.snowflakecomputing.com`.
    pub endpoint: Option<String>,
}

pub struct SnowflakeClient<'a, T: Transport> {
    pub config: SnowflakeConfig,
    pub transport: &'a T,
}

impl<'a, T: Transport> SnowflakeClient<'a, T> {
    pub fn new(config: SnowflakeConfig, transport: &'a T) -> Self {
        Self { config, transport }
    }

    /// Submit one COPY INTO statement against a stage via the SQL Statements
    /// API (v2). Returns the statement handle text for polling operators.
    pub async fn copy_into(
        &self,
        table: &str,
        stage: &str,
        path: &str,
    ) -> Result<String, LoadError> {
        let endpoint = self
            .config
            .endpoint
            .clone()
            .unwrap_or_else(|| format!("https://{}", uri_encode(&self.config.account, false)));
        let sql = format!(
            "COPY INTO {}.{} FROM @{}/{} FILE_FORMAT = (TYPE = JSON) MATCH_BY_COLUMN_NAME = CASE_INSENSITIVE",
            uri_encode(&self.config.database, false),
            uri_encode(table, false),
            uri_encode(stage, false),
            uri_encode(path, false),
        );
        let body = serde_json::json!({
            "statement": sql,
            "timeout": 60,
            "warehouse": self.config.warehouse,
            "role": null,
        });
        let url = format!("{}/api/v2/statements", endpoint.trim_end_matches('/'));
        let req = HttpRequest {
            method: "POST",
            url,
            headers: vec![
                (
                    "authorization".to_string(),
                    format!("Bearer {}", self.config.bearer_token),
                ),
                ("content-type".to_string(), "application/json".to_string()),
                ("accept".to_string(), "application/json".to_string()),
            ],
            body: body.to_string().into_bytes(),
        };
        let resp = self.transport.send(req).await?;
        resp.outcome()?;
        // The statements API returns the async handle in the Location header
        // for async submissions; for sync ones the handle is in the body.
        let text = String::from_utf8_lossy(&resp.body);
        Ok(serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| {
                v.get("statementHandle")
                    .and_then(|h| h.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "sync-complete".to_string()))
    }
}

// ---------------------------------------------------------------------------
// Tests — request-shape contracts against an in-memory transport
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod memory_transport {
    use super::*;
    use std::sync::Mutex;

    pub struct MemoryTransport {
        pub requests: Mutex<Vec<HttpRequest>>,
        pub responder: Box<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>,
    }

    impl MemoryTransport {
        pub fn new(
            responder: impl Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
        ) -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                responder: Box::new(responder),
            }
        }

        pub fn recorded(&self) -> Vec<HttpRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Default for MemoryTransport {
        fn default() -> Self {
            Self::new(|_| HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: Vec::new(),
            })
        }
    }

    impl Transport for MemoryTransport {
        async fn send(&self, req: HttpRequest) -> Result<HttpResponse, LoadError> {
            self.requests.lock().unwrap().push(req.clone());
            Ok((self.responder)(&req))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::memory_transport::*;
    use super::*;

    fn s3_config() -> S3Config {
        S3Config {
            bucket: "crawlkit-exports".into(),
            region: "us-east-1".into(),
            endpoint: "http://127.0.0.1:9000".into(),
            access_key: "AKIA_TEST".into(),
            secret_key: "secret_test".into(),
            session_token: None,
            path_style: true,
        }
    }

    #[tokio::test]
    async fn s3_put_signs_canonical_request_and_sends_body() {
        let t = MemoryTransport::default();
        let client = S3Client::new(s3_config(), &t);
        client
            .put_object("exports/c1/crawl_runs.parquet", b"parquet-bytes".to_vec())
            .await
            .unwrap();
        let reqs = t.recorded();
        assert_eq!(reqs.len(), 1);
        let req = &reqs[0];
        assert_eq!(req.method, "PUT");
        assert_eq!(
            req.url,
            "http://127.0.0.1:9000/crawlkit-exports/exports/c1/crawl_runs.parquet"
        );
        let auth = &req
            .headers
            .iter()
            .find(|(k, _)| k == "authorization")
            .unwrap()
            .1;
        assert!(
            auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIA_TEST/"),
            "{auth}"
        );
        assert!(auth.contains("us-east-1/s3/aws4_request"), "{auth}");
        assert!(
            auth.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"),
            "{auth}"
        );
        // Payload hash matches the sent body.
        let hash = &req
            .headers
            .iter()
            .find(|(k, _)| k == "x-amz-content-sha256")
            .unwrap()
            .1;
        assert_eq!(*hash, sha256_hex(b"parquet-bytes"));
        assert_eq!(req.body, b"parquet-bytes");
    }

    #[tokio::test]
    async fn s3_multipart_fails_below_5mib_part_floor() {
        let t = MemoryTransport::default();
        let client = S3Client::new(s3_config(), &t);
        let err = client
            .multipart_put_object("k", vec![0u8; 1024], 1024)
            .await
            .unwrap_err();
        assert!(matches!(err, LoadError::InvalidInput(_)), "{err}");
    }

    #[tokio::test]
    async fn s3_multipart_initiates_uploads_and_completes() {
        // Responder: initiate returns an UploadId; part PUTs return ETags
        // (folded into the body line for the memory transport); complete
        // returns 200.
        let t = MemoryTransport::new(|req: &HttpRequest| {
            if req.url.contains("uploads=") && !req.url.contains("uploadId") {
                HttpResponse {
                    status: 200,
                    headers: vec![],
                    body: b"<InitiateMultipartUploadResult><UploadId>UPID-123</UploadId></InitiateMultipartUploadResult>".to_vec(),
                }
            } else if req.url.contains("uploadId") && req.method == "POST" {
                HttpResponse { status: 200, headers: vec![], body: b"<CompleteMultipartUploadResult><ETag>\"final\"</ETag></CompleteMultipartUploadResult>".to_vec() }
            } else {
                HttpResponse {
                    status: 200,
                    headers: vec![(
                        "etag".to_string(),
                        "\"d41d8cd98f00b204e9800998ecf8427e\"".to_string(),
                    )],
                    body: Vec::new(),
                }
            }
        });
        let client = S3Client::new(s3_config(), &t);
        let part = 5 * 1024 * 1024;
        let mut body = vec![0u8; part + 1024];
        body[part] = 7;
        let key = client
            .multipart_put_object("exports/c1/findings.parquet", body, part)
            .await
            .unwrap();
        assert_eq!(key, "exports/c1/findings.parquet");
        let reqs = t.recorded();
        // 1 initiate + 2 parts + 1 complete.
        assert_eq!(reqs.len(), 4, "got {} requests", reqs.len());
        assert!(
            reqs[0].url.ends_with("?uploads="),
            "initiate: {}",
            reqs[0].url
        );
        assert!(
            reqs[1].url.contains("partNumber=1"),
            "part1: {}",
            reqs[1].url
        );
        assert!(
            reqs[2].url.contains("partNumber=2"),
            "part2: {}",
            reqs[2].url
        );
        assert!(
            reqs[3].url.contains("uploadId=UPID-123"),
            "complete: {}",
            reqs[3].url
        );
        // Part bodies carry the exact chunk bytes.
        assert_eq!(reqs[1].body.len(), part);
        assert_eq!(reqs[2].body.len(), 1024);
    }

    #[tokio::test]
    async fn s3_session_token_is_signed_and_sent() {
        let t = MemoryTransport::default();
        let mut cfg = s3_config();
        cfg.session_token = Some("TOKEN-XYZ".into());
        let client = S3Client::new(cfg, &t);
        client.put_object("k", b"d".to_vec()).await.unwrap();
        let req = &t.recorded()[0];
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "x-amz-security-token" && v == "TOKEN-XYZ"));
        let auth = &req
            .headers
            .iter()
            .find(|(k, _)| k == "authorization")
            .unwrap()
            .1;
        assert!(auth.contains("x-amz-security-token"), "{auth}");
    }

    #[tokio::test]
    async fn bq_load_builds_multipart_related_job_with_schema() {
        let t = MemoryTransport::default();
        let client = BigQueryClient::new(
            BigQueryConfig {
                project_id: "proj".into(),
                dataset: "crawlkit".into(),
                bearer_token: "tok".into(),
                endpoint: None,
            },
            &t,
        );
        client
            .load_jsonl("findings", b"{\"a\":1}\n".to_vec(), b"{\"columns\":[]}")
            .await
            .unwrap();
        let req = &t.recorded()[0];
        assert_eq!(req.method, "POST");
        assert_eq!(
            req.url,
            "https://bigquery.googleapis.com/upload/bigquery/v2/projects/proj/jobs?uploadType=multipart"
        );
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "authorization" && v == "Bearer tok"));
        let body = String::from_utf8_lossy(&req.body);
        assert!(body.contains("NEWLINE_DELIMITED_JSON"), "{body}");
        assert!(body.contains("WRITE_TRUNCATE"), "{body}");
        assert!(body.contains("destinationTable"), "{body}");
        assert!(body.contains("\"a\":1"), "jsonl part missing: {body}");
    }

    #[tokio::test]
    async fn bq_load_rejects_non_json_schema_before_sending() {
        let t = MemoryTransport::default();
        let client = BigQueryClient::new(
            BigQueryConfig {
                project_id: "p".into(),
                dataset: "d".into(),
                bearer_token: "t".into(),
                endpoint: None,
            },
            &t,
        );
        let err = client
            .load_jsonl("findings", vec![], b"not json")
            .await
            .unwrap_err();
        assert!(matches!(err, LoadError::InvalidInput(_)), "{err}");
        assert!(t.recorded().is_empty(), "no request should be sent");
    }

    #[tokio::test]
    async fn snowflake_copy_statements_api_shape() {
        let t = MemoryTransport::default();
        let client = SnowflakeClient::new(
            SnowflakeConfig {
                account: "acct.snowflakecomputing.com".into(),
                warehouse: "WH".into(),
                database: "CRAWLK".into(),
                schema: "PUBLIC".into(),
                bearer_token: "tok".into(),
                endpoint: None,
            },
            &t,
        );
        let handle = client
            .copy_into("findings", "exports_stage", "exports/c1/findings.jsonl")
            .await
            .unwrap();
        assert_eq!(handle, "sync-complete");
        let req = &t.recorded()[0];
        assert_eq!(
            req.url,
            "https://acct.snowflakecomputing.com/api/v2/statements"
        );
        let body = String::from_utf8_lossy(&req.body);
        assert!(body.contains("COPY INTO"), "{body}");
        assert!(
            body.contains("@exports_stage/exports/c1/findings.jsonl"),
            "{body}"
        );
        assert!(body.contains("TYPE = JSON"), "{body}");
    }

    #[tokio::test]
    async fn outcome_classification_matches_connector_posture() {
        let mk = |status: u16| HttpResponse {
            status,
            headers: vec![],
            body: vec![],
        };
        let ok = mk(200);
        let retry = mk(503);
        let rate = mk(429);
        let fatal = HttpResponse {
            status: 403,
            headers: vec![],
            body: b"denied".to_vec(),
        };
        assert!(ok.outcome().is_ok());
        assert!(matches!(
            retry.outcome(),
            Err(LoadError::Retryable { status: 503 })
        ));
        assert!(matches!(
            rate.outcome(),
            Err(LoadError::Retryable { status: 429 })
        ));
        assert!(matches!(
            fatal.outcome(),
            Err(LoadError::Fatal { status: 403, .. })
        ));
    }

    #[test]
    fn uri_encode_contract() {
        assert_eq!(uri_encode("a b/c", false), "a%20b/c");
        assert_eq!(uri_encode("a b/c", true), "a%20b%2Fc");
        assert_eq!(uri_encode("crawl-1.0_x~", false), "crawl-1.0_x~");
    }
}
