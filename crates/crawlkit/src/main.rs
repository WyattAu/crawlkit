use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "crawlkit",
    about = "A high-performance Rust-based site crawler for SEO analysis",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Crawl a website and analyze pages for SEO signals
    Crawl {
        /// The starting URL to crawl
        url: String,

        /// Maximum number of pages to crawl
        #[arg(long, default_value = "100")]
        max_pages: usize,

        /// Delay between requests in milliseconds
        #[arg(long, default_value = "500")]
        delay: u64,

        /// Number of concurrent fetchers
        #[arg(long, default_value = "4")]
        concurrency: usize,

        /// Output file path (JSON)
        #[arg(short, long, default_value = "crawl-results.json")]
        output: String,
    },

    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("crawlkit=info".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Crawl {
            url,
            max_pages,
            delay,
            concurrency,
            output,
        } => {
            tracing::info!(
                "Starting crawl of {} (max_pages={}, delay={}ms, concurrency={})",
                url,
                max_pages,
                delay,
                concurrency
            );

            // TODO: Implement crawl loop (Phase 0 tasks 0.4–0.9)
            tracing::warn!("Crawl not yet implemented — Phase 0 foundation in progress");
            tracing::info!("Results would be written to: {}", output);
        }
        Commands::Version => {
            println!("crawlkit {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
