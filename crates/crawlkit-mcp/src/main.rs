//! `crawlkit-mcp` binary: expose crawlkit over the Model Context Protocol on
//! stdio (spec G1). See the crate-level docs and the README for client
//! configuration.

use clap::Parser;

/// CLI options for the crawlkit MCP server.
#[derive(Debug, Parser)]
#[command(
    name = "crawlkit-mcp",
    version,
    about = "Expose crawlkit over the Model Context Protocol (stdio JSON-RPC)"
)]
struct Cli {
    /// Require this API key on every tools/call via
    /// `_meta["crawlkit.api_key"]`. Falls back to the CRAWLKIT_MCP_API_KEY
    /// environment variable. Omit to run without the gate.
    #[arg(long)]
    api_key: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let api_key = cli
        .api_key
        .or_else(|| std::env::var(crawlkit_mcp::ENV_API_KEY).ok());

    // Tracing must never touch stdout: that stream is the JSON-RPC channel.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let service = crawlkit_mcp::serve_stdio(api_key).await?;
    service.waiting().await?;
    Ok(())
}
