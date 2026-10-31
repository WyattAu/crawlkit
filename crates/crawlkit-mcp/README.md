# crawlkit-mcp

MCP (Model Context Protocol) server exposing crawlkit to AI agents — Claude
Code, Claude Desktop, Cursor, or any MCP client. Spec G1 in
`docs/GAP_CLOSURE_PLAN.md`.

Implements MCP over **stdio** (newline-delimited JSON-RPC) using the official
Rust MCP SDK [`rmcp`](https://crates.io/crates/rmcp). Tools only — no
resources or prompts in v1. Every tool is a thin wrapper over existing
`crawlkit-engine` / `crawlkit-api` functions; there is no analysis logic in
this crate.

## Tools

| Tool | Arguments | Delegates to |
|---|---|---|
| `crawl` | `url`, `out_dir`, `max_pages?`, `request_delay_ms?`, `respect_robots_txt?` | `crawl_engine::CrawlEngine::run` |
| `compare` | `baseline`, `target`, `format?` (`json`\|`markdown`) | `compare::compare_crawls` |
| `report` | `crawl_dir`, `crawl_id?`, `format?` (`json`\|`markdown`\|`csv`\|`html`) | `export::{export_json,export_markdown,export_csv,export_html}` |
| `findings_search` | `db_path`, `crawl_id?`, `severity?`, `category?`, `code_prefix?`, `limit?` | `storage::Storage::get_issues` |
| `schedule_list` | `base_url?` | crawlkit-api `GET /api/v1/schedules` |

`crawl` blocks until the crawl finishes (respecting `max_pages` and the
politeness delay) and returns the run id plus page/issue counts. The other
tools accept either a crawl **directory** (`<dir>/crawlkit.db`, the CLI
convention) or a direct path to the SQLite file.

## Build & run

```sh
RUSTUP_TOOLCHAIN=1.95.0 cargo build -p crawlkit-mcp
./target/debug/crawlkit-mcp            # serves MCP on stdin/stdout
./target/debug/crawlkit-mcp --api-key SECRET
```

Logs go to stderr only — stdout is reserved for the JSON-RPC channel. Set
`RUST_LOG` (e.g. `RUST_LOG=info`) for more verbose tracing.

## Claude Code

```sh
claude mcp add crawlkit -- crawlkit-mcp
# or with the API-key gate:
claude mcp add crawlkit -- crawlkit-mcp --api-key SECRET
```

## Claude Desktop / generic MCP clients

Add to `claude_desktop_config.json` (or the equivalent MCP client config):

```json
{
  "mcpServers": {
    "crawlkit": {
      "command": "/path/to/crawlkit-mcp",
      "args": [],
      "env": { "CRAWLKIT_MCP_API_KEY": "SECRET" }
    }
  }
}
```

## Authentication

The optional API-key gate is enabled by passing `--api-key` or setting
`CRAWLKIT_MCP_API_KEY`. When enabled, **every `tools/call`** must carry the
key in the request `_meta` object under `crawlkit.api_key`; missing or wrong
keys get a JSON-RPC error `-32001`:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "findings_search",
    "arguments": { "db_path": "./crawls/example" },
    "_meta": { "crawlkit.api_key": "SECRET" }
  }
}
```

The same key, when set, is also forwarded as the bearer token to the
crawlkit-api server by `schedule_list`.

## Example session (raw JSON-RPC)

```sh
crawlkit-mcp <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"curl","version":"0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"findings_search","arguments":{"db_path":"./examples/demo-site","severity":"error","limit":5}}}
EOF
```

## Tests

```sh
RUSTUP_TOOLCHAIN=1.95.0 cargo test -p crawlkit-mcp
```

Covers: tool-schema generation (every tool advertises a JSON-Schema object),
the API-key gate, and the JSON-RPC `initialize` handshake + `tools/list`
round-trip driven over an in-memory duplex pipe (no subprocess).

## Out of scope (v1)

Polling/async crawl jobs, `schedule add/toggle`, `export`, `doc`, and
Streamable HTTP transport are follow-ups from spec G1.
