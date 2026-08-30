//! SSRF boundary contract tests.
//!
//! These tests assert the documented reject-list from
//! `docs/SECURITY_BOUNDARIES.md` against `crawlkit_engine::ssrf::is_public_url`.
//! They exist so the security boundary cannot silently regress: any change
//! that permits a previously-rejected target will fail here.
//!
//! Scope: `is_public_url` validates the *host* portion of a URL against
//! private/loopback/link-local/metadata endpoints. DNS-rebinding and
//! redirect-time protection are enforced at the HTTP-client layer and are
//! documented (not asserted) in the boundary doc.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crawlkit_engine::ssrf::is_public_url;

// ---------------------------------------------------------------------------
// Must-accept (public targets)
// ---------------------------------------------------------------------------

#[test]
fn accepts_public_https_domain() {
    assert!(is_public_url("https://example.com"));
    assert!(is_public_url("https://api.stripe.com/v1/charges"));
}

#[test]
fn accepts_public_http_domain_with_port_and_path() {
    assert!(is_public_url("http://example.com:8080/path?q=1"));
}

#[test]
fn accepts_public_ipv4() {
    assert!(is_public_url("https://8.8.8.8/"));
}

// ---------------------------------------------------------------------------
// Must-reject: scheme / structure
// ---------------------------------------------------------------------------

#[test]
fn rejects_non_http_schemes() {
    assert!(!is_public_url("ftp://example.com/file"));
    assert!(!is_public_url("file:///etc/passwd"));
    assert!(!is_public_url("gopher://example.com/"));
}

#[test]
fn rejects_empty_and_malformed() {
    assert!(!is_public_url(""));
    assert!(!is_public_url("not-a-url"));
    assert!(!is_public_url("://no-scheme/"));
}

// ---------------------------------------------------------------------------
// Must-reject: localhost / metadata endpoints
// ---------------------------------------------------------------------------

#[test]
fn rejects_localhost_hostname() {
    assert!(!is_public_url("http://localhost/"));
    assert!(!is_public_url("http://localhost.localdomain/"));
}

#[test]
fn rejects_cloud_metadata_endpoints() {
    // GCP
    assert!(!is_public_url("http://metadata.google.internal/latest"));
    // AWS / Azure / Alibaba — link-local 169.254.x.x
    assert!(!is_public_url("http://169.254.169.254/latest/meta-data/"));
    assert!(!is_public_url(
        "http://169.254.169.254/metadata/instance?api-version=2021-02-01"
    ));
}

// ---------------------------------------------------------------------------
// Must-reject: private / reserved IPv4 ranges
// ---------------------------------------------------------------------------

#[test]
fn rejects_loopback_ipv4() {
    assert!(!is_public_url("http://127.0.0.1/"));
    assert!(!is_public_url("http://127.255.255.254/"));
}

#[test]
fn rejects_rfc1918_private_ranges() {
    assert!(!is_public_url("http://10.0.0.1/"));
    assert!(!is_public_url("http://10.255.255.255/"));
    assert!(!is_public_url("http://172.16.0.1/"));
    assert!(!is_public_url("http://172.31.255.255/"));
    assert!(!is_public_url("http://192.168.1.1/"));
    assert!(!is_public_url("http://192.168.0.0/"));
}

#[test]
fn rejects_link_local_ipv4() {
    assert!(!is_public_url("http://169.254.0.1/"));
    assert!(!is_public_url("http://169.254.169.254/"));
}

#[test]
fn rejects_carrier_grade_nat_100_64() {
    assert!(!is_public_url("http://100.64.0.1/"));
    assert!(!is_public_url("http://100.127.255.254/"));
}

#[test]
fn rejects_unspecified_and_broadcast_ipv4() {
    assert!(!is_public_url("http://0.0.0.0/"));
    assert!(!is_public_url("http://255.255.255.255/"));
}

#[test]
fn rejects_multicast_ipv4() {
    assert!(!is_public_url("http://224.0.0.1/"));
    assert!(!is_public_url("http://239.255.255.250/"));
}

#[test]
fn rejects_ietf_protocol_assignment_192_0_0() {
    assert!(!is_public_url("http://192.0.0.1/"));
}

// ---------------------------------------------------------------------------
// Must-reject: private / reserved IPv6 ranges
// ---------------------------------------------------------------------------

#[test]
fn rejects_loopback_ipv6() {
    assert!(!is_public_url("http://[::1]/"));
}

#[test]
fn rejects_unique_local_ipv6() {
    assert!(!is_public_url("http://[fd00::1]/"));
    assert!(!is_public_url("http://[fc00::1]/"));
}

#[test]
fn rejects_unspecified_ipv6() {
    assert!(!is_public_url("http://[::]/"));
}
