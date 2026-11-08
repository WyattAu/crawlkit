//! Planned agent-readiness checks (P2/P3 tiers) — the map, not the territory.
//!
//! `docs/AGENT_READINESS_BACKLOG.md` catalogues 21 checks. This pass
//! implements the P0 tier ([`crate::analyzers::agent_readiness_analyzers`])
//! and the P1 well-known-path tier
//! ([`crate::analyzers::agent_discovery_analyzers`]). The twelve checks below
//! are deliberately **not implemented yet**; each row pins its catalogue
//! number, priority tier, target path, and the governing specification so the
//! next implementer starts from the requirement, not from an empty file —
//! the same "written down rather than discovered later" rule that
//! `tests/no_orphan_analyzers.rs` applies to unregistered analyzers.
//!
//! IDs follow the WSTG convention (`<PREFIX>-<CATEGORY>-<NN>`): stable,
//! quotable, and ordered by backlog number so the catalogue stays the index
//! of record.

/// The backlog file this table mirrors (repo-root relative).
pub const CATALOG_REFERENCE: &str = "docs/AGENT_READINESS_BACKLOG.md";

/// One planned check: what will be implemented, against which spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedCheck {
    /// Stable WSTG-style ID (`AIREADY-<TIER>-<NN>`, NN = backlog number).
    pub id: &'static str,
    /// Priority tier from the backlog (P2 = auth + browser/identity, P3 = DNS
    /// + commerce).
    pub tier: &'static str,
    /// The backlog check number in `CATALOG_REFERENCE`.
    pub backlog_number: u8,
    /// Path probed or evidence source, when the check targets one.
    pub target: &'static str,
    /// Governing specification or working group.
    pub spec: &'static str,
    /// One-line statement of what the check will validate.
    pub summary: &'static str,
}

/// The planned (not-yet-implemented) checks, in backlog order.
pub const PLANNED_CHECKS: &[PlannedCheck] = &[
    PlannedCheck {
        id: "AIREADY-AUTH-07",
        tier: "P2",
        backlog_number: 7,
        target: "/auth.md",
        spec: "auth.md convention",
        summary: "Markdown with an H1 containing \"auth.md\"; self-contained (agent \
                  audience, registration endpoints, credential use) or references OAuth PRM.",
    },
    PlannedCheck {
        id: "AIREADY-AUTH-08",
        tier: "P2",
        backlog_number: 8,
        target: "/.well-known/openid-configuration OR /.well-known/oauth-authorization-server",
        spec: "OIDC Discovery 1.0 / RFC 8414",
        summary: "Discovery document with issuer, authorization_endpoint, token_endpoint, \
                  jwks_uri, grant_types_supported, response_types_supported.",
    },
    PlannedCheck {
        id: "AIREADY-AUTH-09",
        tier: "P2",
        backlog_number: 9,
        target: "/.well-known/oauth-protected-resource",
        spec: "RFC 9728",
        summary: "Protected-resource metadata with resource and authorization_servers[]; \
                  optional scopes_supported; optional WWW-Authenticate resource_metadata on \
                  401s.",
    },
    PlannedCheck {
        id: "AIREADY-BRWS-14",
        tier: "P2",
        backlog_number: 14,
        target: "content negotiation (Accept: text/markdown)",
        spec: "Markdown for Agents convention",
        summary: "Accept: text/markdown returns markdown; HTML remains the browser default.",
    },
    PlannedCheck {
        id: "AIREADY-BRWS-15",
        tier: "P2",
        backlog_number: 15,
        target: "page JS (navigator.modelContext)",
        spec: "webmachinelearning/webmcp",
        summary: "Rendered page registers tools via navigator.modelContext.registerTool() with \
                  name/description/inputSchema/execute and AbortController cleanup.",
    },
    PlannedCheck {
        id: "AIREADY-BRWS-16",
        tier: "P2",
        backlog_number: 16,
        target: "/.well-known/http-message-signatures-directory + request signatures",
        spec: "IETF WebBotAuth WG (web-bot-auth)",
        summary: "JWKS directory with at least one public key; bot requests signed with \
                  Signature-Agent + Signature-Input headers.",
    },
    PlannedCheck {
        id: "AIREADY-DNS-13",
        tier: "P3",
        backlog_number: 13,
        target: "_agents SVCB/HTTPS DNS records (via DoH)",
        spec: "RFC 9460 (SVCB/HTTPS) + DNS-AID draft; default resolver \
               https://cloudflare-dns.com/dns-query, fallback https://dns.google/resolve",
        summary: "`_a2a._agents.example.com SVCB 1 agent.example.com. alpn=\"a2a\" port=443` \
                  records; DNSSEC-signed zones; numeric keyNNNNN for experimental params.",
    },
    PlannedCheck {
        id: "AIREADY-CMRC-17",
        tier: "P3",
        backlog_number: 17,
        target: "/.well-known/acp.json",
        spec: "agenticcommerce.dev (ACP)",
        summary: "protocol.name=\"acp\", protocol.version, absolute http(s) api_base_url, \
                  non-empty transports[], non-empty capabilities.services[].",
    },
    PlannedCheck {
        id: "AIREADY-CMRC-18",
        tier: "P3",
        backlog_number: 18,
        target: "/.well-known/agent-card.json (extension entry)",
        spec: "ap2-protocol.org (AP2) over A2A",
        summary: "Extension entry with uri \
                  https://github.com/google-agentic-commerce/AP2/tree/v0.1.0 and params.roles \
                  (merchant|shopper|credentials-provider|payment-processor); merchant agents \
                  mark required.",
    },
    PlannedCheck {
        id: "AIREADY-CMRC-19",
        tier: "P3",
        backlog_number: 19,
        target: "/openapi.json",
        spec: "mpp.dev / paymentauth.org draft (MPP)",
        summary: "x-payment-info per payable operation (intent charge|session, method \
                  tempo|stripe|lightning|card, amount; optional currency/description) plus \
                  top-level x-service-info.",
    },
    PlannedCheck {
        id: "AIREADY-CMRC-20",
        tier: "P3",
        backlog_number: 20,
        target: "/.well-known/ucp",
        spec: "ucp.dev (UCP)",
        summary: "protocol_version, services, capabilities, endpoints; referenced spec URLs \
                  reachable.",
    },
    PlannedCheck {
        id: "AIREADY-CMRC-21",
        tier: "P3",
        backlog_number: 21,
        target: "HTTP 402 responses on protected API routes",
        spec: "x402.org",
        summary: "402 responses carry the payment requirements (middleware-based).",
    },
];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use std::collections::BTreeSet;

    use super::*;

    /// The twelve P2/P3 backlog numbers, each mapped exactly once, and the
    /// tier assignment matching the catalogue's PRIORITY section.
    #[test]
    fn planned_checks_cover_exactly_the_p2_p3_backlog_numbers() {
        let numbers: Vec<u8> = PLANNED_CHECKS.iter().map(|c| c.backlog_number).collect();
        let unique: BTreeSet<u8> = numbers.iter().copied().collect();
        assert_eq!(
            unique,
            BTreeSet::from([7, 8, 9, 13, 14, 15, 16, 17, 18, 19, 20, 21]),
            "planned table must cover the P2/P3 checks exactly once: {numbers:?}"
        );
    }

    #[test]
    fn tier_labels_match_the_backlog_priority_split() {
        for check in PLANNED_CHECKS {
            let expect_p2 = [7, 8, 9, 14, 15, 16].contains(&check.backlog_number);
            let expect_p3 = [13, 17, 18, 19, 20, 21].contains(&check.backlog_number);
            assert!(
                expect_p2 || expect_p3,
                "check {} is in neither tier list (catalogue drift)",
                check.id
            );
            let expected_tier = if expect_p2 { "P2" } else { "P3" };
            assert_eq!(
                check.tier, expected_tier,
                "check {} tier disagrees with the catalogue",
                check.id
            );
        }
    }

    #[test]
    fn ids_are_unique_and_follow_the_wstg_style() {
        let mut ids: Vec<&str> = PLANNED_CHECKS.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let unique = ids.iter().cloned().collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), unique.len(), "IDs must be unique: {ids:?}");
        for id in ids {
            let parts: Vec<&str> = id.split('-').collect();
            assert_eq!(
                parts.len(),
                3,
                "WSTG-style IDs are PREFIX-CATEGORY-NN: {id}"
            );
            assert_eq!(parts[0], "AIREADY");
            assert!(
                parts[2].chars().all(|c| c.is_ascii_digit()) && parts[2].len() == 2,
                "trailing segment must be the two-digit backlog number: {id}"
            );
        }
    }

    #[test]
    fn every_planned_check_names_a_spec_and_a_summary() {
        for check in PLANNED_CHECKS {
            assert!(!check.spec.is_empty(), "{} has no spec reference", check.id);
            assert!(!check.target.is_empty(), "{} has no target", check.id);
            assert!(
                check.summary.len() > 40,
                "{} summary must state what will be validated",
                check.id
            );
        }
    }
}
