//! # crawlkit-mcp
//!
//! MCP (Model Context Protocol) server exposing crawlkit over JSON-RPC —
//! spec G1 in `docs/GAP_CLOSURE_PLAN.md`. Tools only, no resources or prompts
//! in v1.
//!
//! Every tool is a thin wrapper over an existing `crawlkit-engine` or
//! `crawlkit-api` function; this crate contains no analysis logic.
//!
//! | Tool | Delegates to |
//! |---|---|
//! | `crawl` | [`crawlkit_engine::crawl_engine::CrawlEngine::run`] |
//! | `compare` | [`crawlkit_engine::compare::compare_crawls`] |
//! | `report` | [`crawlkit_engine::export`] renderers |
//! | `findings_search` | [`crawlkit_engine::storage::Storage::get_issues`] |
//! | `schedule_list` | crawlkit-api `GET /api/v1/schedules` |
//!
//! Transport framing uses the official Rust MCP SDK ([`rmcp`]); the binary
//! serves newline-delimited JSON-RPC over stdio (see `main.rs`).
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Serve MCP over stdin/stdout (the Claude-Code-style launch path).
//! crawlkit_mcp::serve_stdio(None).await?;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;

use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{ErrorCode, Implementation, ServerCapabilities, ServerConfig},
    service::{RequestContext, RunningService},
    tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt,
};

pub mod convert;
pub mod tools;

/// Environment variable consulted when `--api-key` is not passed.
pub const ENV_API_KEY: &str = "CRAWLKIT_MCP_API_KEY";
/// Server name advertised during the MCP initialize handshake.
pub const SERVER_NAME: &str = "crawlkit-mcp";
/// Server version advertised during the MCP initialize handshake.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON-RPC error code used for failed API-key checks (server-defined range).
const ERROR_UNAUTHORIZED: i32 = -32001;

/// The crawlkit MCP server.
///
/// Holds the optional API key (when set, every `tools/call` must carry it in
/// `_meta`) and the tool router generated from the `#[tool]` methods below.
#[derive(Debug, Clone)]
pub struct CrawlkitMcp {
    api_key: Option<Arc<str>>,
    tool_router: ToolRouter<Self>,
}

impl CrawlkitMcp {
    /// Create a server. `api_key` enables the `_meta`-based gate when set.
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key: api_key.map(Into::into),
            tool_router: Self::tool_router(),
        }
    }

    /// Enforce the optional API-key gate on an incoming request.
    fn authorize(&self, meta: &rmcp::model::MetaObject) -> Result<(), ErrorData> {
        let expected = self.api_key.as_deref();
        let provided = meta
            .0
            .get(tools::META_API_KEY_FIELD)
            .and_then(serde_json::Value::as_str);
        match check_api_key(expected, provided) {
            Ok(()) => Ok(()),
            Err(message) => Err(ErrorData::new(ErrorCode(ERROR_UNAUTHORIZED), message, None)),
        }
    }
}

/// Pure API-key check: `Ok` when the gate is disabled (`expected` is `None`)
/// or the provided key matches, otherwise `Err` with the client-facing
/// message. The comparison is constant-time so a caller on a bridged
/// (non-stdio) transport cannot recover the key by timing byte-wise
/// short-circuits. Extracted as a function so the gate is unit-testable
/// without a live transport.
fn check_api_key(expected: Option<&str>, provided: Option<&str>) -> Result<(), String> {
    use subtle::ConstantTimeEq;
    match expected {
        None => Ok(()),
        Some(expected) => {
            // Constant-time content comparison; `None` stays a miss (an
            // empty-string key must not be bypassable by omitting the field).
            let matched =
                provided.is_some_and(|p| bool::from(expected.as_bytes().ct_eq(p.as_bytes())));
            if matched {
                Ok(())
            } else {
                Err(format!(
                    "unauthorized: requests must carry the API key in _meta.{}",
                    tools::META_API_KEY_FIELD
                ))
            }
        }
    }
}

// The generated `<tool>_tool_attr` schema functions use `panic!` as the
// schema-derivation failure path inside the rmcp macro; that code is not
// ours to edit, so the workspace `clippy::panic` deny is scoped out here.
#[allow(clippy::panic)]
#[tool_router]
impl CrawlkitMcp {
    /// Crawl a site and return the run ID plus a summary. Blocks until the
    /// crawl completes or hits its page/time limits.
    #[tool(
        description = "Start a web crawl of `url` and wait for it to finish. Writes the SQLite \
                       store to `<out_dir>/crawlkit.db` and returns the crawl id plus page/issue \
                       counts. Use `findings_search`, `report`, or `compare` on the result."
    )]
    async fn crawl(
        &self,
        Parameters(args): Parameters<tools::CrawlArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<convert::CrawlSummary>, ErrorData> {
        self.authorize(&context.meta)?;
        Ok(Json(tools::run_crawl(args).await?))
    }

    /// Diff two crawl stores: added/removed pages, status/title/content
    /// changes, and Core Web Vitals shifts.
    #[tool(
        description = "Compare two crawls (directories containing crawlkit.db, or the .db files \
                       themselves) and return the diff as JSON or markdown."
    )]
    fn compare(
        &self,
        Parameters(args): Parameters<tools::CompareArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<convert::CompareOutput>, ErrorData> {
        self.authorize(&context.meta)?;
        Ok(Json(tools::run_compare(args)?))
    }

    /// Render a report for a crawl in json, markdown, csv, or html.
    #[tool(
        description = "Render a report for an existing crawl (directory or crawlkit.db file). \
                       Formats: json (default), markdown, csv, html. Defaults to the latest \
                       crawl in the store."
    )]
    fn report(
        &self,
        Parameters(args): Parameters<tools::ReportArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<convert::ReportOutput>, ErrorData> {
        self.authorize(&context.meta)?;
        Ok(Json(tools::render_report(args)?))
    }

    /// Query findings from a crawl's SQLite store with optional
    /// severity/category/code filters.
    #[tool(
        description = "Search analysis findings in a crawl's SQLite store. Filter by severity \
                       (critical|error|warning|info), category (seo, security, links, ...), or \
                       issue-code prefix. Returns at most 500 rows (default 50) plus the total \
                       match count."
    )]
    fn findings_search(
        &self,
        Parameters(args): Parameters<tools::FindingsSearchArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<convert::FindingsOutput>, ErrorData> {
        self.authorize(&context.meta)?;
        Ok(Json(tools::search_findings(args)?))
    }

    /// List scheduled crawls from a running crawlkit-api server.
    #[tool(
        description = "List scheduled crawls configured on a running crawlkit-api server \
                       (GET /api/v1/schedules). Set base_url if the API is not on \
                       http://127.0.0.1:8080."
    )]
    async fn schedule_list(
        &self,
        Parameters(args): Parameters<tools::ScheduleListArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<convert::ScheduleListOutput>, ErrorData> {
        self.authorize(&context.meta)?;
        let api_key = self.api_key.as_deref();
        Ok(Json(tools::list_schedules(args, api_key).await?))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CrawlkitMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, SERVER_VERSION))
            .with_instructions(
                "crawlkit: crawl sites, search findings, render reports, compare crawls, and \
                 list schedules. Start with `crawl`, then query the returned crawl directory.",
            )
    }
}

/// Serve MCP over stdin/stdout (newline-delimited JSON-RPC, rmcp stdio
/// transport). Returns once the client disconnects or the transport closes.
pub async fn serve_stdio(
    api_key: Option<String>,
) -> Result<RunningService<RoleServer, CrawlkitMcp>, rmcp::service::ServerInitializeError> {
    CrawlkitMcp::new(api_key)
        .serve(rmcp::transport::stdio())
        .await
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    /// Every tool this server must expose (spec G1 v1).
    const EXPECTED_TOOLS: [&str; 5] = [
        "compare",
        "crawl",
        "findings_search",
        "report",
        "schedule_list",
    ];

    #[test]
    fn tool_schemas_validate() {
        let tools = CrawlkitMcp::tool_router().list_all();
        assert_eq!(
            tools.len(),
            EXPECTED_TOOLS.len(),
            "tool set drifted: {:?}",
            tools.iter().map(|t| t.name.as_ref()).collect::<Vec<_>>()
        );
        for tool in &tools {
            assert!(
                EXPECTED_TOOLS.contains(&tool.name.as_ref()),
                "unexpected tool '{}'",
                tool.name
            );
            let description = tool.description.as_deref().unwrap_or_default();
            assert!(
                !description.is_empty(),
                "{}: missing description",
                tool.name
            );

            // Each input schema must be a JSON-Schema object with properties.
            let schema = &tool.input_schema;
            assert_eq!(
                schema.get("type").and_then(Value::as_str),
                Some("object"),
                "{}: input schema is not an object",
                tool.name
            );
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .unwrap_or_else(|| panic!("{}: input schema has no properties", tool.name));
            assert!(!properties.is_empty(), "{}: no properties", tool.name);
        }

        // Spot-check the primary tool's required arguments.
        let crawl = tools
            .iter()
            .find(|t| t.name == "crawl")
            .expect("crawl tool registered");
        for required in ["url", "out_dir"] {
            assert!(
                crawl
                    .input_schema
                    .get("properties")
                    .is_some_and(|p| p.get(required).is_some()),
                "crawl schema is missing `{required}`"
            );
        }
    }

    #[test]
    fn api_key_gate() {
        // Gate disabled: everything passes.
        assert!(check_api_key(None, None).is_ok());
        assert!(check_api_key(None, Some("anything")).is_ok());
        // Gate enabled: exact match passes, everything else is rejected.
        assert!(check_api_key(Some("s3cret"), Some("s3cret")).is_ok());
        let err = check_api_key(Some("s3cret"), Some("wrong")).unwrap_err();
        assert!(err.contains(tools::META_API_KEY_FIELD));
        assert!(check_api_key(Some("s3cret"), None).is_err());
        // Length mismatch must reject, and omitting the key must not
        // accidentally match an empty-string configured key.
        assert!(check_api_key(Some("s"), Some("s3cret")).is_err());
        assert!(check_api_key(Some("s3cret"), Some("")).is_err());
        assert!(check_api_key(Some(""), None).is_err());
        assert!(check_api_key(Some(""), Some("")).is_ok());
    }

    /// Drive the real rmcp service over an in-memory duplex pipe — the same
    /// newline-delimited framing the stdio transport uses — and verify the
    /// initialize handshake plus a tools/list round-trip, without spawning a
    /// subprocess.
    #[tokio::test]
    async fn initialize_handshake_and_tools_list() {
        let server = CrawlkitMcp::new(None);
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let (server_read, server_write) = tokio::io::split(server_io);
        // rmcp's server-side serve() waits for the initialize request before
        // returning, so it must run concurrently with the client writes.
        let service = tokio::spawn(server.serve((server_read, server_write)));

        let (client_read, mut client_write) = tokio::io::split(client_io);
        let mut reader = BufReader::new(client_read);

        async fn send(
            writer: &mut (impl tokio::io::AsyncWrite + Unpin),
            raw: &[u8],
        ) -> std::io::Result<()> {
            writer.write_all(raw).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await
        }
        async fn read_line(reader: &mut BufReader<impl tokio::io::AsyncRead + Unpin>) -> String {
            let mut line = String::new();
            tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
                .await
                .expect("timed out waiting for a response")
                .expect("transport read failed");
            line
        }

        // 1. initialize → result with our serverInfo and tools capability.
        send(
            &mut client_write,
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"handshake-test","version":"0"}}}"#,
        )
        .await
        .expect("write initialize");
        let running = service
            .await
            .expect("serve task panicked")
            .expect("service failed to start");
        let line = read_line(&mut reader).await;
        let response: Value = serde_json::from_str(&line).expect("invalid JSON response");
        assert_eq!(response["jsonrpc"], "2.0", "raw response: {line}");
        assert_eq!(response["id"], 1);
        let result = &response["result"];
        assert!(
            result["serverInfo"]["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("crawlkit")),
            "serverInfo.name missing: {result}"
        );
        assert!(
            result["capabilities"]["tools"].is_object(),
            "tools capability not advertised: {result}"
        );
        assert!(
            result["protocolVersion"].is_string(),
            "protocol version not negotiated: {result}"
        );

        // 2. notifications/initialized completes the handshake.
        send(
            &mut client_write,
            br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        )
        .await
        .expect("write initialized notification");

        // 3. tools/list round-trips through the router.
        send(
            &mut client_write,
            br#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        )
        .await
        .expect("write tools/list");
        let line = read_line(&mut reader).await;
        let response: Value = serde_json::from_str(&line).expect("invalid JSON response");
        assert_eq!(response["id"], 2);
        let tools = response["result"]["tools"]
            .as_array()
            .expect("tools/list result");
        let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, EXPECTED_TOOLS, "wire tool list drifted");

        let _ = running.cancel().await;
    }
}
