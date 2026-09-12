//! Hosted free-scan scanner (ADR-012) — production fetcher.
//!
//! [`PinnedFetcher`] enforces the ADR-012 trust boundary on every outbound
//! request:
//!
//! 1. **Static rules per hop.** The submitted URL and every redirect target
//!    pass [`guard::validate_redirect_hop`] (scheme, credentials, literal-IP
//!    policy) before a request is issued.
//! 2. **DNS pinning via the resolver hook.** The client is built with
//!    [`GuardedResolver`], reqwest's `dns_resolver` hook. Resolution and
//!    policy filtering happen in the same call that produces the connect
//!    addresses, so there is no check-then-connect gap to exploit and no
//!    code path that can re-resolve past the policy. This closes DNS
//!    rebinding by construction rather than by pre-fetch checks.
//! 3. **Manual redirect loop.** Redirects are followed explicitly (≤ 8 hops)
//!    so each hop is an independent trust decision and the full chain is
//!    recorded for analysis.
//! 4. **Body cap.** Response bodies are streamed with a hard byte ceiling
//!    ([`MAX_BODY_BYTES`]); oversized bodies are truncated and flagged.
//!
//! Read-only posture: GET only, no cookie store, no form submission, no
//! state between scans.

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crawlkit_engine::RedirectHop;
use url::Url;

use crate::bounds::{MAX_BODY_BYTES, MAX_REDIRECTS};
use crate::guard::{self, GuardError};
use crate::scan::{Fetch, FetchOutcome, SCANNER_USER_AGENT};

/// Per-request total timeout (scan-level wall clock is enforced separately
/// by the scan engine).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Connection timeout; bounds the pinned connect phase.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// DNS resolver that enforces the scanner address policy (ADR-012 §1).
///
/// Installed via `ClientBuilder::dns_resolver`, this hook is the single
/// production point of DNS answers for the client: every connection (initial
/// request, redirect hop, or pool reconnect) can only ever receive addresses
/// that passed [`guard::is_allowed_ip`]. Because filtering happens inside
/// the resolver future before addresses are returned, an attacker-controlled
/// DNS answer containing a private address yields a resolution error, not a
/// connection.
struct GuardedResolver;

impl reqwest::dns::Resolve for GuardedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?
                .filter(|addr| guard::is_allowed_ip(addr.ip()))
                .collect();
            if addrs.is_empty() {
                // The ADR-012 SSRF boundary just fired: DNS answered, but
                // every address was private/reserved. This is the attack-
                // recon signal the runbook §4 pages on, so it is counted
                // here — the only place the policy decision is visible.
                crate::metrics::METRICS.record_rejected(crate::metrics::RejectionCause::SsrfDenied);
                return Err(format!("scanner policy: {host} has no permitted addresses").into());
            }
            Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

/// Production [`Fetch`] implementation wrapping a policy-pinned client.
pub struct PinnedFetcher {
    client: reqwest::Client,
}

impl PinnedFetcher {
    /// Build the fetcher. Fails only if the underlying client cannot be
    /// constructed (TLS backend init), which is fatal at service startup.
    pub fn new() -> Result<Self, GuardError> {
        let client = reqwest::Client::builder()
            .dns_resolver(Arc::new(GuardedResolver))
            .user_agent(SCANNER_USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            // No cookie store: reqwest only persists cookies when the
            // `cookies` feature is enabled, keeping the read-only posture.
            .build()
            .map_err(|e| GuardError::ClientBuild {
                source: Arc::new(e),
            })?;
        Ok(Self { client })
    }

    async fn fetch_inner(&self, url: &Url) -> FetchOutcome {
        let start = Instant::now();
        let mut current = url.clone();
        let mut chain: Vec<RedirectHop> = Vec::new();

        for _hop in 0..=MAX_REDIRECTS {
            // Static rules for this hop (scheme, credentials, literal IPs).
            if let Err(e) = guard::validate_redirect_hop(&current) {
                return FetchOutcome {
                    status: 0,
                    headers: Vec::new(),
                    body: Vec::new(),
                    elapsed: start.elapsed(),
                    redirect_chain: chain,
                    error: Some(format!("redirect target rejected: {e}")),
                };
            }

            let response = match self.client.get(current.clone()).send().await {
                Ok(r) => r,
                Err(e) => {
                    return FetchOutcome {
                        status: 0,
                        headers: Vec::new(),
                        body: Vec::new(),
                        elapsed: start.elapsed(),
                        redirect_chain: chain,
                        error: Some(format!("request to {current} failed: {e}")),
                    };
                }
            };

            let status = response.status().as_u16();

            // Manual redirect following: each hop re-validated above and
            // re-resolved through GuardedResolver inside the client.
            if (300..400).contains(&status) {
                let Some(location) = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                else {
                    return self.read_body(response, start, chain).await;
                };
                let next = match current.join(location) {
                    Ok(n) => n,
                    Err(e) => {
                        return FetchOutcome {
                            status: 0,
                            headers: Vec::new(),
                            body: Vec::new(),
                            elapsed: start.elapsed(),
                            redirect_chain: chain,
                            error: Some(format!("invalid redirect location: {e}")),
                        };
                    }
                };
                chain.push(RedirectHop {
                    from: current.clone(),
                    to: next.clone(),
                    status_code: status,
                });
                current = next;
                continue;
            }

            return self.read_body(response, start, chain).await;
        }

        FetchOutcome {
            status: 0,
            headers: Vec::new(),
            body: Vec::new(),
            elapsed: start.elapsed(),
            redirect_chain: chain,
            error: Some(format!("redirect chain exceeded {MAX_REDIRECTS} hops")),
        }
    }

    /// Stream the response body under the byte ceiling and produce the
    /// final outcome. Oversized bodies are truncated; the scan layer flags
    /// the record when the ceiling was hit.
    async fn read_body(
        &self,
        response: reqwest::Response,
        start: Instant,
        chain: Vec<RedirectHop>,
    ) -> FetchOutcome {
        let status = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|vs| (k.as_str().to_string(), vs.to_string()))
            })
            .collect();

        let mut body: Vec<u8> = Vec::new();
        let mut oversized = false;
        let mut response = response;
        while let Some(chunk) = response.chunk().await.transpose() {
            match chunk {
                Ok(bytes) => {
                    if (body.len() as u64) + (bytes.len() as u64) > MAX_BODY_BYTES {
                        body.extend_from_slice(
                            &bytes[..usize::try_from(
                                MAX_BODY_BYTES.saturating_sub(body.len() as u64),
                            )
                            .unwrap_or(0)],
                        );
                        oversized = true;
                        break;
                    }
                    body.extend_from_slice(&bytes);
                }
                Err(e) => {
                    // Partial bytes crossed the wire before the failure;
                    // egress cost was incurred, so it is counted.
                    crate::metrics::METRICS.record_egress_bytes(body.len() as u64);
                    return FetchOutcome {
                        status: 0,
                        headers,
                        body: Vec::new(),
                        elapsed: start.elapsed(),
                        redirect_chain: chain,
                        error: Some(format!("body read failed: {e}")),
                    };
                }
            }
        }

        // Application-level egress (runbook §4): body bytes actually pulled,
        // including truncated reads — those bytes did cross the wire.
        crate::metrics::METRICS.record_egress_bytes(body.len() as u64);

        FetchOutcome {
            status,
            headers,
            body,
            elapsed: start.elapsed(),
            redirect_chain: chain,
            error: oversized.then(|| format!("body exceeded {MAX_BODY_BYTES} byte ceiling")),
        }
    }
}

impl Fetch for PinnedFetcher {
    fn fetch<'a>(
        &'a self,
        url: &'a Url,
    ) -> Pin<Box<dyn Future<Output = FetchOutcome> + Send + 'a>> {
        Box::pin(self.fetch_inner(url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarded_resolver_filters_private_answers() {
        // The resolver's policy function is the security core; exercise it
        // directly over synthetic answer sets (no DNS in unit tests).
        let cases: &[(&str, bool)] = &[
            ("127.0.0.1", false),
            ("10.1.2.3", false),
            ("169.254.169.254", false),
            ("192.168.0.9", false),
            ("::ffff:10.0.0.7", false),
            ("93.184.216.34", true),
            ("2606:2800:220:1:248:1893:25c8:1946", true),
        ];
        for (ip, allowed) in cases {
            let parsed: std::net::IpAddr = ip.parse().unwrap();
            assert_eq!(guard::is_allowed_ip(parsed), *allowed, "{ip}");
        }
    }

    #[test]
    fn fetcher_builds_without_network() {
        assert!(PinnedFetcher::new().is_ok());
    }
}
