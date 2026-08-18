use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use http::HeaderValue;

use crate::types::*;

pub async fn request_metrics_middleware(
    State(state): State<AppState>,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed().as_secs_f64();

    let endpoint = uri.path().to_string();
    state
        .metrics
        .requests_total
        .get_or_create(&EndpointLabel { endpoint })
        .inc();
    state.metrics.request_duration_seconds.observe(duration);

    response
}

pub async fn csp_headers(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_str(&csp_policy()).unwrap_or_else(|_| {
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
                 img-src 'self' data: https:; connect-src 'self'; frame-ancestors 'none'; \
                 base-uri 'self'; form-action 'self'",
            )
        }),
    );
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

pub fn csp_policy() -> String {
    std::env::var("CSP_POLICY").unwrap_or_else(|_| {
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; connect-src 'self'; frame-ancestors 'none'; \
         base-uri 'self'; form-action 'self'"
            .to_string()
    })
}

pub async fn csrf_origin_validation(
    State(allowed_origins): State<Vec<String>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();

    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let origin = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("referer")
                .and_then(|v| v.to_str().ok())
                .and_then(|referer| {
                    url::Url::parse(referer)
                        .ok()
                        .map(|u| format!("{}://{}", u.scheme(), u.authority()))
                })
        });

    match origin {
        Some(ref origin_str) => {
            if allowed_origins.iter().any(|o| o == origin_str) {
                Ok(next.run(request).await)
            } else {
                tracing::warn!("CSRF origin rejected: {origin_str}");
                Err(StatusCode::FORBIDDEN)
            }
        }
        None => {
            tracing::warn!("CSRF check: missing Origin/Referer header on {method}");
            Err(StatusCode::FORBIDDEN)
        }
    }
}

pub async fn api_key_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, ApiError> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing X-API-Key header".to_string()))?
        .to_string();

    let key_info = state
        .api_keys
        .get(&api_key)
        .ok_or_else(|| ApiError::Unauthorized("Invalid API key".to_string()))?;

    let rpm = key_info.requests_per_minute;
    drop(key_info);

    // Rate limit check
    let mut bucket = state
        .rate_limits
        .entry(api_key.clone())
        .or_insert_with(|| RateLimitBucket::new(rpm));

    if !bucket.try_consume() {
        return Err(ApiError::RateLimited);
    }

    #[allow(clippy::cast_possible_truncation)]
    let remaining = bucket.tokens.floor() as u64;
    #[allow(clippy::cast_possible_truncation)]
    let reset_seconds = ((bucket.max_tokens - bucket.tokens) / bucket.refill_rate).ceil() as u64;
    drop(bucket);

    let mut response = next.run(request).await;
    let resp_headers = response.headers_mut();
    resp_headers.insert("X-RateLimit-Limit", rpm.into());
    resp_headers.insert("X-RateLimit-Remaining", remaining.into());
    resp_headers.insert("X-RateLimit-Reset", reset_seconds.into());
    Ok(response)
}
