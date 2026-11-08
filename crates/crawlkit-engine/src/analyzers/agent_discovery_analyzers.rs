//! Agent-discovery surface (P1 tier) — live probes of the well-known paths
//! where a site publishes its agent-facing API surface.
//!
//! # Why this exists
//!
//! `docs/AGENT_READINESS_BACKLOG.md` catalogues 21 checks an agent-oriented
//! audit must run. This module implements the P1 well-known-path tier —
//! checks 5, 6, 10, 11, 12, "the agent discovery surface":
//!
//! | Code | Backlog check | Path / evidence | Requirement (quoted) |
//! |------|---------------|-----------------|----------------------|
//! | `AIREADY-APICATALOG` | 5 | `/.well-known/api-catalog` | "Content-Type application/linkset+json, 200; `linkset` array; each entry `anchor` + relations service-desc/service-doc" |
//! | `AIREADY-LINKHDR` | 6 | origin root `/` | "`Link:` headers with rel api-catalog / service-desc / service-doc / describedby. Multiple headers or comma-separated both valid" |
//! | `AIREADY-A2A` | 10 | `/.well-known/agent-card.json` | "name, version, description, supportedInterfaces (service URL + transport), capabilities, skills[] (id/name/description)" |
//! | `AIREADY-AGSKILLS` | 11 | `/.well-known/agent-skills/index.json` | "`$schema` = …/discovery/0.2.0/schema.json; skills[] each with name (lowercase+hyphens), type (skill-md\|archive), description, url, digest (sha256:{hex})" |
//! | `AIREADY-MCPCARD` | 12 | `/.well-known/mcp/server-card.json` | "serverInfo{name,version}, transport endpoint URL, capabilities (tools/resources/prompts)" |
//!
//! Each check emits exactly one finding per page: failures carry a Warning
//! whose recommendation quotes the requirement; passes carry an Info finding,
//! matching the registry convention for positive results.
//!
//! # Probing, rate limiting, and safety
//!
//! Probes are **opt-in** because they add network requests to the analyzer
//! phase (the G7 invariant keeps every other analyzer passive). Enable with
//! `CRAWLKIT_AGENT_DISCOVERY=1` in the environment, or construct via
//! [`AgentDiscoveryAnalyzer::with_probes`] /
//! [`AgentDiscoveryAnalyzer::with_probes_and_prober`]. When disabled the
//! analyzer emits nothing, so default registry runs (tests, fixtures) are
//! unaffected.
//!
//! When enabled, each of the five paths is fetched at most **once per
//! origin** per crawl (results are memoized), through the existing
//! [`crate::ratelimit::RateLimiter`] (1 rps per domain), on the dedicated
//! blocking runtime ([`crate::plugin::fetch_runtime`]), after the
//! [`crate::ssrf::is_public_url`] guard rejects private/loopback/metadata
//! targets. Response bodies are capped at 256 KiB — the card formats are
//! small JSON documents.
//!
//! Unit tests inject a fake prober via
//! [`AgentDiscoveryAnalyzer::with_probes_and_prober`], so shape validation is
//! fixture-tested without any network.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// Response body cap per probe. Card formats are small JSON documents; the
/// cap bounds memory when a site serves a tarball or a log at the path.
const MAX_BODY_BYTES: usize = 256 * 1024;

/// Probe path order, shared by the prober loop and the cached results.
const PROBE_PATHS: [&str; 5] = [
    "/.well-known/api-catalog",             // APICATALOG
    "/",                                    // LINKHDR (origin root)
    "/.well-known/agent-card.json",         // A2A
    "/.well-known/agent-skills/index.json", // AGSKILLS
    "/.well-known/mcp/server-card.json",    // MCPCARD
];

const IDX_API_CATALOG: usize = 0;
const IDX_LINK_HEADERS: usize = 1;
const IDX_A2A_CARD: usize = 2;
const IDX_AGENT_SKILLS: usize = 3;
const IDX_MCP_CARD: usize = 4;

/// What a probe observed. `status == None` means the probe could not complete
/// (blocked by the SSRF guard, DNS/connect failure, rate-limit timeout, or
/// the prober was never enabled).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProbeOutcome {
    /// Final HTTP status, if a response arrived.
    status: Option<u16>,
    /// Media type from Content-Type (before any `;` parameter).
    content_type: Option<String>,
    /// Body, UTF-8 (lossy), capped at [`MAX_BODY_BYTES`].
    body: Option<String>,
    /// Every `Link:` header value, in response order.
    link_headers: Vec<String>,
}

impl ProbeOutcome {
    fn unreachable() -> Self {
        Self {
            status: None,
            content_type: None,
            body: None,
            link_headers: Vec::new(),
        }
    }

    /// A 200-shaped outcome with a content type and body. Test-only: the
    /// real prober builds outcomes directly from a response.
    #[cfg(test)]
    fn reachable(status: u16, content_type: Option<&str>, body: Option<&str>) -> Self {
        Self {
            status: Some(status),
            content_type: content_type.map(|ct| {
                ct.split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase()
            }),
            body: body.map(|b| b.chars().take(MAX_BODY_BYTES).collect()),
            link_headers: Vec::new(),
        }
    }

    /// Whether a 200 response arrived.
    fn is_ok(&self) -> bool {
        self.status == Some(200)
    }
}

/// The fetch function the analyzer uses. Takes an absolute URL, returns what
/// the server answered (or [`ProbeOutcome::unreachable`]-shaped data).
pub(crate) type Prober = Arc<dyn Fn(&str) -> ProbeOutcome + Send + Sync>;

/// The finding category for the whole agent-readiness group (shared with the
/// P0 analyzer so both tiers group together in reports).
fn agent_category() -> IssueCategory {
    IssueCategory::Custom("agent-readiness".to_string())
}

/// Agent-discovery surface probes: api-catalog, Link headers, A2A card,
/// agent-skills index, and MCP server card. Opt-in via
/// `CRAWLKIT_AGENT_DISCOVERY=1`; emits nothing when disabled.
pub struct AgentDiscoveryAnalyzer {
    /// Whether probes run. `false` ⇒ `analyze` always returns `vec![]`.
    enabled: bool,
    /// The fetch function. `None` when probes were never wired (non-`full`
    /// build without an injected prober).
    prober: Option<Prober>,
    /// Per-origin probe results, memoized so the five GETs run once per host
    /// per crawl rather than once per page.
    cache: Mutex<HashMap<String, Arc<[ProbeOutcome; PROBE_PATHS.len()]>>>,
    /// Origins currently being probed (guards duplicate work when the same
    /// origin is analyzed concurrently on two rayon workers).
    in_flight: Mutex<HashSet<String>>,
}

impl Default for AgentDiscoveryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDiscoveryAnalyzer {
    /// Disabled-by-default constructor used by the registry: probes run only
    /// when `CRAWLKIT_AGENT_DISCOVERY=1` (and the `full` feature provides the
    /// HTTP stack). Under non-`full` builds this never enables.
    #[must_use]
    pub fn new() -> Self {
        let enabled = cfg!(feature = "full")
            && std::env::var("CRAWLKIT_AGENT_DISCOVERY").as_deref() == Ok("1");
        Self {
            enabled,
            prober: None,
            cache: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashSet::new()),
        }
    }

    /// Enable live probing with the real HTTP stack: SSRF guard, the existing
    /// rate limiter (1 rps per domain), and the dedicated blocking runtime.
    #[cfg(feature = "full")]
    #[must_use]
    pub fn with_probes() -> Self {
        Self {
            enabled: true,
            prober: Some(Arc::new(real_prober) as Prober),
            cache: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashSet::new()),
        }
    }

    /// Enable probing with an injected fetch function. Test-only: unit tests
    /// feed canned responses through it so shape validation runs without a
    /// network. External enablement is `CRAWLKIT_AGENT_DISCOVERY=1`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_probes_and_prober(prober: Prober) -> Self {
        Self {
            enabled: true,
            prober: Some(prober),
            cache: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashSet::new()),
        }
    }

    /// Memoized per-origin probe of all five paths. The first page of an
    /// origin pays the five (rate-limited) GETs; every later page reuses the
    /// results, so a 1000-page crawl of one host adds five requests, not
    /// five thousand.
    fn probe_set(&self, origin: &str) -> Option<Arc<[ProbeOutcome; PROBE_PATHS.len()]>> {
        if !self.enabled {
            return None;
        }
        let Some(prober) = &self.prober else {
            return None;
        };

        {
            let cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(set) = cache.get(origin) {
                return Some(Arc::clone(set));
            }
        }

        // Serialize per origin: two rayon workers on the same host would
        // otherwise both run the probe loop. The loser re-reads the cache
        // after waiting.
        {
            let mut in_flight = self
                .in_flight
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !in_flight.insert(origin.to_string()) {
                // Another worker is probing this origin. Under the analyzer
                // isolation contract (findings must be deterministic per
                // page) the honest answer while a probe runs is "no findings
                // yet": the caching worker will emit the full set on its
                // next page.
                return None;
            }
        }

        let mut results: [ProbeOutcome; PROBE_PATHS.len()] =
            std::array::from_fn(|_| ProbeOutcome::unreachable());
        for (idx, path) in PROBE_PATHS.iter().enumerate() {
            results[idx] = prober(&format!("{origin}{path}"));
        }

        let set = Arc::new(results);
        {
            let mut in_flight = self
                .in_flight
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            in_flight.remove(origin);
        }
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(origin.to_string(), Arc::clone(&set));
        Some(set)
    }
}

impl Analyzer for AgentDiscoveryAnalyzer {
    fn name(&self) -> &str {
        "agent-discovery"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        if !self.enabled {
            return Vec::new();
        }
        let url = ctx.page.url.to_string();
        let Some(origin) = origin_of(&url) else {
            return Vec::new();
        };
        let Some(set) = self.probe_set(&origin) else {
            return Vec::new();
        };

        vec![
            api_catalog_finding(&url, &set[IDX_API_CATALOG]),
            link_header_finding(&url, &set[IDX_LINK_HEADERS]),
            a2a_card_finding(&url, &set[IDX_A2A_CARD]),
            agent_skills_finding(&url, &set[IDX_AGENT_SKILLS]),
            mcp_card_finding(&url, &set[IDX_MCP_CARD]),
        ]
    }
}

/// `scheme://host[:port]` of `url`, with a trailing slash — the probe prefix.
fn origin_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let scheme = parsed.scheme().to_string();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let host = parsed.host_str()?;
    let port = match (parsed.port_or_known_default(), scheme.as_str()) {
        (Some(p), "http") if p != 80 => format!(":{p}"),
        (Some(p), "https") if p != 443 => format!(":{p}"),
        _ => String::new(),
    };
    Some(format!("{scheme}://{host}{port}"))
}

/// Build the one finding for a check from its verdict parts.
fn finding_for(
    code: &'static str,
    title: String,
    description: String,
    recommendation: String,
    severity: Severity,
    url: &str,
) -> Finding {
    Finding {
        severity,
        category: agent_category(),
        code: code.to_string(),
        title,
        description,
        url: url.to_string(),
        recommendation,
    }
}

/// The shared verdict shape each validator produces before wrapping.
struct Verdict {
    severity: Severity,
    title: String,
    description: String,
    recommendation: String,
}

/// Wrap a validator verdict, prefixing unreachable probes.
fn verdict_finding(code: &'static str, verdict: Verdict, url: &str) -> Finding {
    finding_for(
        code,
        verdict.title,
        verdict.description,
        verdict.recommendation,
        verdict.severity,
        url,
    )
}

/// The verdict for a probe that never got a response (blocked, DNS failure,
/// rate-limit timeout). Each check reports it once rather than pretending.
fn unreachable_verdict(path: &str, requirement: &str) -> Verdict {
    Verdict {
        severity: Severity::Warning,
        title: format!("probe of {path} could not complete"),
        description: format!(
            "The probe of {path} received no HTTP response (blocked by the SSRF guard, \
             DNS/connect failure, or rate-limit timeout), so this check could not be \
             assessed against the live site. Requirement: \"{requirement}\"."
        ),
        recommendation: format!(
            "Verify {path} is reachable over the public internet and answers with the \
             documented response. Requirement: \"{requirement}\"."
        ),
    }
}

// ---------------------------------------------------------------------------
// Check 5 — /.well-known/api-catalog (RFC 9727)
// ---------------------------------------------------------------------------

/// Validate the api-catalog probe: 200, `application/linkset+json`, a
/// `linkset` array whose entries carry `anchor` plus a service-desc or
/// service-doc relation.
fn api_catalog_verdict(outcome: &ProbeOutcome) -> Verdict {
    let requirement = "Content-Type application/linkset+json, 200; `linkset` array; each \
                       entry `anchor` + relations service-desc/service-doc (+optional status)";
    if outcome.status.is_none() {
        return unreachable_verdict("/.well-known/api-catalog", requirement);
    }
    if !outcome.is_ok() {
        return Verdict {
            severity: Severity::Warning,
            title: format!(
                "/.well-known/api-catalog answered HTTP {}",
                outcome
                    .status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "?".into()),
            ),
            description: format!(
                "Agent-readiness check 5 (RFC 9727) requires /.well-known/api-catalog to \
                 answer HTTP 200; the server answered HTTP {}. Without a catalog, agents \
                 cannot discover the site's service descriptions.",
                outcome
                    .status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "?".into()),
            ),
            recommendation: format!(
                "Serve an api-catalog document at /.well-known/api-catalog with HTTP 200. \
                 Requirement: \"{requirement}\"."
            ),
        };
    }
    if outcome.content_type.as_deref() != Some("application/linkset+json") {
        return Verdict {
            severity: Severity::Warning,
            title: "api-catalog is not served as application/linkset+json".to_string(),
            description: format!(
                "/.well-known/api-catalog answered 200 but with Content-Type {:?}; RFC 9727 \
                 requires application/linkset+json. Agents keying on the media type will not \
                 recognize the document.",
                outcome.content_type.as_deref().unwrap_or("(missing)"),
            ),
            recommendation: format!(
                "Set `Content-Type: application/linkset+json` on /.well-known/api-catalog. \
                 Requirement: \"{requirement}\"."
            ),
        };
    }
    let Some(body) = &outcome.body else {
        return Verdict {
            severity: Severity::Warning,
            title: "api-catalog response had no readable body".to_string(),
            description: "/.well-known/api-catalog answered 200 with the right media type, \
                          but no body was captured to validate the linkset shape."
                .to_string(),
            recommendation: "Re-run the audit if this is unexpected; the probe caps bodies at \
                             256 KiB."
                .to_string(),
        };
    };
    let Some(entries) = parse_linkset(body) else {
        return Verdict {
            severity: Severity::Warning,
            title: "api-catalog body is not a linkset document".to_string(),
            description: "/.well-known/api-catalog answered 200 as \
                          application/linkset+json, but the body is not a JSON object with a \
                          non-empty `linkset` array. Requirement quoted from the backlog: \
                          \"`linkset` array; each entry `anchor` + relations \
                          service-desc/service-doc\"."
                .to_string(),
            recommendation: format!(
                "Publish a JSON object with a `linkset` array whose entries carry `anchor` \
                 and service-desc/service-doc relations. Requirement: \"{requirement}\"."
            ),
        };
    };
    if entries.is_empty() {
        return Verdict {
            severity: Severity::Warning,
            title: "api-catalog linkset array is empty".to_string(),
            description: "/.well-known/api-catalog parses as a linkset document but declares \
                          no entries, so no service is discoverable. Requirement: \
                          \"`linkset` array; each entry `anchor` + relations \
                          service-desc/service-doc\"."
                .to_string(),
            recommendation: format!(
                "Add one entry per service, each with `anchor` and a service-desc or \
                 service-doc relation. Requirement: \"{requirement}\"."
            ),
        };
    }

    let mut problems: Vec<String> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let has_anchor = entry
            .get("anchor")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|a| !a.is_empty());
        if !has_anchor {
            problems.push(format!("entry {i}: missing `anchor`"));
        }
        let has_relation = ["service-desc", "service-doc"]
            .iter()
            .any(|rel| entry.get(*rel).is_some_and(|v| !v.is_null()));
        if !has_relation {
            problems.push(format!(
                "entry {i}: neither service-desc nor service-doc relation present"
            ));
        }
    }
    if problems.is_empty() {
        Verdict {
            severity: Severity::Info,
            title: "api-catalog is a valid linkset document".to_string(),
            description: format!(
                "/.well-known/api-catalog answered 200 as application/linkset+json with a \
                 `linkset` array of {} entrie(s), each carrying `anchor` and a \
                 service-desc/service-doc relation, satisfying agent-readiness check 5 \
                 (RFC 9727).",
                entries.len(),
            ),
            recommendation: "No action required for this check. Keep the catalog in sync with \
                             the published services."
                .to_string(),
        }
    } else {
        Verdict {
            severity: Severity::Warning,
            title: "api-catalog linkset entries are incomplete".to_string(),
            description: format!(
                "/.well-known/api-catalog parses as a linkset document with {} entrie(s), but: \
                 {}. Requirement: \"each entry `anchor` + relations service-desc/service-doc\".",
                entries.len(),
                problems.join("; "),
            ),
            recommendation: format!(
                "Give every linkset entry an `anchor` and at least one of service-desc / \
                 service-doc. Requirement: \"{requirement}\"."
            ),
        }
    }
}

/// Parse a linkset+json body: `{"linkset": [ … ]}`. Returns the entry array.
fn parse_linkset(body: &str) -> Option<Vec<serde_json::Value>> {
    let root: serde_json::Value = serde_json::from_str(body).ok()?;
    root.get("linkset")?.as_array().cloned()
}

fn api_catalog_finding(url: &str, outcome: &ProbeOutcome) -> Finding {
    verdict_finding("AIREADY-APICATALOG", api_catalog_verdict(outcome), url)
}

// ---------------------------------------------------------------------------
// Check 6 — Link headers on the origin root (RFC 8288 + RFC 9727 §3)
// ---------------------------------------------------------------------------

/// The rel values the backlog check names.
const REQUIRED_LINK_RELS: [&str; 4] = ["api-catalog", "service-desc", "service-doc", "describedby"];

/// Extract `rel="…"` tokens from one Link-header value. Comma-separated
/// header values have already been split by the caller.
fn link_rel_tokens(value: &str) -> Vec<String> {
    let mut rels = Vec::new();
    for param in value.split(';').skip(1) {
        let param = param.trim();
        let Some((key, val)) = param.split_once('=') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case("rel") {
            continue;
        }
        let val = val.trim().trim_matches('"').trim_matches('\'');
        for token in val.split_whitespace() {
            rels.push(
                token
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_ascii_lowercase(),
            );
        }
    }
    rels
}

/// All rel tokens found across the probe's `Link:` headers, comma-split (RFC
/// 8288 allows several links in one header or several headers — both shapes
/// are accepted here).
fn observed_link_rels(outcome: &ProbeOutcome) -> Vec<String> {
    let mut rels = Vec::new();
    for header in &outcome.link_headers {
        for link in header.split(',') {
            rels.extend(link_rel_tokens(link));
        }
    }
    rels.sort();
    rels.dedup();
    rels
}

fn link_header_verdict(outcome: &ProbeOutcome) -> Verdict {
    let requirement = "homepage returns `Link:` headers with rel api-catalog / service-desc / \
                       service-doc / describedby. Multiple headers or comma-separated both valid";
    if outcome.status.is_none() {
        return unreachable_verdict("the origin root (/)", requirement);
    }
    if !outcome.is_ok() {
        return Verdict {
            severity: Severity::Warning,
            title: "origin root did not answer 200; Link headers not assessed".to_string(),
            description: format!(
                "The Link-header check inspects the homepage response, which answered HTTP \
                 {} instead of 200, so agent-readiness check 6 could not be assessed.",
                outcome
                    .status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "?".into()),
            ),
            recommendation: format!(
                "Serve the homepage with HTTP 200 and publish `Link:` headers describing the \
                 API surface. Requirement: \"{requirement}\"."
            ),
        };
    }
    let rels = observed_link_rels(outcome);
    let found: Vec<&str> = REQUIRED_LINK_RELS
        .iter()
        .copied()
        .filter(|r| rels.iter().any(|obs| obs == r))
        .collect();
    if found.is_empty() {
        Verdict {
            severity: Severity::Warning,
            title: "no agent-relevant Link: headers on the homepage".to_string(),
            description: format!(
                "The homepage response carries no `Link:` header with rel api-catalog, \
                 service-desc, service-doc, or describedby ({} header(s) seen). RFC 8288 \
                 defines the Link header; RFC 9727 §3 asks the homepage to point agents at \
                 the catalog and service documents with these relations.",
                outcome.link_headers.len(),
            ),
            recommendation: format!(
                "Add `Link: <https://…/.well-known/api-catalog>; rel=\"api-catalog\"` (and \
                 service-desc / service-doc / describedby as applicable) to homepage \
                 responses. Requirement: \"{requirement}\"."
            ),
        }
    } else {
        Verdict {
            severity: Severity::Info,
            title: "homepage Link: headers advertise the API surface".to_string(),
            description: format!(
                "The homepage carries `Link:` headers with the agent-relevant relations \
                 {}. Multiple headers and comma-separated values are both valid per RFC \
                 8288, satisfying agent-readiness check 6.",
                found.join(", "),
            ),
            recommendation: "No action required for this check. Keep the Link headers aligned \
                             with the published catalog."
                .to_string(),
        }
    }
}

fn link_header_finding(url: &str, outcome: &ProbeOutcome) -> Finding {
    verdict_finding("AIREADY-LINKHDR", link_header_verdict(outcome), url)
}

// ---------------------------------------------------------------------------
// Check 10 — /.well-known/agent-card.json (A2A)
// ---------------------------------------------------------------------------

/// Validate the A2A Agent Card: name, version, description,
/// supportedInterfaces (each url + transport), capabilities, skills[]
/// (each id/name/description).
fn a2a_card_verdict(outcome: &ProbeOutcome) -> Verdict {
    let requirement = "name, version, description, supportedInterfaces (service URL + \
                       transport), capabilities, skills[] (id/name/description)";
    let path = "/.well-known/agent-card.json";
    if outcome.status.is_none() {
        return unreachable_verdict(path, requirement);
    }
    let Some(card) = probe_json(outcome) else {
        return missing_or_unparseable_verdict(outcome, path, requirement);
    };

    let mut problems: Vec<String> = Vec::new();
    for field in ["name", "version", "description"] {
        if !card
            .get(field)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|v| !v.is_empty())
        {
            problems.push(format!("missing `{field}` string"));
        }
    }
    match card.get("supportedInterfaces") {
        Some(serde_json::Value::Array(interfaces)) if !interfaces.is_empty() => {
            for (i, iface) in interfaces.iter().enumerate() {
                for field in ["url", "transport"] {
                    if iface
                        .get(field)
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(|v| v.is_empty())
                    {
                        problems.push(format!("supportedInterfaces[{i}]: missing `{field}`"));
                    }
                }
            }
        }
        _ => problems.push("missing non-empty `supportedInterfaces` array".to_string()),
    }
    if !card
        .get("capabilities")
        .is_some_and(serde_json::Value::is_object)
    {
        problems.push("missing `capabilities` object".to_string());
    }
    match card.get("skills") {
        Some(serde_json::Value::Array(skills)) if !skills.is_empty() => {
            for (i, skill) in skills.iter().enumerate() {
                for field in ["id", "name", "description"] {
                    if skill
                        .get(field)
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(|v| v.is_empty())
                    {
                        problems.push(format!("skills[{i}]: missing `{field}`"));
                    }
                }
            }
        }
        _ => problems.push("missing non-empty `skills` array".to_string()),
    }

    if problems.is_empty() {
        Verdict {
            severity: Severity::Info,
            title: "A2A Agent Card is complete".to_string(),
            description: "/.well-known/agent-card.json answered 200 with name, version, \
                          description, supportedInterfaces (each with service URL and \
                          transport), capabilities, and skills entries carrying id, name, and \
                          description — satisfying agent-readiness check 10."
                .to_string(),
            recommendation: "No action required for this check. Bump the card version when the \
                             agent surface changes."
                .to_string(),
        }
    } else {
        Verdict {
            severity: Severity::Warning,
            title: "A2A Agent Card is missing required fields".to_string(),
            description: format!(
                "/.well-known/agent-card.json {} but is incomplete: {}. Agents consuming the \
                 A2A protocol need every field to evaluate and contact the agent.",
                served_clause(outcome),
                problems.join("; "),
            ),
            recommendation: format!(
                "Complete the card: name, version, description, supportedInterfaces (each \
                 with url + transport), capabilities, and skills entries (id/name/description). \
                 Requirement: \"{requirement}\"."
            ),
        }
    }
}

fn a2a_card_finding(url: &str, outcome: &ProbeOutcome) -> Finding {
    verdict_finding("AIREADY-A2A", a2a_card_verdict(outcome), url)
}

// ---------------------------------------------------------------------------
// Check 11 — /.well-known/agent-skills/index.json
// ---------------------------------------------------------------------------

/// The `$schema` URI the backlog pins for the agent-skills discovery index.
const AGENT_SKILLS_SCHEMA: &str = "https://schemas.agentskills.io/discovery/0.2.0/schema.json";

/// Skill names must be lowercase and hyphenated (`^[a-z0-9]+(-[a-z0-9]+)*$`).
fn is_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

/// `sha256:{64 hex}`.
fn is_sha256_digest(digest: &str) -> bool {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn agent_skills_verdict(outcome: &ProbeOutcome) -> Verdict {
    let requirement = "`$schema` = https://schemas.agentskills.io/discovery/0.2.0/schema.json; \
                       skills[] each with name (lowercase+hyphens), type (skill-md|archive), \
                       description, url, digest (sha256:{hex})";
    let path = "/.well-known/agent-skills/index.json";
    if outcome.status.is_none() {
        return unreachable_verdict(path, requirement);
    }
    let Some(index) = probe_json(outcome) else {
        return missing_or_unparseable_verdict(outcome, path, requirement);
    };

    let mut problems: Vec<String> = Vec::new();
    match index.get("$schema").and_then(serde_json::Value::as_str) {
        Some(schema) if schema == AGENT_SKILLS_SCHEMA => {}
        Some(other) => problems.push(format!(
            "`$schema` is {other:?} but must be {AGENT_SKILLS_SCHEMA:?}"
        )),
        None => problems.push("missing `$schema`".to_string()),
    }
    match index.get("skills") {
        Some(serde_json::Value::Array(skills)) if !skills.is_empty() => {
            for (i, skill) in skills.iter().enumerate() {
                match skill.get("name").and_then(serde_json::Value::as_str) {
                    Some(name) if !is_skill_name(name) => {
                        problems.push(format!(
                            "skills[{i}]: name {name:?} is not lowercase+hyphens"
                        ));
                    }
                    None => problems.push(format!("skills[{i}]: missing `name`")),
                    _ => {}
                }
                match skill.get("type").and_then(serde_json::Value::as_str) {
                    Some("skill-md" | "archive") => {}
                    Some(other) => problems.push(format!(
                        "skills[{i}]: type {other:?} is not skill-md|archive"
                    )),
                    None => problems.push(format!("skills[{i}]: missing `type`")),
                }
                for field in ["description", "url"] {
                    if skill
                        .get(field)
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(|v| v.is_empty())
                    {
                        problems.push(format!("skills[{i}]: missing `{field}`"));
                    }
                }
                match skill.get("digest").and_then(serde_json::Value::as_str) {
                    Some(digest) if !is_sha256_digest(digest) => {
                        problems.push(format!(
                            "skills[{i}]: digest {digest:?} is not sha256:{{64 hex}}"
                        ));
                    }
                    None => problems.push(format!("skills[{i}]: missing `digest`")),
                    _ => {}
                }
            }
        }
        _ => problems.push("missing non-empty `skills` array".to_string()),
    }

    if problems.is_empty() {
        Verdict {
            severity: Severity::Info,
            title: "agent-skills index validates".to_string(),
            description: "/.well-known/agent-skills/index.json answered 200 with the pinned \
                          $schema and a skills array whose entries all carry a \
                          lowercase-hyphenated name, a skill-md|archive type, a description, a \
                          url, and a sha256 digest — satisfying agent-readiness check 11."
                .to_string(),
            recommendation: "No action required for this check. Update digests when skill \
                             artifacts change."
                .to_string(),
        }
    } else {
        Verdict {
            severity: Severity::Warning,
            title: "agent-skills index fails validation".to_string(),
            description: format!(
                "/.well-known/agent-skills/index.json {} but: {}. Consumers of the discovery \
                 index reject entries that do not match the pinned schema.",
                served_clause(outcome),
                problems.join("; "),
            ),
            recommendation: format!(
                "Fix the entries to match the pinned schema ({AGENT_SKILLS_SCHEMA}): name \
                 lowercase+hyphens, type skill-md|archive, description, url, digest \
                 sha256:{{hex}}. Requirement: \"{requirement}\"."
            ),
        }
    }
}

fn agent_skills_finding(url: &str, outcome: &ProbeOutcome) -> Finding {
    verdict_finding("AIREADY-AGSKILLS", agent_skills_verdict(outcome), url)
}

// ---------------------------------------------------------------------------
// Check 12 — /.well-known/mcp/server-card.json (SEP-1649)
// ---------------------------------------------------------------------------

fn mcp_card_verdict(outcome: &ProbeOutcome) -> Verdict {
    let requirement = "serverInfo{name,version}, transport endpoint URL, capabilities \
                       (tools/resources/prompts)";
    let path = "/.well-known/mcp/server-card.json";
    if outcome.status.is_none() {
        return unreachable_verdict(path, requirement);
    }
    let Some(card) = probe_json(outcome) else {
        return missing_or_unparseable_verdict(outcome, path, requirement);
    };

    let mut problems: Vec<String> = Vec::new();
    match card.get("serverInfo") {
        Some(info) if info.is_object() => {
            for field in ["name", "version"] {
                if info
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|v| v.is_empty())
                {
                    problems.push(format!("serverInfo: missing `{field}`"));
                }
            }
        }
        _ => problems.push("missing `serverInfo` object".to_string()),
    }
    // SEP-1649 carries the transport endpoint as an absolute URL. Accept the
    // `endpoint` spelling and the `url` spelling; anything else (or a
    // non-http(s) value) fails the check.
    let endpoint = card
        .get("endpoint")
        .or_else(|| card.get("url"))
        .and_then(serde_json::Value::as_str);
    match endpoint {
        Some(u) if is_absolute_http_url(u) => {}
        Some(u) => problems.push(format!("transport endpoint {u:?} is not an http(s) URL")),
        None => problems.push("missing transport endpoint (`endpoint` or `url`)".to_string()),
    }
    match card.get("capabilities") {
        Some(caps) if caps.is_object() => {
            let any = ["tools", "resources", "prompts"]
                .iter()
                .any(|cap| caps.get(*cap).is_some_and(|v| !v.is_null()));
            if !any {
                problems.push(
                    "capabilities object declares none of tools/resources/prompts".to_string(),
                );
            }
        }
        _ => problems.push("missing `capabilities` object".to_string()),
    }

    if problems.is_empty() {
        Verdict {
            severity: Severity::Info,
            title: "MCP Server Card is complete".to_string(),
            description: "/.well-known/mcp/server-card.json answered 200 with \
                          serverInfo{name,version}, an http(s) transport endpoint URL, and a \
                          capabilities object declaring at least one of \
                          tools/resources/prompts — satisfying agent-readiness check 12 \
                          (SEP-1649)."
                .to_string(),
            recommendation: "No action required for this check. Keep the card in sync with the \
                             server's tool surface."
                .to_string(),
        }
    } else {
        Verdict {
            severity: Severity::Warning,
            title: "MCP Server Card is missing required fields".to_string(),
            description: format!(
                "/.well-known/mcp/server-card.json {} but: {}. Agents discovering an MCP \
                 server need the identity, the transport endpoint, and the capability list.",
                served_clause(outcome),
                problems.join("; "),
            ),
            recommendation: format!(
                "Complete the card: serverInfo{{name,version}}, an http(s) transport endpoint \
                 URL, and capabilities declaring tools/resources/prompts. Requirement: \
                 \"{requirement}\"."
            ),
        }
    }
}

fn mcp_card_finding(url: &str, outcome: &ProbeOutcome) -> Finding {
    verdict_finding("AIREADY-MCPCARD", mcp_card_verdict(outcome), url)
}

// ---------------------------------------------------------------------------
// Shared verdict helpers
// ---------------------------------------------------------------------------

/// Parse the probe body as JSON, if a 200 body was captured.
fn probe_json(outcome: &ProbeOutcome) -> Option<serde_json::Value> {
    if !outcome.is_ok() {
        return None;
    }
    let body = outcome.body.as_deref()?;
    serde_json::from_str(body).ok()
}

/// "answered 200" / "answered HTTP 404" — the served clause validators quote.
fn served_clause(outcome: &ProbeOutcome) -> String {
    match outcome.status {
        Some(200) => "answered 200".to_string(),
        Some(status) => format!("answered HTTP {status}"),
        None => "received no response".to_string(),
    }
}

/// The verdict for a non-200 or unparseable body on a JSON card path.
fn missing_or_unparseable_verdict(
    outcome: &ProbeOutcome,
    path: &str,
    requirement: &str,
) -> Verdict {
    if !outcome.is_ok() {
        return Verdict {
            severity: Severity::Warning,
            title: format!("{path} is not published (HTTP {})", served_status(outcome)),
            description: format!(
                "{path} {status}, so the check cannot pass: agents probing the path find \
                 nothing. Requirement: \"{requirement}\".",
                status = served_clause(outcome),
            ),
            recommendation: format!(
                "Publish the document at {path} with HTTP 200. Requirement: \"{requirement}\"."
            ),
        };
    }
    Verdict {
        severity: Severity::Warning,
        title: format!("{path} body is not valid JSON"),
        description: format!(
            "{path} answered 200 but the body is not parseable JSON, so the card cannot be \
             consumed. Requirement: \"{requirement}\"."
        ),
        recommendation: format!(
            "Serve well-formed JSON at {path}. Requirement: \"{requirement}\"."
        ),
    }
}

fn served_status(outcome: &ProbeOutcome) -> String {
    outcome
        .status
        .map(|s| s.to_string())
        .unwrap_or_else(|| "no response".to_string())
}

/// Whether `url` parses as an absolute http(s) URL (local copy so the module
/// stays self-contained).
fn is_absolute_http_url(url: &str) -> bool {
    match url::Url::parse(url) {
        Ok(parsed) => matches!(parsed.scheme(), "http" | "https"),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// The real prober (feature `full`): SSRF guard → rate limiter → fetch
// ---------------------------------------------------------------------------

/// Build the production fetch function: guard, throttle, then GET on the
/// dedicated blocking runtime. Redirects are followed (a well-known path may
/// legitimately redirect); the final response is what gets validated.
#[cfg(feature = "full")]
fn real_prober(url: &str) -> ProbeOutcome {
    use crate::ratelimit::RateLimiter;

    if !crate::ssrf::is_public_url(url) {
        return ProbeOutcome::unreachable();
    }
    let Ok(parsed) = url::Url::parse(url) else {
        return ProbeOutcome::unreachable();
    };
    let Some(domain) = parsed.host_str() else {
        return ProbeOutcome::unreachable();
    };

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION"),
            " agent-readiness-probe"
        ))
        .build();
    let Ok(client) = client else {
        return ProbeOutcome::unreachable();
    };

    // The existing limiter, at a deliberately polite rate: probes add
    // requests to a host the crawl may already be hammering.
    let limiter = RateLimiter::new(1.0, 2.0);

    crate::plugin::fetch_runtime().block_on(async move {
        if limiter.acquire(domain).await.is_err() {
            return ProbeOutcome::unreachable();
        }
        let Ok(response) = client.get(url).send().await else {
            return ProbeOutcome::unreachable();
        };
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let link_headers: Vec<String> = response
            .headers()
            .get_all(reqwest::header::LINK)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(str::to_string)
            .collect();
        let body = response.bytes().await.ok().map(|bytes| {
            String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BODY_BYTES)]).into_owned()
        });
        ProbeOutcome {
            status: Some(status),
            content_type: content_type.map(|ct| {
                ct.split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase()
            }),
            body,
            link_headers,
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn fixture_page() -> ParsedPage {
        ParsedPage {
            url: "https://site.example/".to_string(),
            meta: MetaTags::default(),
            headings: vec![],
            links: vec![],
            images: vec![],
            forms: vec![],
            scripts: vec![],
            styles: vec![],
            structured_data: vec![],
            word_count: 0,
            sentence_count: 0,
            landmarks: vec![],
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: true,
            html_lang: Some("en".to_string()),
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn ctx_with<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext::new(page)
    }

    /// A prober serving canned responses from a path→outcome table; every
    /// other path is a 404. Records the URLs it was asked for.
    fn canned_prober(
        responses: Vec<(&'static str, ProbeOutcome)>,
    ) -> (Prober, Arc<Mutex<Vec<String>>>) {
        let table: HashMap<String, ProbeOutcome> = responses
            .into_iter()
            .map(|(path, outcome)| (path.to_string(), outcome))
            .collect();
        let requested = Arc::new(Mutex::new(Vec::new()));
        let requested_for_prober = Arc::clone(&requested);
        let prober: Prober = Arc::new(move |url: &str| {
            requested_for_prober
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(url.to_string());
            let path = url::Url::parse(url)
                .ok()
                .map(|u| u.path().to_string())
                .unwrap_or_default();
            table
                .get(&path)
                .cloned()
                .unwrap_or_else(|| ProbeOutcome::reachable(404, Some("text/html"), None))
        });
        (prober, requested)
    }

    fn json_outcome(status: u16, content_type: &str, body: &str) -> ProbeOutcome {
        let mut outcome = ProbeOutcome::reachable(status, Some(content_type), Some(body));
        outcome.body = Some(body.to_string());
        outcome
    }

    fn link_outcome(status: u16, headers: &[&str]) -> ProbeOutcome {
        ProbeOutcome {
            status: Some(status),
            content_type: Some("text/html".to_string()),
            body: Some("<html></html>".to_string()),
            link_headers: headers.iter().map(|h| (*h).to_string()).collect(),
        }
    }

    fn one_finding<'a>(findings: &'a [Finding], code: &str) -> &'a Finding {
        let matches: Vec<&Finding> = findings.iter().filter(|f| f.code == code).collect();
        assert_eq!(
            matches.len(),
            1,
            "check {code} must emit exactly one finding, got {}: {:?}",
            matches.len(),
            findings.iter().map(|f| &f.code).collect::<Vec<_>>()
        );
        matches[0]
    }

    #[test]
    fn disabled_analyzer_emits_nothing_and_hits_nothing() {
        let page = fixture_page();
        let ctx = ctx_with(&page);
        let (prober, requested) = canned_prober(vec![]);
        let analyzer = AgentDiscoveryAnalyzer {
            enabled: false,
            prober: Some(prober),
            cache: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashSet::new()),
        };
        assert!(analyzer.analyze(&ctx).is_empty());
        assert!(requested
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty());
    }

    #[test]
    fn enabled_analyzer_probes_each_wellknown_path_once() {
        let page = fixture_page();
        let ctx = ctx_with(&page);
        let (prober, requested) = canned_prober(vec![]);
        let analyzer = AgentDiscoveryAnalyzer::with_probes_and_prober(prober);
        let findings = analyzer.analyze(&ctx);
        assert_eq!(findings.len(), 5, "one finding per P1 check");

        let requested = requested
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for path in PROBE_PATHS {
            let expected = format!("https://site.example{path}");
            assert!(
                requested.iter().any(|u| u == &expected),
                "probe of {expected} missing from {requested:?}"
            );
        }
        assert_eq!(
            requested.len(),
            PROBE_PATHS.len(),
            "exactly one probe per path"
        );
    }

    #[test]
    fn second_page_of_same_origin_reuses_cached_probes() {
        let page = fixture_page();
        let ctx = ctx_with(&page);
        let (prober, requested) = canned_prober(vec![]);
        let analyzer = AgentDiscoveryAnalyzer::with_probes_and_prober(prober);
        let first = analyzer.analyze(&ctx);
        let second = analyzer.analyze(&ctx);
        assert_eq!(first.len(), second.len());
        let requested = requested
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(
            requested.len(),
            PROBE_PATHS.len(),
            "cache must prevent re-probing"
        );
    }

    #[test]
    fn all_checks_pass_on_valid_responses() {
        let page = fixture_page();
        let ctx = ctx_with(&page);
        let (prober, _) = canned_prober(vec![
            (
                "/.well-known/api-catalog",
                json_outcome(
                    200,
                    "application/linkset+json",
                    r#"{"linkset":[{"anchor":"https://site.example/","service-desc":{"href":"https://site.example/openapi.json"},"service-doc":{"href":"https://site.example/docs"},"status":"dev"}]}"#,
                ),
            ),
            (
                "/",
                link_outcome(
                    200,
                    &[
                        r#"<https://site.example/.well-known/api-catalog>; rel="api-catalog""#,
                        r#"<https://site.example/openapi.json>; rel="service-desc", <https://site.example/docs>; rel="service-doc""#,
                    ],
                ),
            ),
            (
                "/.well-known/agent-card.json",
                json_outcome(
                    200,
                    "application/json",
                    r#"{"name":"site-agent","version":"1.0.0","description":"does things","supportedInterfaces":[{"url":"https://site.example/a2a","transport":"JSONRPC"}],"capabilities":{"streaming":true},"skills":[{"id":"search","name":"search","description":"site search"}]}"#,
                ),
            ),
            (
                "/.well-known/agent-skills/index.json",
                json_outcome(
                    200,
                    "application/json",
                    r#"{"$schema":"https://schemas.agentskills.io/discovery/0.2.0/schema.json","skills":[{"name":"doc-search","type":"skill-md","description":"search the docs","url":"https://site.example/skills/doc-search.md","digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}]}"#,
                ),
            ),
            (
                "/.well-known/mcp/server-card.json",
                json_outcome(
                    200,
                    "application/json",
                    r#"{"serverInfo":{"name":"site-mcp","version":"0.1.0"},"endpoint":"https://site.example/mcp","capabilities":{"tools":{}}}"#,
                ),
            ),
        ]);
        let analyzer = AgentDiscoveryAnalyzer::with_probes_and_prober(prober);
        let findings = analyzer.analyze(&ctx);
        assert_eq!(findings.len(), 5);
        for f in &findings {
            assert_eq!(f.severity, Severity::Info, "{} should pass", f.code);
        }
        assert_eq!(
            one_finding(&findings, "AIREADY-APICATALOG").code,
            "AIREADY-APICATALOG"
        );
    }

    #[test]
    fn unprobeable_paths_report_specific_warnings() {
        let page = fixture_page();
        let ctx = ctx_with(&page);
        // Nothing served: every path 404s except the root.
        let (prober, _) = canned_prober(vec![("/", link_outcome(200, &[]))]);
        let analyzer = AgentDiscoveryAnalyzer::with_probes_and_prober(prober);
        let findings = analyzer.analyze(&ctx);
        assert_eq!(findings.len(), 5);
        for code in [
            "AIREADY-APICATALOG",
            "AIREADY-A2A",
            "AIREADY-AGSKILLS",
            "AIREADY-MCPCARD",
        ] {
            let f = one_finding(&findings, code);
            assert_eq!(f.severity, Severity::Warning, "{code}");
            assert!(
                f.title.contains("404") || f.title.contains("not published"),
                "{code} must name the status: {}",
                f.title
            );
        }
        // Check 6 saw a 200 with no relevant rels.
        let linkhdr = one_finding(&findings, "AIREADY-LINKHDR");
        assert_eq!(linkhdr.severity, Severity::Warning);
        assert!(linkhdr.title.contains("Link"));
    }

    #[test]
    fn api_catalog_wrong_content_type_warns() {
        let outcome = json_outcome(
            200,
            "application/json",
            r#"{"linkset":[{"anchor":"https://x.example/","service-doc":{"href":"https://x.example/d"}}]}"#,
        );
        let v = api_catalog_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
        assert!(v.description.contains("application/linkset+json"));
    }

    #[test]
    fn api_catalog_missing_relations_warn() {
        let outcome = json_outcome(
            200,
            "application/linkset+json",
            r#"{"linkset":[{"anchor":"https://x.example/"},{"nope":true}]}"#,
        );
        let v = api_catalog_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
        assert!(v.description.contains("service-desc"), "{}", v.description);
    }

    #[test]
    fn api_catalog_linkset_not_array_warns() {
        let outcome = json_outcome(200, "application/linkset+json", r#"{"linkset": 5}"#);
        assert_eq!(api_catalog_verdict(&outcome).severity, Severity::Warning);
        let outcome = json_outcome(200, "application/linkset+json", "not json at all");
        assert_eq!(api_catalog_verdict(&outcome).severity, Severity::Warning);
    }

    #[test]
    fn link_headers_accept_comma_separated_and_multiple_headers() {
        let outcome = link_outcome(
            200,
            &[
                r#"<https://x.example/.well-known/api-catalog>; rel="api-catalog""#,
                r#"<https://x.example/openapi.json>; rel="service-desc", <https://x.example/docs>; rel='service-doc', <https://x.example/describedby>; rel="describedby""#,
            ],
        );
        let v = link_header_verdict(&outcome);
        assert_eq!(v.severity, Severity::Info);
        assert!(v.description.contains("describedby"));
    }

    #[test]
    fn link_header_irrelevant_rels_warn() {
        let outcome = link_outcome(200, &[r#"<https://x.example/a>; rel="stylesheet""#]);
        assert_eq!(link_header_verdict(&outcome).severity, Severity::Warning);
    }

    #[test]
    fn a2a_card_missing_skills_fields_warn() {
        let outcome = json_outcome(
            200,
            "application/json",
            r#"{"name":"a","version":"1","description":"d","supportedInterfaces":[{"url":"https://x/a2a","transport":"JSONRPC"}],"capabilities":{},"skills":[{"id":"s","name":"s"}]}"#,
        );
        let v = a2a_card_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
        assert!(
            v.description.contains("skills[0]: missing `description`"),
            "{}",
            v.description
        );
    }

    #[test]
    fn a2a_card_empty_supported_interfaces_warns() {
        let outcome = json_outcome(
            200,
            "application/json",
            r#"{"name":"a","version":"1","description":"d","supportedInterfaces":[],"capabilities":{},"skills":[{"id":"s","name":"s","description":"d"}]}"#,
        );
        let v = a2a_card_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
    }

    #[test]
    fn agent_skills_index_requires_pinned_schema() {
        let body = r#"{"$schema":"https://schemas.agentskills.io/discovery/0.1.0/schema.json","skills":[{"name":"doc-search","type":"skill-md","description":"d","url":"https://x/s.md","digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}]}"#;
        let outcome = json_outcome(200, "application/json", body);
        let v = agent_skills_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
        assert!(v.description.contains("$schema"), "{}", v.description);
    }

    #[test]
    fn agent_skills_entry_shape_is_enforced() {
        // Uppercase name, wrong type, bad digest: three distinct problems.
        let body = r#"{"$schema":"https://schemas.agentskills.io/discovery/0.2.0/schema.json","skills":[{"name":"Doc Search","type":"zip","description":"d","url":"https://x/s.md","digest":"md5:abc"}]}"#;
        let outcome = json_outcome(200, "application/json", body);
        let v = agent_skills_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
        let description = &v.description;
        assert!(description.contains("lowercase+hyphens"), "{description}");
        assert!(description.contains("skill-md|archive"), "{description}");
        assert!(description.contains("sha256"), "{description}");
    }

    #[test]
    fn agent_skills_name_validation_matches_spec_examples() {
        assert!(is_skill_name("doc-search"));
        assert!(is_skill_name("a"));
        assert!(is_skill_name("skill-2-fast"));
        assert!(!is_skill_name("Doc-Search"));
        assert!(!is_skill_name("-leading"));
        assert!(!is_skill_name("trailing-"));
        assert!(!is_skill_name("double--hyphen"));
        assert!(!is_skill_name("under_score"));
        assert!(!is_skill_name(""));
    }

    #[test]
    fn sha256_digest_validation() {
        assert!(is_sha256_digest(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!is_sha256_digest("sha256:abc"));
        assert!(!is_sha256_digest(
            "md5:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!is_sha256_digest("sha256:xyz"));
    }

    #[test]
    fn mcp_card_requires_serverinfo_endpoint_and_capabilities() {
        let body = r#"{"serverInfo":{"name":"m"},"endpoint":"ftp://x/mcp","capabilities":{}}"#;
        let outcome = json_outcome(200, "application/json", body);
        let v = mcp_card_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
        let description = &v.description;
        assert!(description.contains("version"), "{description}");
        assert!(description.contains("http(s)"), "{description}");
        assert!(
            description.contains("tools/resources/prompts"),
            "{description}"
        );
    }

    #[test]
    fn mcp_card_passes_with_valid_shape() {
        let body = r#"{"serverInfo":{"name":"m","version":"1"},"endpoint":"https://x/mcp","capabilities":{"tools":{},"resources":{},"prompts":{}}}"#;
        let outcome = json_outcome(200, "application/json", body);
        assert_eq!(mcp_card_verdict(&outcome).severity, Severity::Info);
    }

    #[test]
    fn unreachable_probe_reports_warning_quoting_requirement() {
        let outcome = ProbeOutcome::unreachable();
        let v = api_catalog_verdict(&outcome);
        assert_eq!(v.severity, Severity::Warning);
        assert!(
            v.title.contains("could not complete"),
            "the title must say the probe did not complete: {}",
            v.title
        );
        assert!(
            v.description.contains("no HTTP response"),
            "the description must say why: {}",
            v.description
        );
        assert!(v.recommendation.contains("linkset"));
    }

    #[test]
    fn non_http_origin_is_not_probed() {
        assert_eq!(origin_of("file:///etc/passwd"), None);
        assert_eq!(origin_of("ftp://x.example/"), None);
        assert_eq!(
            origin_of("https://site.example:8443/a"),
            Some("https://site.example:8443".to_string())
        );
        assert_eq!(
            origin_of("http://site.example/a"),
            Some("http://site.example".to_string())
        );
    }

    #[test]
    fn findings_are_canonical_per_check_even_on_partial_probes() {
        // A prober that fails everything still yields exactly five findings,
        // one per check, all Warnings — the per-page contract holds.
        let page = fixture_page();
        let ctx = ctx_with(&page);
        let prober: Prober = Arc::new(|_| ProbeOutcome::unreachable());
        let analyzer = AgentDiscoveryAnalyzer::with_probes_and_prober(prober);
        let findings = analyzer.analyze(&ctx);
        assert_eq!(findings.len(), 5);
        for f in &findings {
            assert_eq!(f.severity, Severity::Warning, "{}", f.code);
        }
    }
}
