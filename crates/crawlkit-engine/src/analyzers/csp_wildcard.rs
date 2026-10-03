//! CSP source-expression helpers.
//!
//! # Why this exists
//!
//! Several analyzers decided whether a CSP directive was permissive by asking
//! whether the directive text "contains a `*`". That test cannot tell a bare
//! wildcard apart from a *scoped subdomain* wildcard, which is an ordinary and
//! much safer construct:
//!
//! ```text
//! script-src 'self' *.youtube.com *.analytics.google.com   <- scoped, fine
//! script-src *                                             <- any host
//! ```
//!
//! Auditing gov.uk — whose CSP pins a long, specific `script-src` list using
//! `*.youtube.com`, `*.ytimg.com`, `*.analytics.google.com` and a per-response
//! nonce — the substring test reported "CSP script-src wildcard" at **Critical**
//! on all 40 pages crawled. The site has no bare wildcard at all. Flagging a
//! government site's careful policy as critically broken is exactly the kind of
//! alarm that teaches people to ignore an audit tool.
//!
//! Only a bare `*` host expression — which really does permit any origin — is
//! treated as a wildcard here.

/// True when a CSP source expression permits *any* host.
///
/// Recognises the bare forms (`*`, `*:*`, `*://*`). A subdomain wildcard such as
/// `*.example.com` matches only that domain's subdomains and is deliberately
/// **not** reported.
#[must_use]
pub fn is_bare_host_wildcard(expression: &str) -> bool {
    let token = expression.trim();
    if token.is_empty() || token.starts_with('\'') {
        return false; // keyword source such as 'self', 'none', 'unsafe-inline'
    }
    // `*:` and `*://` are the scheme-agnostic spelling of `*`.
    let host = token
        .rsplit_once("://")
        .map_or(token, |(_, rest)| rest);
    let host = host.rsplit_once(':').map_or(host, |(h, _)| h);
    host == "*"
}

/// True when any source expression in a CSP directive is a bare host wildcard.
///
/// `directive` is the text after the directive name, e.g.
/// `" 'self' *.youtube.com"` for `script-src 'self' *.youtube.com`.
#[must_use]
pub fn directive_allows_any_host(directive: &str) -> bool {
    directive
        .split_whitespace()
        .any(is_bare_host_wildcard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_wildcard_is_detected() {
        assert!(is_bare_host_wildcard("*"));
        assert!(is_bare_host_wildcard(" * "));
        assert!(is_bare_host_wildcard("*:*"));
        assert!(is_bare_host_wildcard("*://*"));
    }

    #[test]
    fn subdomain_wildcards_are_not_bare_wildcards() {
        // This is the gov.uk case that produced the false positive.
        assert!(!is_bare_host_wildcard("*.youtube.com"));
        assert!(!is_bare_host_wildcard("*.analytics.google.com"));
        assert!(!is_bare_host_wildcard("*.ytimg.com"));
        assert!(!is_bare_host_wildcard("https://*.example.com"));
    }

    #[test]
    fn keyword_and_host_sources_are_not_wildcards() {
        for token in ["'self'", "'none'", "'unsafe-inline'", "'nonce-abc'"] {
            assert!(!is_bare_host_wildcard(token), "{token} must not match");
        }
        assert!(!is_bare_host_wildcard("www.google-analytics.com"));
        assert!(!is_bare_host_wildcard(""));
    }

    #[test]
    fn gov_uk_script_src_is_not_permissive() {
        let gov_uk = "'self' www.google-analytics.com ssl.google-analytics.com \
                      www.googletagmanager.com *.analytics.google.com www.gstatic.com \
                      *.ytimg.com www.youtube.com 'nonce-bflPMUNqgc1Ceo1S9YFhYA=='";
        assert!(!directive_allows_any_host(gov_uk));
    }

    #[test]
    fn genuinely_permissive_directive_is_caught() {
        assert!(directive_allows_any_host("*"));
        assert!(directive_allows_any_host("'unsafe-inline' * data:"));
    }

    #[test]
    fn port_suffix_does_not_confuse_detection() {
        assert!(!is_bare_host_wildcard("*.example.com:443"));
        assert!(is_bare_host_wildcard("*:*"));
    }
}