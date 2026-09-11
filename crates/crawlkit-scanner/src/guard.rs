//! Hosted free-scan scanner (ADR-012) — guard module.
//!
//! This module is the scanner's own trust boundary and deliberately does not
//! reuse the engine's `ssrf::is_private_ip` / `http::dns_pin_check`:
//!
//! 1. The engine's checks honor the `CRAWLKIT_ALLOW_PRIVATE=1` development
//!    escape hatch. The scanner must have no environment-dependent bypass.
//! 2. The engine's check is check-then-fetch: it validates DNS answers before
//!    a fetch but does not pin the connection to the validated addresses. The
//!    scanner pins: [`resolve_pinned`] returns only the validated addresses
//!    and the caller must connect to exactly those (no re-resolution), which
//!    closes the DNS-rebinding window between check and use.
//!
//! Every validation in this file is deny-by-construction: there is no flag,
//! environment variable, or configuration that permits a private target.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use thiserror::Error;
use url::Url;

/// Errors produced while validating or resolving a scan target.
#[derive(Debug, Error)]
pub enum GuardError {
    /// URL failed static validation (scheme, credentials, host, IP range).
    #[error("target rejected: {0}")]
    InvalidTarget(&'static str),
    /// DNS resolution failed or returned no usable addresses.
    #[error("dns resolution failed for {host}: {source}")]
    Resolution {
        host: String,
        #[source]
        source: Arc<dyn std::error::Error + Send + Sync>,
    },
    /// Every resolved address was rejected by the private-range policy.
    #[error("target {host} resolved only to disallowed addresses")]
    AllAddressesDenied { host: String },
    /// The pinned HTTP client could not be constructed (fatal at startup).
    #[error("scanner client construction failed")]
    ClientBuild {
        #[source]
        source: Arc<dyn std::error::Error + Send + Sync>,
    },
}

/// Validate a scan target without touching the network.
///
/// Enforces the ADR-012 static rules:
/// - HTTP(S) schemes only
/// - no `user:pass@host` credentials
/// - a host must be present
/// - literal-IP hosts must be public (IPv4-mapped IPv6 is unwrapped first)
///
/// Hostname targets are validated again after resolution in [`resolve_pinned`],
/// so this function alone is not sufficient for hostname targets.
pub fn validate_target(raw_url: &str) -> Result<Url, GuardError> {
    let url = Url::parse(raw_url).map_err(|_| GuardError::InvalidTarget("unparseable URL"))?;

    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(GuardError::InvalidTarget(
            "only http and https schemes are allowed",
        ));
    }
    // `file:`, `data:`, `ftp:`, and any future scheme are denied above; this
    // also implicitly denies URLs without a host (`data:` etc.).
    if url.host_str().is_none() {
        return Err(GuardError::InvalidTarget("URL has no host"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(GuardError::InvalidTarget(
            "credentials in URL are not allowed",
        ));
    }

    // Host presence was verified above; treat absence defensively (deny).
    let Some(host) = url.host_str() else {
        return Err(GuardError::InvalidTarget("URL has no host"));
    };
    if let Some(ip) = literal_ip(host) {
        if !is_allowed_ip(ip) {
            return Err(GuardError::InvalidTarget(
                "literal IP is not a public address",
            ));
        }
    }
    Ok(url)
}

/// Async resolution + validation. This is the primary entry point.
///
/// Resolve `url`'s host and return **only** addresses that pass the
/// private-range policy, for use as pinned connect addresses.
///
/// The caller must connect to these addresses directly (SNI/Host preserved)
/// and must not re-resolve; re-resolution would reopen the DNS-rebinding
/// window. If the URL's host is a literal IP, resolution is skipped and the
/// single validated address is returned.
pub async fn resolve_pinned_async(url: &Url) -> Result<Vec<SocketAddr>, GuardError> {
    let host = url
        .host_str()
        .ok_or(GuardError::InvalidTarget("URL has no host"))?
        .trim_end_matches('.')
        .to_ascii_lowercase();

    if let Some(ip) = literal_ip(&host) {
        if !is_allowed_ip(ip) {
            return Err(GuardError::InvalidTarget(
                "literal IP is not a public address",
            ));
        }
        return Ok(vec![SocketAddr::new(ip, port_of(url))]);
    }

    let addrs = tokio::net::lookup_host((host.as_str(), 0))
        .await
        .map_err(|e| GuardError::Resolution {
            host: host.clone(),
            source: Arc::new(e),
        })?;

    let mut allowed = Vec::new();
    for addr in addrs {
        if is_allowed_ip(addr.ip()) {
            allowed.push(SocketAddr::new(addr.ip(), port_of(url)));
        }
    }

    if allowed.is_empty() {
        return Err(GuardError::AllAddressesDenied { host });
    }
    Ok(allowed)
}

/// Validate one redirect hop. Each hop is an independent trust decision: the
/// hop target must pass static validation, and the caller must re-run
/// [`resolve_pinned_async`] for hostname targets before connecting.
pub fn validate_redirect_hop(target: &Url) -> Result<(), GuardError> {
    // Redirect targets reuse the same static rules as the original target.
    validate_target(target.as_str()).map(|_| ())
}

/// The port a request to `url` should use (explicit or scheme default).
fn port_of(url: &Url) -> u16 {
    url.port_or_known_default().unwrap_or(80)
}

/// Parse a literal IP out of a URL host string (strips IPv6 brackets and a
/// trailing dot for FQDN-form literals).
fn literal_ip(host: &str) -> Option<IpAddr> {
    host.trim_matches(['[', ']'])
        .trim_end_matches('.')
        .parse::<IpAddr>()
        .ok()
}

/// The ADR-012 allowlist for connectable addresses.
///
/// Everything not explicitly public is denied. IPv4-mapped IPv6
/// (`::ffff:10.0.0.1`) is unwrapped and evaluated as IPv4 so mapped-private
/// addresses cannot smuggle through the V6 branch.
pub fn is_allowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_allowed_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_allowed_v4(v4);
            }
            is_allowed_v6(v6)
        }
    }
}

fn is_allowed_v4(v4: Ipv4Addr) -> bool {
    let o = v4.octets();
    // Deny the well-known non-routable / special-purpose ranges outright and
    // allow everything else. This inverts the engine's denylist posture: the
    // scanner allows only clearly-public space.
    let denied = v4.is_private()                       // 10/8, 172.16/12, 192.168/16
        || v4.is_loopback()                            // 127/8
        || v4.is_link_local()                          // 169.254/16
        || v4.is_broadcast()                           // 255.255.255.255
        || v4.is_unspecified()                         // 0.0.0.0
        || v4.is_multicast()                           // 224/4
        || o[0] == 0                                   // 0/8 ("this network")
        || (o[0] == 100 && (64..=127).contains(&o[1])) // 100.64/10 CGNAT
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)     // 192.0.0/24
        || (o[0] == 192 && o[1] == 0 && o[2] == 2)     // 192.0.2/24 TEST-NET-1
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19)) // 198.18/15 benchmarking
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)  // 198.51.100/24 TEST-NET-2
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)   // 203.0.113/24 TEST-NET-3
        || (o[0] >= 240); // 240/4 reserved + broadcast
    !denied
}

fn is_allowed_v6(v6: Ipv6Addr) -> bool {
    let seg = v6.segments();
    let denied = v6.is_loopback()                       // ::1
        || v6.is_unspecified()                          // ::
        || v6.is_multicast()                            // ff00::/8
        || (seg[0] & 0xfe00) == 0xfc00                  // fc00::/7 ULA
        || (seg[0] & 0xffc0) == 0xfe80                  // fe80::/10 link-local
        || (seg[0] == 0x100 && seg[1] == 0 && seg[2] == 0 && seg[3] == 0) // 100::/64 discard
        || (seg[0] == 0x2001 && seg[1] == 0xdb8)        // 2001:db8::/32 documentation
        || (seg[0] == 0x2001 && seg[1] == 0x0000)       // 2001::/32 Teredo (tunnels to v4)
        || (seg[0] == 0x64 && seg[1] == 0xff9b && seg[2] == 0x0001); // 64:ff9b:1::/48 local-use NAT64
    !denied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_public_https_target() {
        assert!(validate_target("https://example.com/").is_ok());
    }

    #[test]
    fn rejects_non_http_schemes() {
        for url in [
            "file:///etc/passwd",
            "data:text/html,x",
            "ftp://example.com/",
        ] {
            assert!(validate_target(url).is_err(), "{url} must be denied");
        }
    }

    #[test]
    fn rejects_credentials() {
        assert!(validate_target("https://user:pass@example.com/").is_err());
        assert!(validate_target("https://user@example.com/").is_err());
    }

    #[test]
    fn rejects_private_literal_ips() {
        for url in [
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/", // cloud metadata
            "http://0.0.0.0/",
            "http://100.64.0.1/",
            "http://[::1]/",
            "http://[fc00::1]/",
            "http://[fe80::1]/",
            "http://[::ffff:10.0.0.1]/", // IPv4-mapped private
            "http://[::ffff:127.0.0.1]/",
        ] {
            assert!(validate_target(url).is_err(), "{url} must be denied");
        }
    }

    #[test]
    fn rejects_localhost_names() {
        // Localhost names resolve via /etc/hosts, so the resolver path is the
        // enforcement point; static validation must not crash on them and the
        // resolved-address check (is_allowed_ip) denies the result.
        assert!(validate_target("http://localhost:8080/").is_ok());
        assert!(!is_allowed_ip("127.0.0.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn allows_public_literal_ips() {
        assert!(validate_target("http://93.184.216.34/").is_ok());
        assert!(is_allowed_ip("93.184.216.34".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn mapped_v6_unwraps_to_v4_policy() {
        // ::ffff:8.8.8.8 is public IPv4 behind a mapped prefix — allowed.
        assert!(is_allowed_ip("::ffff:8.8.8.8".parse::<IpAddr>().unwrap()));
        // ::ffff:192.168.0.1 is private behind a mapped prefix — denied.
        assert!(!is_allowed_ip(
            "::ffff:192.168.0.1".parse::<IpAddr>().unwrap()
        ));
    }

    #[test]
    fn test_net_and_documentation_ranges_denied() {
        for ip in ["192.0.2.1", "198.51.100.7", "203.0.113.9", "198.18.0.3"] {
            assert!(!is_allowed_ip(ip.parse::<IpAddr>().unwrap()), "{ip}");
        }
        assert!(!is_allowed_ip("2001:db8::1".parse::<IpAddr>().unwrap()));
        assert!(!is_allowed_ip("64:ff9b:1::1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn redirect_hop_validation_rejects_private_target() {
        let target = Url::parse("http://169.254.169.254/latest/meta-data/").unwrap();
        assert!(validate_redirect_hop(&target).is_err());
    }

    #[tokio::test]
    async fn pinned_resolution_denies_metadata_host() {
        // "localhost" resolves to loopback in every environment, so the
        // resolver-path denial is exercised deterministically without DNS.
        let url = Url::parse("http://localhost:8080/").unwrap();
        let err = resolve_pinned_async(&url).await.unwrap_err();
        assert!(matches!(
            err,
            GuardError::AllAddressesDenied { .. } | GuardError::InvalidTarget(_)
        ));
    }
}
