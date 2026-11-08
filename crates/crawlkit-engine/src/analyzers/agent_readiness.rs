//! Agent-readiness audit (docs/AGENT_READINESS_BACKLOG.md checks 1-6, 10-12).
//!
//! # Why this exists
//!
//! Agentic consumers (AI crawlers, agent frameworks, MCP clients) discover a
//! site through a small set of convention files: `robots.txt` directives,
//! `sitemap.xml`, and the `/.well-known/` document family (api-catalog,
//! agent-card, agent-skills index, MCP server card). crawlkit's SEO analyzers
//! judge discoverability for search engines; nothing judged the agent
//! discovery surface. The backlog catalog (docs/AGENT_READINESS_BACKLOG.md)
//! enumerates 21 checks across three tiers; this analyzer implements the P0
//! crawl-basics tier and the P1 discovery-surface tier. P2/P3 checks are
//! mapped, not implemented, in [`super::agent_readiness_planned`].
//!
//! # What is reported
//!
//! One Warning/Info finding per failing check, code `AIREADY-<CHECK>`, with an
//! actionable recommendation. Silence means the check passed or did not apply:
//!
//! | Code                 | Check (backlog #)                       | Data source              |
//! |----------------------|------------------------------------------|--------------------------|
//! | `AIREADY-ROBOTS`     | robots.txt served + has directives (1)   | `ctx.robots_txt`         |
//! | `AIREADY-SITEMAP`    | sitemap referenced from robots.txt (2)   | `ctx.robots_txt`         |
//! | `AIREADY-SIGNAL`     | Content-Signal directives (3)            | `ctx.robots_txt`         |
//! | `AIREADY-AICRAWL`    | explicit AI-crawler User-agent rules (4) | `ctx.robots_txt`         |
//! | `AIREADY-LINKHDR`    | discovery `Link:` headers on `/` (6)     | homepage response headers|
//! | `AIREADY-APICATALOG` | /.well-known/api-catalog shape (5)       | the fetched document     |
//! | `AIREADY-A2A`        | /.well-known/agent-card.json shape (10)  | the fetched document     |
//! | `AIREADY-AGSKILLS`   | agent-skills index.json shape (11)       | the fetched document     |
//! | `AIREADY-MCPCARD`    | /.well-known/mcp/server-card.json (12)   | the fetched document     |
//!
//! # What is deliberately NOT done
//!
//! - **No network requests.** Every check matches data the crawl already
//!   stored, the same G7 acceptance invariant `tech_detection` holds. The
//!   crawler does not synthesize fetches for `/.well-known/` paths, so a
//!   well-known document is validated when the crawl encounters it (queued by
//!   sitemap discovery, linked from a page, or fetched by an embedder) and
//!   stays silent otherwise — absence of a fetch is not evidence that the
//!   document is missing, and reporting it as such would be fabricated advice.
//! - **No policy judgement on AI-crawler rules.** The check verifies that
//!   explicit `User-agent` groups exist for AI crawlers; whether a given
//!   Disallow is the right policy is the operator's decision, not the
//!   analyzer's.
//! - **No XML schema validation of sitemaps.** `SitemapAnalyzer` and its V2/V3
//!   ladders already own sitemap content checks; this check owns only the
//!   robots.txt reference.

use url::Url;

use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// Finding category shared by every agent-readiness check.
fn category() -> IssueCategory {
    IssueCategory::Custom("agent-readiness".to_string())
}

/// AI-crawler User-agent tokens (backlog check 4). Matched case-insensitively
/// against the exact `User-agent:` token.
const AI_CRAWLER_TOKENS: &[&str] = &[
    "gptbot",
    "oai-searchbot",
    "claudebot",
    "claude-web",
    "google-extended",
    "ccbot",
    "anthropic-ai",
    "bytespider",
    "cohere-ai",
    "applebot-extended",
    "amazonbot",
    "meta-externalagent",
    "diffbot",
];

/// The shape of a parsed robots.txt, as far as the readiness checks care.
#[derive(Debug, Default, PartialEq, Eq)]
struct RobotsSummary {
    /// At least one `User-agent:` line was present.
    has_user_agent: bool,
    /// At least one group carried an `Allow:`/`Disallow:` directive.
    has_crawl_directive: bool,
    /// Absolute HTTP(S) URLs from `Sitemap:` lines.
    sitemaps: Vec<String>,
    /// Values from `Content-Signal:` lines (contentsignals.org).
    content_signals: Vec<String>,
    /// At least one AI-crawler group carried an `Allow:`/`Disallow:` rule.
    has_ai_crawler_rules: bool,
}

/// Parse the parts of robots.txt the readiness checks read.
///
/// Group semantics follow RFC 9309 §2.1: consecutive `User-agent:` lines
/// belong to one group, and the group's rules start at the first
/// `Allow:`/`Disallow:` line. `Sitemap:` and `Content-Signal:` lines are
/// file-global. Malformed lines (no colon, unknown key) are skipped rather
/// than reported — `RobotsTxtDirectivesAnalyzer` owns directive-syntax
/// findings.
fn summarize_robots(txt: &str) -> RobotsSummary {
    let mut summary = RobotsSummary::default();
    // User-agent tokens of the group currently being read, and whether any of
    // them names an AI crawler.
    let mut current_uas: Vec<String> = Vec::new();
    let mut current_is_ai = false;
    let mut group_has_rule = false;

    for raw in txt.lines() {
        let line = raw.split('#').next().unwrap_or(raw).trim();
        if line.is_empty() {
            // A blank line separates groups (RFC 9309); start a fresh one so a
            // later `User-agent:` does not inherit the previous group's rules.
            if group_has_rule {
                current_uas.clear();
                current_is_ai = false;
                group_has_rule = false;
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().to_string();
        match key.as_str() {
            "user-agent" => {
                if group_has_rule {
                    // A new User-agent after rules begins a new group.
                    current_uas.clear();
                    current_is_ai = false;
                    group_has_rule = false;
                }
                summary.has_user_agent = true;
                let token = value.to_ascii_lowercase();
                current_is_ai |= AI_CRAWLER_TOKENS.contains(&token.as_str());
                current_uas.push(token);
            }
            "allow" | "disallow" => {
                if !current_uas.is_empty() {
                    group_has_rule = true;
                    summary.has_crawl_directive = true;
                    if current_is_ai {
                        summary.has_ai_crawler_rules = true;
                    }
                }
            }
            "sitemap" => {
                // Collect every value; the check-2 branch classifies absolute
                // http(s) URLs as references and anything else as invalid.
                summary.sitemaps.push(value);
            }
            "content-signal" => {
                summary.content_signals.push(value);
            }
            _ => {}
        }
    }
    summary
}

/// Which well-known document a URL path identifies, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WellKnownKind {
    ApiCatalog,
    AgentCard,
    AgentSkillsIndex,
    McpServerCard,
}

/// Match the URL's path against the four P1 well-known paths.
///
/// Path comparison is exact and case-sensitive: `/.well-known/` is a
/// registry-controlled namespace (RFC 8615 §3) and agents fetch these exact
/// paths.
fn well_known_kind(url: &str) -> Option<WellKnownKind> {
    let path = Url::parse(url).ok()?.path().to_string();
    match path.as_str() {
        "/.well-known/api-catalog" => Some(WellKnownKind::ApiCatalog),
        "/.well-known/agent-card.json" => Some(WellKnownKind::AgentCard),
        "/.well-known/agent-skills/index.json" => Some(WellKnownKind::AgentSkillsIndex),
        "/.well-known/mcp/server-card.json" => Some(WellKnownKind::McpServerCard),
        _ => None,
    }
}

/// Whether the URL is the site homepage (path `/` or empty).
fn is_homepage(url: &str) -> bool {
    Url::parse(url)
        .map(|u| matches!(u.path(), "/" | ""))
        .unwrap_or(false)
}

/// Every `rel` value across all `Link:` headers, lowercased.
///
/// RFC 8288 §3 allows multiple `Link:` headers and comma-separated
/// link-values in one header; both are collected. A link-value's parameters
/// are semicolon-separated after the URI (`</desc>; rel="service-desc"`), so
/// the rel parameter is found by scanning the parameters of each split.
fn link_rel_values(headers: &[(String, String)]) -> Vec<String> {
    let mut rels = Vec::new();
    for (name, value) in headers {
        if !name.eq_ignore_ascii_case("link") {
            continue;
        }
        for link_value in value.split(',') {
            for param in link_value.split(';') {
                let param = param.trim();
                let Some(rest) = param
                    .strip_prefix("rel=")
                    .or_else(|| param.strip_prefix("Rel="))
                else {
                    continue;
                };
                let cleaned = rest.trim_matches('"').trim_matches('\'');
                for rel in cleaned.split_whitespace() {
                    rels.push(rel.to_ascii_lowercase());
                }
            }
        }
    }
    rels
}

/// Whether a response status code counts as "served".
fn served(status_code: Option<u16>) -> bool {
    status_code.is_some_and(|s| s == 200)
}

/// Media type (before `;` parameters), lowercased.
fn media_type(content_type: Option<&str>) -> String {
    content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn finding(
    code: &str,
    title: &str,
    description: impl Into<String>,
    recommendation: impl Into<String>,
    url: &str,
    severity: Severity,
) -> Finding {
    Finding {
        severity,
        category: category(),
        code: code.to_string(),
        title: title.to_string(),
        description: description.into(),
        url: url.to_string(),
        recommendation: recommendation.into(),
    }
}

/// Passive agent-readiness audit over data the crawl already stored
/// (backlog checks 1-6 and 10-12).
///
/// # Ownership contract with the live prober
///
/// `agent_discovery_analyzers::AgentDiscoveryAnalyzer` implements the same
/// five discovery checks (5, 6, 10-12) from explicit rate-limited GETs, gated
/// behind `CRAWLKIT_AGENT_DISCOVERY=1`. `collapse_duplicates` deliberately
/// never merges two same-code findings, so both analyzers emitting
/// `AIREADY-LINKHDR` on one page would ship duplicates. The split: when the
/// prober is enabled it owns the discovery checks and this analyzer's
/// overlapping checks stand down; in the default configuration the prober is
/// silent and these passive checks are the only coverage a crawl-encountered
/// discovery document gets. The robots-family checks (1-4) have no prober
/// counterpart and always run.
pub struct AgentReadinessAnalyzer {
    /// Whether the opt-in live prober owns checks 5, 6 and 10-12.
    discovery_owned_by_prober: bool,
}

/// Whether the opt-in agent-discovery prober is enabled, read once.
///
/// `agent_discovery_analyzers` reads the same variable per construction; a
/// shared `OnceLock` keeps both sides on one answer for the process lifetime
/// (analyzer registries are built once per crawl).
fn prober_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("CRAWLKIT_AGENT_DISCOVERY").is_ok_and(|v| v == "1"))
}

impl Default for AgentReadinessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentReadinessAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            discovery_owned_by_prober: prober_enabled(),
        }
    }

    /// A passive-only analyzer that never stands down, for tests and for
    /// embedders that want the crawl-encountered-document checks regardless
    /// of the prober environment.
    #[must_use]
    pub fn passive_only() -> Self {
        Self {
            discovery_owned_by_prober: false,
        }
    }

    /// Checks 1-4 (robots.txt family) plus check 6 (homepage Link headers):
    /// the site-level discovery surface every crawled page carries.
    fn analyze_site_level(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let url = ctx.page.url.clone();
        let mut findings = Vec::new();
        let robots = ctx.robots_txt.map(summarize_robots);

        // --- Check 1: robots.txt (RFC 9309) --------------------------------
        match robots {
            None => {
                findings.push(finding(
                    "AIREADY-ROBOTS",
                    "robots.txt is not served with HTTP 200",
                    "No robots.txt was served for this host: the fetch returned a \
                     non-200 status or failed. Agentic crawlers treat an absent \
                     robots.txt as unlimited access and cannot discover the site's \
                     Sitemap or Content-Signal directives.",
                    "Publish /robots.txt as text/plain with HTTP 200, containing at \
                     least one User-agent group with Allow/Disallow directives.",
                    &url,
                    Severity::Warning,
                ));
            }
            Some(ref summary) => {
                if !summary.has_user_agent || !summary.has_crawl_directive {
                    findings.push(finding(
                        "AIREADY-ROBOTS",
                        "robots.txt has no usable crawl directives",
                        "robots.txt is served but contains no User-agent group with \
                         Allow/Disallow directives, so it grants no crawl rules to \
                         any agent.",
                        "Add at least one User-agent group with Allow/Disallow \
                         directives to /robots.txt (RFC 9309).",
                        &url,
                        Severity::Warning,
                    ));
                }

                // --- Check 2: sitemap referenced from robots.txt ------------
                if summary.sitemaps.is_empty() {
                    findings.push(finding(
                        "AIREADY-SITEMAP",
                        "robots.txt does not reference a sitemap",
                        "No `Sitemap:` directive was found in robots.txt, so agents \
                         and search engines must guess the sitemap location.",
                        "Add a `Sitemap: <absolute-URL>` line to /robots.txt \
                         pointing at the sitemap index.",
                        &url,
                        Severity::Warning,
                    ));
                } else {
                    // Referenced URLs are validated above (only absolute
                    // http(s) URLs are collected); report malformed references.
                    let invalid = ctx
                        .robots_txt
                        .unwrap_or("")
                        .lines()
                        .filter_map(|l| {
                            let l = l.split('#').next().unwrap_or(l).trim();
                            let (k, v) = l.split_once(':')?;
                            k.trim()
                                .eq_ignore_ascii_case("sitemap")
                                .then(|| v.trim().to_string())
                        })
                        .filter(|v| {
                            Url::parse(v)
                                .map(|u| !matches!(u.scheme(), "http" | "https"))
                                .unwrap_or(true)
                        })
                        .collect::<Vec<_>>();
                    if !invalid.is_empty() {
                        findings.push(finding(
                            "AIREADY-SITEMAP",
                            "robots.txt references an invalid sitemap URL",
                            format!(
                                "The `Sitemap:` value(s) {} are not absolute \
                                 http(s) URLs, so crawlers will ignore them.",
                                invalid
                                    .iter()
                                    .map(|v| format!("`{v}`"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            "Point every `Sitemap:` directive at an absolute \
                             http(s) URL.",
                            &url,
                            Severity::Warning,
                        ));
                    }
                }

                // --- Check 3: Content-Signal directives ---------------------
                if summary.content_signals.is_empty() {
                    findings.push(finding(
                        "AIREADY-SIGNAL",
                        "robots.txt declares no Content-Signal directives",
                        "No `Content-Signal:` directive was found (contentsignals.org). \
                         Without one there is no machine-readable statement of how \
                         content may be used (ai-train, ai-input, search).",
                        "Publish your usage policy with Content-Signal directives, \
                         e.g. `Content-Signal: ai-train=no, search=yes` in \
                         /robots.txt.",
                        &url,
                        Severity::Info,
                    ));
                }

                // --- Check 4: explicit AI-crawler rules ---------------------
                if !summary.has_ai_crawler_rules {
                    findings.push(finding(
                        "AIREADY-AICRAWL",
                        "robots.txt has no explicit AI-crawler rules",
                        "No User-agent group for a known AI crawler (GPTBot, \
                         OAI-SearchBot, ClaudeBot/Claude-Web, Google-Extended, CCBot, \
                         anthropic-ai, Bytespider, Cohere-AI, Applebot-Extended, \
                         Amazonbot, Meta-ExternalAgent, Diffbot) carries \
                         Allow/Disallow rules, so AI crawlers fall through to the \
                         `*` group and your AI access policy is implicit.",
                        "Add explicit User-agent groups for the AI crawlers you want \
                         to govern, each with Allow/Disallow rules matching your \
                         policy.",
                        &url,
                        Severity::Warning,
                    ));
                }
            }
        }

        // --- Check 6: discovery Link headers on the homepage (RFC 8288).
        // Stands down when the opt-in prober owns this check (ownership
        // contract in the struct docs).
        if !self.discovery_owned_by_prober && is_homepage(&url) && served(ctx.status_code) {
            const DISCOVERY_RELS: &[&str] =
                &["api-catalog", "service-desc", "service-doc", "describedby"];
            let rels = link_rel_values(ctx.headers);
            if !DISCOVERY_RELS
                .iter()
                .any(|want| rels.contains(&(*want).to_string()))
            {
                findings.push(finding(
                    "AIREADY-LINKHDR",
                    "homepage exposes no agent-discovery Link headers",
                    "The homepage response carries no `Link:` header with rel \
                     api-catalog, service-desc, service-doc, or describedby, so \
                     agents cannot discover API documentation from headers alone.",
                    "Return `Link: </.well-known/api-catalog>; rel=\"api-catalog\"` \
                     (RFC 8288) and service-desc/service-doc links from the \
                     homepage response.",
                    &url,
                    Severity::Info,
                ));
            }
        }

        findings
    }

    /// Checks 5, 10, 11, 12: validate a well-known discovery document when the
    /// crawl actually fetched that path. Silent on every other resource.
    ///
    /// Runs on non-HTML bodies too — the JSON cards are exactly the resources
    /// the HTML document gate excludes, so the pipeline calls this from its
    /// non-HTML branch in addition to [`non_html_finding`]. Stands down when
    /// the opt-in prober owns the discovery checks (struct docs).
    ///
    /// [`non_html_finding`]: super::non_html_finding
    #[must_use]
    pub fn analyze_resource(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        if self.discovery_owned_by_prober {
            return Vec::new();
        }
        let url = ctx.page.url.clone();
        let Some(kind) = well_known_kind(&url) else {
            return Vec::new();
        };
        let mut findings = Vec::new();
        let status_ok = served(ctx.status_code);

        // Status first: a non-200 well-known path fails its check outright.
        if !status_ok {
            let (code, name) = match kind {
                WellKnownKind::ApiCatalog => ("AIREADY-APICATALOG", "/.well-known/api-catalog"),
                WellKnownKind::AgentCard => ("AIREADY-A2A", "/.well-known/agent-card.json"),
                WellKnownKind::AgentSkillsIndex => {
                    ("AIREADY-AGSKILLS", "/.well-known/agent-skills/index.json")
                }
                WellKnownKind::McpServerCard => {
                    ("AIREADY-MCPCARD", "/.well-known/mcp/server-card.json")
                }
            };
            findings.push(finding(
                code,
                "agent discovery document is not served with HTTP 200",
                format!(
                    "{name} responded with status {} (a successful fetch requires \
                     HTTP 200 with the document body).",
                    ctx.status_code
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ),
                "Serve the discovery document at the exact well-known path with \
                 HTTP 200 and its specified media type.",
                &url,
                Severity::Warning,
            ));
            return findings;
        }

        let Some(body) = ctx.body else {
            return findings;
        };
        let Ok(doc) = serde_json::from_str::<serde_json::Value>(body) else {
            let code = match kind {
                WellKnownKind::ApiCatalog => "AIREADY-APICATALOG",
                WellKnownKind::AgentCard => "AIREADY-A2A",
                WellKnownKind::AgentSkillsIndex => "AIREADY-AGSKILLS",
                WellKnownKind::McpServerCard => "AIREADY-MCPCARD",
            };
            findings.push(finding(
                code,
                "agent discovery document is not valid JSON",
                "The well-known document was served with HTTP 200 but its body is \
                 not a valid JSON document, so agent clients cannot parse it.",
                "Return well-formed JSON (correct media type, no trailing HTML \
                 error pages) at the well-known path.",
                &url,
                Severity::Warning,
            ));
            return findings;
        };

        match kind {
            WellKnownKind::ApiCatalog => self.check_api_catalog(ctx, &doc, &url, &mut findings),
            WellKnownKind::AgentCard => self.check_agent_card(ctx, &doc, &url, &mut findings),
            WellKnownKind::AgentSkillsIndex => {
                self.check_agent_skills(ctx, &doc, &url, &mut findings);
            }
            WellKnownKind::McpServerCard => {
                self.check_mcp_card(ctx, &doc, &url, &mut findings);
            }
        }
        findings
    }

    /// Check 5 (RFC 9727): `linkset` array with `anchor` and
    /// service-desc/service-doc relations.
    fn check_api_catalog(
        &self,
        ctx: &AnalysisContext,
        doc: &serde_json::Value,
        url: &str,
        findings: &mut Vec<Finding>,
    ) {
        let media = media_type(ctx.content_type);
        if !media.is_empty() && media != "application/linkset+json" && media != "application/json" {
            findings.push(finding(
                "AIREADY-APICATALOG",
                "api-catalog served with unexpected media type",
                format!(
                    "Content-Type is `{media}`; RFC 9727 specifies \
                     application/linkset+json."
                ),
                "Serve /.well-known/api-catalog as application/linkset+json.",
                url,
                Severity::Info,
            ));
        }

        let Some(linkset) = doc.get("linkset").filter(|v| v.is_array()) else {
            findings.push(finding(
                "AIREADY-APICATALOG",
                "api-catalog has no linkset array",
                "The document parses as JSON but has no top-level `linkset` \
                 array, so no API descriptions are discoverable from it.",
                "Structure the catalog as {\"linkset\": [...]} per RFC 9727, one \
                 entry per service.",
                url,
                Severity::Warning,
            ));
            return;
        };

        let Some(arr) = linkset.as_array() else {
            return;
        };
        let mut problems: Vec<String> = Vec::new();
        for (i, entry) in arr.iter().enumerate() {
            if entry.get("anchor").and_then(|v| v.as_str()).is_none() {
                problems.push(format!("linkset[{i}] is missing `anchor`"));
            }
            let has_service_rel = ["service-desc", "service-doc"].iter().any(|rel| {
                entry
                    .get(rel)
                    .is_some_and(|v| v.is_string() || v.is_array() || v.is_object())
            });
            if !has_service_rel {
                problems.push(format!(
                    "linkset[{i}] has no service-desc or service-doc relation"
                ));
            }
        }
        if !problems.is_empty() {
            findings.push(finding(
                "AIREADY-APICATALOG",
                "api-catalog linkset entries are incomplete",
                format!(
                    "The linkset array has {} entr{} with problems: {}.",
                    problems.len(),
                    if problems.len() == 1 { "y" } else { "ies" },
                    problems.join("; ")
                ),
                "Give every linkset entry an `anchor` and at least one \
                 service-desc or service-doc relation pointing at its \
                 description document.",
                url,
                Severity::Warning,
            ));
        }
    }

    /// Check 10: A2A Agent Card — name, version, description, capabilities,
    /// skills, and interface information.
    fn check_agent_card(
        &self,
        _ctx: &AnalysisContext,
        doc: &serde_json::Value,
        url: &str,
        findings: &mut Vec<Finding>,
    ) {
        let mut missing: Vec<&str> = Vec::new();
        for field in ["name", "version", "description"] {
            if doc
                .get(field)
                .and_then(|v| v.as_str())
                .is_none_or(str::is_empty)
            {
                missing.push(field);
            }
        }
        if doc.get("capabilities").is_none_or(|v| !v.is_object()) {
            missing.push("capabilities");
        }
        let skills = doc.get("skills");
        if skills.is_none_or(|v| !v.is_array()) {
            missing.push("skills");
        }

        // Interface information: the card names the service endpoint either in
        // `supportedInterfaces` (array of {url, transport}) or the older
        // flat `url`/`serviceUrl` + transport fields.
        let interfaces = doc
            .get("supportedInterfaces")
            .or_else(|| doc.get("interfaces"));
        let has_interface = interfaces.and_then(|v| v.as_array()).is_some_and(|arr| {
            arr.iter().any(|i| {
                i.get("url")
                    .and_then(|v| v.as_str())
                    .is_some_and(|u| !u.is_empty())
            })
        }) || doc
            .get("url")
            .or_else(|| doc.get("serviceUrl"))
            .and_then(|v| v.as_str())
            .is_some_and(|u| !u.is_empty());
        if !has_interface {
            missing.push("supportedInterfaces (or url/serviceUrl)");
        }

        if !missing.is_empty() {
            findings.push(finding(
                "AIREADY-A2A",
                "agent-card.json is missing required fields",
                format!(
                    "The A2A Agent Card is served but lacks: {}. Agents relying \
                     on the card cannot describe or invoke the service.",
                    missing.join(", ")
                ),
                "Complete the Agent Card: name, version, description, \
                 capabilities, skills[] (id/name/description), and \
                 supportedInterfaces with a service URL and transport.",
                url,
                Severity::Warning,
            ));
            return;
        }

        // Skills must identify what they expose.
        let Some(skill_arr) = skills.and_then(|v| v.as_array()) else {
            return; // already reported as missing above
        };
        let bad_skills = skill_arr
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let ok = ["id", "name", "description"].iter().all(|f| {
                    s.get(*f)
                        .and_then(|v| v.as_str())
                        .is_some_and(|v| !v.is_empty())
                });
                (!ok).then(|| i.to_string())
            })
            .collect::<Vec<_>>();
        if !bad_skills.is_empty() {
            findings.push(finding(
                "AIREADY-A2A",
                "agent-card.json skills are missing id/name/description",
                format!(
                    "skills[{}] lack id, name, or description.",
                    bad_skills.join(", ")
                ),
                "Give every entry in skills[] an id, a name, and a description.",
                url,
                Severity::Warning,
            ));
        }
    }

    /// Check 11: Agent Skills discovery index — $schema pin and per-skill
    /// name/type/description/url/digest.
    fn check_agent_skills(
        &self,
        _ctx: &AnalysisContext,
        doc: &serde_json::Value,
        url: &str,
        findings: &mut Vec<Finding>,
    ) {
        const SCHEMA: &str = "https://schemas.agentskills.io/discovery/0.2.0/schema.json";
        if doc.get("$schema").and_then(|v| v.as_str()) != Some(SCHEMA) {
            findings.push(finding(
                "AIREADY-AGSKILLS",
                "agent-skills index.json pins no discovery schema",
                format!(
                    "`$schema` is not {SCHEMA}; consumers cannot validate the \
                     index against a known discovery version."
                ),
                format!("Set `\"$schema\": \"{SCHEMA}\"` at the top of the index."),
                url,
                Severity::Warning,
            ));
        }

        let Some(skills) = doc.get("skills").filter(|v| v.is_array()) else {
            findings.push(finding(
                "AIREADY-AGSKILLS",
                "agent-skills index.json has no skills array",
                "The discovery index contains no `skills` array, so no skills \
                 are discoverable from it.",
                "List each skill in `skills[]` with name, type, description, \
                 url, and digest.",
                url,
                Severity::Warning,
            ));
            return;
        };

        let Some(skill_arr) = skills.as_array() else {
            return; // already reported as missing above
        };
        let mut problems: Vec<String> = Vec::new();
        for (i, skill) in skill_arr.iter().enumerate() {
            let name = skill.get("name").and_then(|v| v.as_str());
            if name.is_none_or(|n| {
                n.is_empty() || !n.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            }) {
                problems.push(format!(
                    "skills[{i}].name must be lowercase letters and hyphens"
                ));
            }
            match skill.get("type").and_then(|v| v.as_str()) {
                Some("skill-md") | Some("archive") => {}
                _ => problems.push(format!(
                    "skills[{i}].type must be \"skill-md\" or \"archive\""
                )),
            }
            for field in ["description", "url"] {
                if skill
                    .get(field)
                    .and_then(|v| v.as_str())
                    .is_none_or(str::is_empty)
                {
                    problems.push(format!("skills[{i}] is missing {field}"));
                }
            }
            let digest_ok = skill
                .get("digest")
                .and_then(|v| v.as_str())
                .and_then(|d| d.strip_prefix("sha256:"))
                .is_some_and(|hex| hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()));
            if !digest_ok {
                problems.push(format!(
                    "skills[{i}].digest must be \"sha256:\" followed by 64 hex \
                     characters"
                ));
            }
        }
        if !problems.is_empty() {
            findings.push(finding(
                "AIREADY-AGSKILLS",
                "agent-skills entries fail the discovery schema",
                format!("Problems found: {}.", problems.join("; ")),
                "Fix each skill entry to satisfy the agentskills.io discovery \
                 0.2.0 schema: name (lowercase+hyphens), type (skill-md|archive), \
                 description, url, and a sha256 digest.",
                url,
                Severity::Warning,
            ));
        }
    }

    /// Check 12 (SEP-1649): MCP Server Card — serverInfo{name,version},
    /// transport endpoint, capabilities.
    fn check_mcp_card(
        &self,
        _ctx: &AnalysisContext,
        doc: &serde_json::Value,
        url: &str,
        findings: &mut Vec<Finding>,
    ) {
        let mut missing: Vec<&str> = Vec::new();
        let server_info = doc.get("serverInfo");
        if server_info.is_none_or(|v| !v.is_object()) {
            missing.push("serverInfo");
        } else {
            let info = server_info.unwrap_or(&serde_json::Value::Null);
            for field in ["name", "version"] {
                if info
                    .get(field)
                    .and_then(|v| v.as_str())
                    .is_none_or(str::is_empty)
                {
                    missing.push(if field == "name" {
                        "serverInfo.name"
                    } else {
                        "serverInfo.version"
                    });
                }
            }
        }

        // The transport endpoint: `endpoint` (URL string) or `transport` with
        // an `endpoint`/`url` member.
        let endpoint = doc.get("endpoint").and_then(|v| v.as_str()).or_else(|| {
            doc.get("transport").and_then(|t| {
                t.get("endpoint")
                    .or_else(|| t.get("url"))
                    .and_then(|v| v.as_str())
            })
        });
        match endpoint {
            Some(u) if !u.is_empty() => {
                if Url::parse(u).is_err() {
                    findings.push(finding(
                        "AIREADY-MCPCARD",
                        "MCP server card endpoint is not a valid URL",
                        format!("The transport endpoint `{u}` does not parse as a URL."),
                        "Point the card's transport endpoint at the absolute HTTP(S) \
                         URL agents should connect to.",
                        url,
                        Severity::Warning,
                    ));
                }
            }
            _ => missing.push("transport endpoint"),
        }

        let caps = doc.get("capabilities");
        if caps.is_none_or(|v| !v.is_object()) {
            missing.push("capabilities");
        } else {
            let caps = caps.unwrap_or(&serde_json::Value::Null);
            let announces = ["tools", "resources", "prompts"]
                .iter()
                .any(|c| caps.get(*c).is_some_and(|v| !v.is_null()));
            if !announces {
                missing.push("capabilities.tools/resources/prompts");
            }
        }

        if !missing.is_empty() {
            findings.push(finding(
                "AIREADY-MCPCARD",
                "MCP server card is missing required fields",
                format!(
                    "The MCP Server Card (SEP-1649) lacks: {}. MCP clients \
                     cannot bootstrap a connection from it.",
                    missing.join(", ")
                ),
                "Complete the card: serverInfo{name,version}, the transport \
                 endpoint URL, and a capabilities object announcing the \
                 tools/resources/prompts the server supports.",
                url,
                Severity::Warning,
            ));
        }
    }
}

impl Analyzer for AgentReadinessAnalyzer {
    fn name(&self) -> &str {
        "agent-readiness"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = self.analyze_site_level(ctx);
        // An HTML-served well-known document (a misconfigured server) still
        // reaches the registry path; validate it here too. `analyze_resource`
        // stands down when the opt-in prober owns the discovery checks.
        findings.extend(self.analyze_resource(ctx));
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    /// A minimal parsed page; readiness checks read the URL and raw data only.
    fn page_at(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
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

    /// A context over a leaked fixture: headers/body/robots live for the test.
    fn ctx<'a>(
        page: &'a ParsedPage,
        headers: &'a [(String, String)],
        body: Option<&'a str>,
        robots: Option<&'a str>,
        status: Option<u16>,
        content_type: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            body,
            status_code: status,
            headers,
            robots_txt: robots,
            content_type,
            ..AnalysisContext::new(page)
        }
    }

    fn leak<T>(value: T) -> &'static T
    where
        T: Sized,
    {
        Box::leak(Box::new(value))
    }

    fn leak_pairs(pairs: &[(&str, &str)]) -> &'static [(String, String)] {
        leak(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<Vec<_>>(),
        )
    }

    fn codes(findings: &[Finding]) -> Vec<String> {
        findings.iter().map(|f| f.code.clone()).collect()
    }

    const GOOD_ROBOTS: &str = "User-agent: *\nDisallow: /private/\n\n\
        Sitemap: https://site.example/sitemap.xml\n";

    // --- robots.txt (check 1) -------------------------------------------

    #[test]
    fn missing_robots_fails_the_robots_check() {
        let page = page_at("https://site.example/");
        let ctx = ctx(&page, &[], None, None, Some(200), Some("text/html"));
        let found = AgentReadinessAnalyzer::passive_only().analyze(&ctx);
        let robots = found
            .iter()
            .find(|f| f.code == "AIREADY-ROBOTS")
            .expect("missing robots.txt must fail check 1");
        assert_eq!(robots.severity, Severity::Warning);
        assert!(
            robots.recommendation.contains("/robots.txt"),
            "recommendation must be actionable: {}",
            robots.recommendation
        );
    }

    #[test]
    fn valid_robots_passes_checks_1_and_2() {
        let page = page_at("https://site.example/");
        let ctx = ctx(
            &page,
            &[],
            Some("<html></html>"),
            Some(GOOD_ROBOTS),
            Some(200),
            Some("text/html"),
        );
        let codes = codes(&AgentReadinessAnalyzer::passive_only().analyze(&ctx));
        assert!(!codes.contains(&"AIREADY-ROBOTS".to_string()), "{codes:?}");
        assert!(!codes.contains(&"AIREADY-SITEMAP".to_string()), "{codes:?}");
    }

    #[test]
    fn robots_without_directives_fails_check_1() {
        let page = page_at("https://site.example/");
        let robots = "# just a comment\nUser-agent: *\n";
        let ctx = ctx(&page, &[], None, Some(robots), Some(200), None);
        let found = AgentReadinessAnalyzer::passive_only().analyze(&ctx);
        let robots_finding = found
            .iter()
            .find(|f| f.code == "AIREADY-ROBOTS")
            .expect("a User-agent group with no rules must fail check 1");
        assert!(robots_finding.title.contains("no usable crawl directives"));
    }

    // --- sitemap reference (check 2) -------------------------------------

    #[test]
    fn robots_without_sitemap_reference_fails_check_2() {
        let page = page_at("https://site.example/");
        let robots = "User-agent: *\nDisallow: /private/\n";
        let ctx = ctx(&page, &[], None, Some(robots), Some(200), None);
        let found = AgentReadinessAnalyzer::passive_only().analyze(&ctx);
        assert!(
            found.iter().any(|f| f.code == "AIREADY-SITEMAP"),
            "no Sitemap: line must fail check 2"
        );
    }

    #[test]
    fn malformed_sitemap_reference_fails_check_2() {
        let page = page_at("https://site.example/");
        let robots = "User-agent: *\nDisallow: /\nSitemap: /sitemap.xml\n";
        let ctx = ctx(&page, &[], None, Some(robots), Some(200), None);
        let found = AgentReadinessAnalyzer::passive_only().analyze(&ctx);
        let sitemap = found
            .iter()
            .find(|f| f.code == "AIREADY-SITEMAP")
            .expect("a relative Sitemap: URL must fail check 2");
        assert!(
            sitemap.title.contains("invalid sitemap URL"),
            "relative reference must be reported as invalid: {}",
            sitemap.title
        );
    }

    // --- Content-Signal (check 3) ----------------------------------------

    #[test]
    fn content_signal_directives_pass_check_3() {
        let page = page_at("https://site.example/");
        let robots = "User-agent: *\nDisallow: /\nContent-Signal: ai-train=no, search=yes\n\
            Sitemap: https://site.example/sitemap.xml\n";
        let ctx = ctx(&page, &[], None, Some(robots), Some(200), None);
        let codes = codes(&AgentReadinessAnalyzer::passive_only().analyze(&ctx));
        assert!(!codes.contains(&"AIREADY-SIGNAL".to_string()), "{codes:?}");
    }

    #[test]
    fn robots_without_content_signal_emits_info_check_3() {
        let page = page_at("https://site.example/");
        let ctx = ctx(&page, &[], None, Some(GOOD_ROBOTS), Some(200), None);
        let found = AgentReadinessAnalyzer::passive_only().analyze(&ctx);
        let signal = found
            .iter()
            .find(|f| f.code == "AIREADY-SIGNAL")
            .expect("absent Content-Signal must report check 3");
        assert_eq!(signal.severity, Severity::Info);
        assert!(signal.recommendation.contains("Content-Signal"));
    }

    // --- AI crawler rules (check 4) ---------------------------------------

    #[test]
    fn explicit_ai_bot_group_passes_check_4() {
        let page = page_at("https://site.example/");
        let robots = "User-agent: GPTBot\nDisallow: /private/\n\n\
            User-agent: *\nDisallow: /\nSitemap: https://site.example/sitemap.xml\n";
        let ctx = ctx(&page, &[], None, Some(robots), Some(200), None);
        let codes = codes(&AgentReadinessAnalyzer::passive_only().analyze(&ctx));
        assert!(!codes.contains(&"AIREADY-AICRAWL".to_string()), "{codes:?}");
    }

    #[test]
    fn star_group_only_fails_check_4() {
        let page = page_at("https://site.example/");
        let ctx = ctx(&page, &[], None, Some(GOOD_ROBOTS), Some(200), None);
        let found = AgentReadinessAnalyzer::passive_only().analyze(&ctx);
        let ai = found
            .iter()
            .find(|f| f.code == "AIREADY-AICRAWL")
            .expect("no explicit AI-crawler group must fail check 4");
        assert!(
            ai.description.contains("GPTBot") && ai.description.contains("ClaudeBot"),
            "the bot list must be named in the finding: {}",
            ai.description
        );
    }

    #[test]
    fn ai_bot_name_matching_is_case_insensitive_and_group_scoped() {
        // `User-agent: gptbot` (lowercase) must match; a bot named in a group
        // with no rules must not.
        let robots = "User-agent: gptbot\nDisallow: /\n\n\
            User-agent: ClaudeBot\n\nUser-agent: *\nDisallow: /\n";
        let summary = summarize_robots(robots);
        assert!(summary.has_ai_crawler_rules, "lowercase gptbot must match");
    }

    #[test]
    fn ai_bot_without_rules_does_not_count() {
        // ClaudeBot is named but no group in the file carries an
        // Allow/Disallow, so the "explicit rules" requirement is not met.
        let robots = "User-agent: ClaudeBot\n";
        let summary = summarize_robots(robots);
        assert!(
            !summary.has_ai_crawler_rules,
            "a rule-less AI group must not satisfy check 4"
        );
    }

    // --- Link headers (check 6) -------------------------------------------

    #[test]
    fn homepage_without_discovery_link_headers_emits_info() {
        let page = page_at("https://site.example/");
        let headers = leak_pairs(&[("Content-Type", "text/html")]);
        let ctx = ctx(&page, headers, None, Some(GOOD_ROBOTS), Some(200), None);
        let found = AgentReadinessAnalyzer::passive_only().analyze(&ctx);
        let linkhdr = found
            .iter()
            .find(|f| f.code == "AIREADY-LINKHDR")
            .expect("homepage without discovery Link headers must report check 6");
        assert_eq!(linkhdr.severity, Severity::Info);
    }

    #[test]
    fn homepage_with_service_desc_link_header_passes_check_6() {
        let page = page_at("https://site.example/");
        let headers = leak_pairs(&[
            ("Content-Type", "text/html"),
            (
                "Link",
                "</openapi.json>; rel=\"service-desc\", </docs>; rel=\"service-doc\"",
            ),
        ]);
        let ctx = ctx(&page, headers, None, Some(GOOD_ROBOTS), Some(200), None);
        let codes = codes(&AgentReadinessAnalyzer::passive_only().analyze(&ctx));
        assert!(!codes.contains(&"AIREADY-LINKHDR".to_string()), "{codes:?}");
    }

    #[test]
    fn discovery_link_header_across_multiple_headers_passes() {
        let page = page_at("https://site.example/");
        let headers = leak_pairs(&[
            ("Link", "</.well-known/api-catalog>; rel=\"api-catalog\""),
            ("Link", "</openapi.json>; rel=describedby"),
        ]);
        let ctx = ctx(&page, headers, None, Some(GOOD_ROBOTS), Some(200), None);
        let codes = codes(&AgentReadinessAnalyzer::passive_only().analyze(&ctx));
        assert!(!codes.contains(&"AIREADY-LINKHDR".to_string()), "{codes:?}");
    }

    #[test]
    fn non_homepage_pages_skip_check_6() {
        let page = page_at("https://site.example/about");
        let headers = leak_pairs(&[]);
        let ctx = ctx(&page, headers, None, Some(GOOD_ROBOTS), Some(200), None);
        let codes = codes(&AgentReadinessAnalyzer::passive_only().analyze(&ctx));
        assert!(!codes.contains(&"AIREADY-LINKHDR".to_string()), "{codes:?}");
    }

    #[test]
    fn non_200_homepage_skips_check_6() {
        let page = page_at("https://site.example/");
        let headers = leak_pairs(&[]);
        let ctx = ctx(&page, headers, None, Some(GOOD_ROBOTS), Some(304), None);
        let codes = codes(&AgentReadinessAnalyzer::passive_only().analyze(&ctx));
        assert!(!codes.contains(&"AIREADY-LINKHDR".to_string()), "{codes:?}");
    }

    // --- well-known documents (checks 5, 10, 11, 12) -----------------------

    #[test]
    fn non_well_known_paths_are_silent_in_resource_mode() {
        let page = page_at("https://site.example/llms.txt");
        let ctx = ctx(
            &page,
            &[],
            Some("# llms"),
            None,
            Some(200),
            Some("text/plain"),
        );
        assert!(
            AgentReadinessAnalyzer::new()
                .analyze_resource(&ctx)
                .is_empty(),
            "ordinary non-HTML resources must produce no readiness findings"
        );
    }

    #[test]
    fn well_known_path_with_non_200_fails_its_check() {
        let page = page_at("https://site.example/.well-known/agent-card.json");
        let ctx = ctx(
            &page,
            &[],
            Some("not found"),
            None,
            Some(404),
            Some("text/plain"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        assert_eq!(codes(&found), vec!["AIREADY-A2A".to_string()]);
        assert!(found[0].description.contains("404"));
    }

    #[test]
    fn well_known_path_with_invalid_json_fails_its_check() {
        let page = page_at("https://site.example/.well-known/mcp/server-card.json");
        let ctx = ctx(
            &page,
            &[],
            Some("<html>oops</html>"),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        assert_eq!(codes(&found), vec!["AIREADY-MCPCARD".to_string()]);
        assert!(found[0].title.contains("not valid JSON"));
    }

    #[test]
    fn valid_agent_card_passes_check_10() {
        let page = page_at("https://site.example/.well-known/agent-card.json");
        let body = r#"{
            "name": "example-agent",
            "version": "1.0.0",
            "description": "Does things for agents",
            "supportedInterfaces": [{"url": "https://site.example/a2a", "transport": "JSONRPC"}],
            "capabilities": {"streaming": true},
            "skills": [{"id": "s1", "name": "search", "description": "Site search"}]
        }"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        assert!(
            AgentReadinessAnalyzer::new()
                .analyze_resource(&ctx)
                .is_empty(),
            "a complete agent card must pass"
        );
    }

    #[test]
    fn agent_card_missing_fields_fails_check_10() {
        let page = page_at("https://site.example/.well-known/agent-card.json");
        let body = r#"{"name": "example-agent"}"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        let a2a = found
            .iter()
            .find(|f| f.code == "AIREADY-A2A")
            .expect("an incomplete card must fail check 10");
        assert!(
            a2a.description.contains("version")
                && a2a.description.contains("skills")
                && a2a.description.contains("supportedInterfaces"),
            "missing fields must be enumerated: {}",
            a2a.description
        );
    }

    #[test]
    fn agent_card_with_bad_skills_fails_check_10() {
        let page = page_at("https://site.example/.well-known/agent-card.json");
        let body = r#"{
            "name": "a", "version": "1", "description": "d",
            "supportedInterfaces": [{"url": "https://x.example"}],
            "capabilities": {},
            "skills": [{"id": "s1"}]
        }"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        assert!(
            found
                .iter()
                .any(|f| f.title.contains("skills are missing id/name/description")),
            "a skill without name/description must be reported: {:?}",
            found.iter().map(|f| &f.title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn valid_api_catalog_passes_check_5() {
        let page = page_at("https://site.example/.well-known/api-catalog");
        let body = r#"{"linkset": [{"anchor": "https://site.example/",
            "service-desc": "https://site.example/openapi.json"}]}"#;
        let headers = leak_pairs(&[("Content-Type", "application/linkset+json")]);
        let ctx = ctx(
            &page,
            headers,
            Some(body),
            None,
            Some(200),
            Some("application/linkset+json"),
        );
        assert!(
            AgentReadinessAnalyzer::new()
                .analyze_resource(&ctx)
                .is_empty(),
            "a complete api-catalog must pass"
        );
    }

    #[test]
    fn api_catalog_without_linkset_fails_check_5() {
        let page = page_at("https://site.example/.well-known/api-catalog");
        let ctx = ctx(
            &page,
            &[],
            Some(r#"{"services": []}"#),
            None,
            Some(200),
            Some("application/linkset+json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        let cat = found
            .iter()
            .find(|f| f.code == "AIREADY-APICATALOG")
            .expect("a catalog without linkset must fail check 5");
        assert!(cat.title.contains("no linkset array"));
    }

    #[test]
    fn api_catalog_entry_without_anchor_fails_check_5() {
        let page = page_at("https://site.example/.well-known/api-catalog");
        let body = r#"{"linkset": [{"service-desc": "https://x.example/openapi.json"}]}"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        let cat = found
            .iter()
            .find(|f| f.code == "AIREADY-APICATALOG")
            .expect("an entry without anchor must fail check 5");
        assert!(
            cat.description.contains("anchor") && cat.description.contains("linkset[0]"),
            "the specific entry must be named: {}",
            cat.description
        );
    }

    #[test]
    fn api_catalog_with_wrong_media_type_is_reported() {
        let page = page_at("https://site.example/.well-known/api-catalog");
        let body = r#"{"linkset": [{"anchor": "https://x.example/",
            "service-doc": "https://x.example/docs"}]}"#;
        let ctx = ctx(&page, &[], Some(body), None, Some(200), Some("text/html"));
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        assert!(
            found
                .iter()
                .any(|f| f.title.contains("unexpected media type")),
            "non-linkset+json media type must be reported: {:?}",
            found.iter().map(|f| &f.title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn valid_agent_skills_index_passes_check_11() {
        let page = page_at("https://site.example/.well-known/agent-skills/index.json");
        let body = r#"{
            "$schema": "https://schemas.agentskills.io/discovery/0.2.0/schema.json",
            "skills": [{
                "name": "site-search",
                "type": "skill-md",
                "description": "Search the site",
                "url": "https://site.example/skills/search.md",
                "digest": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            }]
        }"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        assert!(
            AgentReadinessAnalyzer::new()
                .analyze_resource(&ctx)
                .is_empty(),
            "a schema-complete skills index must pass"
        );
    }

    #[test]
    fn agent_skills_index_with_bad_digest_and_name_fails_check_11() {
        let page = page_at("https://site.example/.well-known/agent-skills/index.json");
        let body = r#"{
            "$schema": "https://schemas.agentskills.io/discovery/0.2.0/schema.json",
            "skills": [{
                "name": "Site Search",
                "type": "skill-md",
                "description": "d", "url": "https://x.example/s.md",
                "digest": "sha256:deadbeef"
            }]
        }"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        let skills = found
            .iter()
            .find(|f| f.code == "AIREADY-AGSKILLS")
            .expect("bad name/digest must fail check 11");
        assert!(
            skills.description.contains("skills[0].name") && skills.description.contains("digest"),
            "both problems must be listed: {}",
            skills.description
        );
    }

    #[test]
    fn agent_skills_index_with_wrong_schema_fails_check_11() {
        let page = page_at("https://site.example/.well-known/agent-skills/index.json");
        let body = r#"{"$schema": "https://example.com/other.json", "skills": []}"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        assert!(
            found
                .iter()
                .any(|f| f.title.contains("pins no discovery schema")),
            "an unrecognised $schema must be reported: {:?}",
            found.iter().map(|f| &f.title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn valid_mcp_server_card_passes_check_12() {
        let page = page_at("https://site.example/.well-known/mcp/server-card.json");
        let body = r#"{
            "serverInfo": {"name": "site-mcp", "version": "2.1.0"},
            "endpoint": "https://site.example/mcp",
            "capabilities": {"tools": {}, "resources": {}, "prompts": {}}
        }"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        assert!(
            AgentReadinessAnalyzer::new()
                .analyze_resource(&ctx)
                .is_empty(),
            "a complete MCP server card must pass"
        );
    }

    #[test]
    fn mcp_card_missing_server_info_fails_check_12() {
        let page = page_at("https://site.example/.well-known/mcp/server-card.json");
        let body = r#"{"endpoint": "https://site.example/mcp", "capabilities": {"tools": {}}}"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        let card = found
            .iter()
            .find(|f| f.code == "AIREADY-MCPCARD")
            .expect("a card without serverInfo must fail check 12");
        assert!(
            card.description.contains("serverInfo"),
            "the missing field must be named: {}",
            card.description
        );
    }

    #[test]
    fn mcp_card_with_invalid_endpoint_url_fails_check_12() {
        let page = page_at("https://site.example/.well-known/mcp/server-card.json");
        let body = r#"{
            "serverInfo": {"name": "m", "version": "1"},
            "endpoint": "not a url",
            "capabilities": {"tools": {}}
        }"#;
        let ctx = ctx(
            &page,
            &[],
            Some(body),
            None,
            Some(200),
            Some("application/json"),
        );
        let found = AgentReadinessAnalyzer::passive_only().analyze_resource(&ctx);
        assert!(
            found
                .iter()
                .any(|f| f.title.contains("endpoint is not a valid URL")),
            "an unparseable endpoint must be reported: {:?}",
            found.iter().map(|f| &f.title).collect::<Vec<_>>()
        );
    }

    // --- determinism -------------------------------------------------------

    #[test]
    fn output_order_is_deterministic() {
        let page = page_at("https://site.example/");
        let build = || {
            let ctx = ctx(&page, leak_pairs(&[]), None, None, Some(200), None);
            AgentReadinessAnalyzer::passive_only().analyze(&ctx)
        };
        let first = build();
        let second = build();
        // robots.txt is absent here, so checks 2-4 are unassessable and only
        // the robots-missing gate plus the homepage Link-header check fire;
        // the assertion pins that order and its stability across calls.
        let expected = vec!["AIREADY-ROBOTS", "AIREADY-LINKHDR"];
        assert_eq!(codes(&first), expected, "check order is the report order");
        let signature = |findings: &[Finding]| -> Vec<(String, String)> {
            findings
                .iter()
                .map(|f| (f.code.clone(), f.description.clone()))
                .collect()
        };
        assert_eq!(signature(&first), signature(&second));
    }

    #[test]
    fn analyzer_name_is_stable() {
        assert_eq!(AgentReadinessAnalyzer::new().name(), "agent-readiness");
    }

    #[test]
    fn prober_ownership_stands_down_the_discovery_checks() {
        // The ownership contract: with the opt-in prober enabled it owns
        // checks 5, 6 and 10-12, so the passive analyzer must emit nothing
        // for them while the robots-family checks (1-4) keep running.
        let prober_owns = AgentReadinessAnalyzer {
            discovery_owned_by_prober: true,
        };
        let home = page_at("https://site.example/");
        let home_ctx = ctx(&home, leak_pairs(&[]), None, None, Some(200), None);
        let found = prober_owns.analyze(&home_ctx);
        let got = codes(&found);
        assert!(
            !got.contains(&"AIREADY-LINKHDR".to_string()),
            "check 6 must stand down when the prober owns it: {got:?}"
        );
        assert!(
            got.contains(&"AIREADY-ROBOTS".to_string()),
            "robots-family checks always run: {got:?}"
        );
        let card = page_at("https://site.example/.well-known/agent-card.json");
        let card_ctx = ctx(
            &card,
            &[],
            Some("null"),
            None,
            Some(200),
            Some("application/json"),
        );
        assert!(
            prober_owns.analyze_resource(&card_ctx).is_empty(),
            "well-known validation must stand down when the prober owns it"
        );
    }
}
