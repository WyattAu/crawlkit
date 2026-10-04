//! Advice text for the cross-origin isolation header family (COOP, COEP, CORP).
//!
//! # Why this exists
//!
//! These three headers were originally emitted with unconditional imperative
//! advice — *"Add Cross-Origin-Opener-Policy: same-origin"*, *"Add
//! Cross-Origin-Embedder-Policy: require-corp"* — from nine separate call sites.
//! That advice is actively destructive when followed on a typical site:
//!
//! * `COOP: same-origin` severs `window.opener`, which breaks OAuth and payment
//!   flows that depend on a popup handing a result back to its opener.
//! * `COEP: require-corp` refuses to load any cross-origin subresource that does
//!   not itself send `Cross-Origin-Resource-Policy`. That includes most
//!   analytics, embedded video, and CDN-hosted fonts and scripts, none of which
//!   send CORP.
//! * The two are a package. Turning on either generally requires the other.
//!
//! Google's web.dev guidance explicitly advises **against** deploying COOP and
//! COEP for sites that do not need cross-origin isolation, because of exactly
//! this breakage. Search Central's security-header guidance does not list them.
//!
//! The findings themselves are legitimate and stay: they are all `Info`, they
//! describe a real property of the response, and a site that *does* use
//! `SharedArrayBuffer` or wasm threads has a genuine reason to set these. What
//! was wrong was the framing — advice presented as an instruction rather than a
//! trade-off, which led users to break working sites for no ranking or security
//! benefit in the common case.
//!
//! Kept as one module rather than nine edited strings so the wording cannot drift
//! apart between call sites.

/// Advice for a missing `Cross-Origin-Opener-Policy`.
#[must_use]
pub fn coop_advice() -> &'static str {
    "Only needed if this page uses cross-origin isolation (SharedArrayBuffer, wasm \
     threads). COOP: same-origin severs window.opener, which breaks OAuth and \
     payment popups that pass a result back to their opener. Google's web.dev \
     guidance advises against it otherwise — weigh that before setting it."
}

/// Advice for a missing `Cross-Origin-Embedder-Policy`.
#[must_use]
pub fn coep_advice() -> &'static str {
    "Only needed if this page uses cross-origin isolation (SharedArrayBuffer, wasm \
     threads). COEP refuses to load cross-origin subresources that do not send \
     Cross-Origin-Resource-Policy themselves, which commonly breaks analytics, \
     embedded media and CDN-hosted assets. Google's web.dev guidance advises \
     against it otherwise."
}

/// Advice for a missing `Cross-Origin-Resource-Policy`.
#[must_use]
pub fn corp_advice() -> &'static str {
    "Only applies to resources that should be loaded exclusively by your own pages. \
     CORP: same-origin stops other sites from embedding the resource, and COEP \
     requires it of every subresource it governs -- so it is a prerequisite for \
     COEP, not a general hardening step."
}

/// Advice when the response sets COOP/COEP inconsistently — one without the other.
#[must_use]
pub fn isolation_pair_advice() -> &'static str {
    "COOP and COEP are a package: neither achieves cross-origin isolation alone. \
     Only pursue both if this page uses SharedArrayBuffer or wasm threads, and \
     expect COEP to require CORP from every third-party subresource — which is \
     why most sites should leave both unset."
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of this module: no advice may read as an unconditional
    /// instruction. These assertions are the regression guard, because the
    /// failure mode is a plausible-sounding sentence rather than a crash.
    #[test]
    fn no_advice_is_an_unconditional_imperative() {
        for (name, advice) in [
            ("coop", coop_advice()),
            ("coep", coep_advice()),
            ("corp", corp_advice()),
            ("pair", isolation_pair_advice()),
        ] {
            // A bare instruction is short and unconditional; advice that carries a
            // trade-off is neither. Length is a crude proxy but it fails in the
            // right direction: it cannot be satisfied by a terse command.
            assert!(
                advice.len() > 120,
                "{name} advice is too short to carry a trade-off: {advice:?}"
            );
            assert!(
                advice.contains("only") || advice.contains("Only"),
                "{name} advice must state when it applies: {advice:?}"
            );
            assert!(
                advice.contains('.'),
                "{name} advice should be more than one clause: {advice:?}"
            );
        }
    }

    /// A user who follows COEP or COOP advice on an ordinary site breaks OAuth,
    /// payment and third-party embeds. Both must say so, or the advice is
    /// incomplete in the way that causes damage.
    #[test]
    fn destructive_consequences_are_named() {
        assert!(
            coop_advice().contains("window.opener"),
            "COOP must name the breakage"
        );
        assert!(
            coop_advice().contains("OAuth") || coop_advice().contains("payment"),
            "COOP must name who is affected"
        );
        assert!(
            coep_advice().contains("Cross-Origin-Resource-Policy"),
            "COEP must explain the CORP dependency, which is the actual cause of breakage"
        );
        assert!(
            isolation_pair_advice().contains("SharedArrayBuffer"),
            "the combined advice must say when isolation is actually wanted"
        );
    }

    /// The pair advice exists because COOP and COEP are only useful together; a
    /// reader who sets one without the other gets nothing but the breakage.
    #[test]
    fn pair_advice_explains_the_dependency() {
        let advice = isolation_pair_advice();
        assert!(advice.contains("COEP") && advice.contains("COOP"));
        assert!(advice.contains("CORP"));
    }
}
