use anyhow::{Context, Result};
use crawlkit_engine::log_analyzer::analyze_log_entries;
use crawlkit_engine::log_parser::{parse_log_line, LogFormat};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct LogAnalyzeArgs {
    /// Path to log file
    #[arg(short, long)]
    pub path: PathBuf,

    /// Log format: nginx-combined, apache-combined, or json
    #[arg(long, default_value = "nginx-combined")]
    pub format: String,

    /// Output file for JSON report
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn run(args: LogAnalyzeArgs) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("Parsing log file...");

    let content = fs::read_to_string(&args.path)
        .with_context(|| format!("Failed to read log file: {}", args.path.display()))?;

    let format = match args.format.as_str() {
        "nginx-combined" => LogFormat::NginxCombined,
        "apache-combined" => LogFormat::ApacheCombined,
        "json" => LogFormat::JsonStructured,
        _ => return Err(anyhow::anyhow!("Unsupported format: {}", args.format)),
    };

    let entries: Vec<_> = content
        .lines()
        .filter_map(|line| parse_log_line(line, &format))
        .collect();

    pb.set_message(format!("Analyzing {} log entries...", entries.len()));

    let analysis = analyze_log_entries(&entries);

    let report = serde_json::to_string_pretty(&analysis)?;

    pb.finish_with_message("Log analysis complete");

    if let Some(out) = &args.output {
        fs::write(out, &report)
            .with_context(|| format!("Failed to write output to {}", out.display()))?;
        tracing::info!("Wrote log analysis to {}", out.display());
    } else {
        println!("{}", report);
    }

    Ok(())
}
