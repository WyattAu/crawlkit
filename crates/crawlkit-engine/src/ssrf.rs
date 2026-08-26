use std::net::IpAddr;

/// Checks whether a URL points to a public, routable HTTP(S) target.
///
/// Rejects non-HTTP schemes, localhost, metadata endpoints, and RFC 1918
/// / RFC 4193 / link-local / broadcast / multicast IP addresses.
/// DNS resolution is intentionally not performed; the caller must also
/// enforce redirect and resolver policy at connection time.
///
/// # Examples
///
/// ```
/// assert!(crawlkit_engine::ssrf::is_public_url("https://example.com"));
/// assert!(!crawlkit_engine::ssrf::is_public_url("http://192.168.1.1/"));
/// ```
pub fn is_public_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "localhost" | "localhost.localdomain" | "metadata.google.internal"
    ) {
        return false;
    }
    let ip_host = host.trim_matches(['[', ']']);
    if let Ok(ip) = ip_host.parse::<IpAddr>() {
        return !is_private_ip(ip);
    }
    true
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_multicast()
                || (v4.octets()[0] == 100 && (64..=127).contains(&v4.octets()[1]))
                || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.is_multicast()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_schemes() {
        assert!(!is_public_url("ftp://example.com/file"));
        assert!(!is_public_url("file:///etc/passwd"));
    }

    #[test]
    fn rejects_localhost_and_metadata() {
        assert!(!is_public_url("http://localhost/"));
        assert!(!is_public_url("http://metadata.google.internal/latest"));
    }

    #[test]
    fn rejects_private_ipv4() {
        assert!(!is_public_url("http://127.0.0.1/"));
        assert!(!is_public_url("http://10.0.0.1/api"));
        assert!(!is_public_url("http://169.254.169.254/latest/meta-data"));
        assert!(!is_public_url("http://192.168.1.1/admin"));
        assert!(!is_public_url("http://172.16.0.1/internal"));
    }

    #[test]
    fn rejects_private_ipv6() {
        assert!(!is_public_url("http://[::1]/"));
        assert!(!is_public_url("http://[fd00::1]/"));
    }

    #[test]
    fn rejects_empty_and_malformed() {
        assert!(!is_public_url(""));
        assert!(!is_public_url("not-a-url"));
    }

    #[test]
    fn accepts_valid_public_https() {
        assert!(is_public_url("https://example.com"));
        assert!(is_public_url("https://api.stripe.com/v1/charges"));
        assert!(is_public_url("http://example.com:8080/path?q=1"));
    }
}
