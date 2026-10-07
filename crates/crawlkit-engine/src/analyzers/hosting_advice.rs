//! Whether the host can set response headers at all.
//!
//! # The defect this exists to prevent
//!
//! A crawl of four static sites on GitHub Pages reported 220 of 1,319 defects
//! (16%) as missing response headers: no `X-Frame-Options`, no CSP, no
//! `Referrer-Policy`. Every one was factually correct and none was fixable —
//! GitHub Pages does not allow custom response headers, so the recommendation
//! "add `X-Frame-Options: DENY`" asked the operator to do something impossible
//! without moving hosts.
//!
//! Advice that cannot be followed is noise, and noise trains people to ignore
//! the report. But the finding itself is *true*: a page on GitHub Pages genuinely
//! is iframe-able. Deleting it would be a false negative, which is the worst
//! class of defect this codebase has.
//!
//! So the finding stays and the *advice* changes. When a forbidding platform is
//! detected, the recommendation names what is actually possible instead of
//! repeating a header name the operator cannot set.
//!
//! # Detection is best-effort, and says so
//!
//! Platforms are recognised from the `Server` header, which is trivial to spoof
//! and is often rewritten by a CDN in front of the origin. Two failure modes
//! follow, both benign:
//!
//! - undetected platform → generic header advice, exactly as before
//! - a proxy reporting someone else's `Server` → advice about a platform the
//!   origin may not be on, which is still true of the response the crawler saw
//!
//! Nothing is suppressed on the strength of this check.

/// A hosting platform that does not allow custom response headers.
pub struct Platform {
    /// Value of the `Server` header that identifies it.
    pub server: &'static str,
    /// The operator-facing name.
    pub name: &'static str,
    /// What actually can be done, given the constraint.
    pub remedy: &'static str,
}

/// Platforms known to forbid custom response headers.
///
/// `server` is matched case-insensitively and by prefix, because hosts append
/// versions (`GitHub.com`, `cloudflare`) or sit behind a proxy that rewrites it.
const PLATFORMS: &[Platform] = &[Platform {
    server: "github.com",
    name: "GitHub Pages",
    remedy: "GitHub Pages serves static files and does not allow custom response \
             headers, so no header can fix this. A `Content-Security-Policy` \
             `<meta>` tag can carry `frame-ancestors` for some browsers but is \
             not a substitute for the header; real clickjacking protection needs \
             hosting that sets headers, or accepting the exposure.",
}];

/// The platform that served this response, when it forbids custom headers.
#[must_use]
pub fn forbidding(server: Option<&str>) -> Option<&'static Platform> {
    let server = server?.trim();
    if server.is_empty() {
        return None;
    }
    PLATFORMS
        .iter()
        .find(|p| server.to_ascii_lowercase().starts_with(p.server))
}

/// Adjust a header recommendation for a platform that cannot set headers.
///
/// Returns the platform's remedy when one is detected, otherwise the original
/// advice unchanged. Callers keep their own text; this only overrides it when
/// there is something more useful to say.
#[must_use]
pub fn adjust<'a>(server: Option<&str>, default_advice: &'a str) -> &'a str {
    match forbidding(server) {
        Some(p) => p.remedy,
        None => default_advice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Case and version suffixes must not prevent recognition.
    #[test]
    fn matches_case_insensitively_and_by_prefix() {
        assert!(forbidding(Some("GitHub.com")).is_some());
        assert!(forbidding(Some("github.com/xyz")).is_some());
        assert!(forbidding(Some("  GitHub.com  ")).is_some());
    }

    /// An absent, empty, or unrelated `Server` header must not match. A wrong
    /// positive here would rewrite advice for every site on an ordinary host.
    #[test]
    fn does_not_match_other_servers() {
        assert!(forbidding(None).is_none());
        assert!(forbidding(Some("")).is_none());
        assert!(forbidding(Some("   ")).is_none());
        assert!(forbidding(Some("nginx/1.24.0")).is_none());
        assert!(forbidding(Some("cloudflare")).is_none());
        assert!(forbidding(Some("Vercel")).is_none());
    }

    /// The advice must name the platform, so the operator can tell a detected
    /// constraint from a guessed one.
    #[test]
    fn remedy_names_the_platform() {
        let p = forbidding(Some("GitHub.com")).expect("github pages");
        assert!(p.remedy.contains(p.name));
        assert!(p.remedy.contains("does not allow custom response headers"));
    }

    /// Undetected platform leaves the caller's advice untouched, so a detection
    /// mistake can only ever cost a more specific sentence, never a wrong one.
    #[test]
    fn undetected_platform_keeps_the_callers_advice() {
        let mine = "Add X-Frame-Options: DENY or CSP frame-ancestors.";
        assert_eq!(adjust(Some("nginx"), mine), mine);
        assert_eq!(adjust(None, mine), mine);
        assert_ne!(adjust(Some("GitHub.com"), mine), mine);
    }

    /// Nothing here suppresses a finding: the point is better advice on a true
    /// finding, not fewer findings. Pinned so a future edit cannot quietly turn
    /// this module into a suppression list.
    #[test]
    fn detection_does_not_remove_the_finding() {
        // The return type carries only text. There is no way to express "skip
        // this finding" through it, and that is deliberate.
        let adjusted = adjust(Some("GitHub.com"), "Add X-Frame-Options: DENY.");
        assert!(!adjusted.is_empty());
    }
}
