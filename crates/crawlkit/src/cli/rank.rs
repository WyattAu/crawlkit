//! CLI handlers for keyword rank tracking (`crawlkit rank ...`).

use anyhow::{Context, Result};
use clap::Subcommand;
use crawlkit_engine::rank::{
    analyze_rank_trend, DuckDuckGoProvider, GscSerpProvider, RankPosition, RankSnapshot,
    RankTracker, SerpProvider,
};
use crawlkit_engine::storage::Storage;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Subcommands for `crawlkit rank`.
#[derive(Subcommand)]
pub enum RankCommands {
    /// Add a rank-tracking project with one or more keywords
    Add {
        /// Domain to track (e.g. example.com)
        domain: String,

        /// Keyword to track (repeat for multiple)
        #[arg(short, long = "keyword")]
        keywords: Vec<String>,

        /// Search engine backend: duckduckgo or gsc
        #[arg(long, default_value = "duckduckgo")]
        engine: String,

        /// Device profile: desktop or mobile
        #[arg(long, default_value = "desktop")]
        device: String,

        /// Locale tag (e.g. en-US)
        #[arg(long, default_value = "en-US")]
        locale: String,

        /// Rank database path (default: ~/.crawlkit/rank.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },

    /// List rank projects and their keywords
    List {
        /// Only show the project for this domain
        domain: Option<String>,

        /// Output format: json or md
        #[arg(long, default_value = "md")]
        format: String,

        /// Rank database path (default: ~/.crawlkit/rank.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },

    /// Check current positions for a project's enabled keywords
    Check {
        /// Only check the project for this domain
        domain: Option<String>,

        /// Delay between keyword checks in milliseconds
        #[arg(long, default_value_t = 1500)]
        delay_ms: u64,

        /// Rank database path (default: ~/.crawlkit/rank.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },

    /// Show position history for a single keyword
    History {
        /// Project domain
        domain: String,

        /// Keyword to show history for
        #[arg(short, long)]
        keyword: String,

        /// Lookback window in days
        #[arg(long, default_value_t = 30)]
        days: u32,

        /// Output format: json or md
        #[arg(long, default_value = "md")]
        format: String,

        /// Rank database path (default: ~/.crawlkit/rank.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },

    /// Summarize position trends across keywords
    Trend {
        /// Only analyze the project for this domain
        domain: Option<String>,

        /// Lookback window in days
        #[arg(long, default_value_t = 90)]
        days: u32,

        /// Output format: json or md
        #[arg(long, default_value = "md")]
        format: String,

        /// Rank database path (default: ~/.crawlkit/rank.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

/// Default rank database location.
fn default_db() -> Result<PathBuf> {
    let home = dirs::home_dir().context("cannot determine home directory for rank database")?;
    Ok(home.join(".crawlkit").join("rank.db"))
}

fn open_db(path: Option<&Path>) -> Result<Storage> {
    let db = match path {
        Some(p) => p.to_path_buf(),
        None => default_db()?,
    };
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    Storage::new(&db).with_context(|| format!("failed to open rank database {}", db.display()))
}

fn select_projects(
    storage: &Storage,
    domain: Option<&str>,
) -> Result<Vec<crawlkit_engine::rank::RankProject>> {
    let projects = storage
        .list_rank_projects()
        .context("failed to list rank projects")?;
    match domain {
        Some(d) => {
            let filtered: Vec<_> = projects
                .into_iter()
                .filter(|p| p.domain.eq_ignore_ascii_case(d))
                .collect();
            if filtered.is_empty() {
                return Err(anyhow::anyhow!("no rank project found for domain: {d}"));
            }
            Ok(filtered)
        }
        None => Ok(projects),
    }
}

fn build_provider(engine: &str) -> Result<Box<dyn SerpProvider>> {
    match engine.to_ascii_lowercase().as_str() {
        "duckduckgo" => Ok(Box::new(DuckDuckGoProvider::new_default())),
        "gsc" => GscSerpProvider::from_env()
            .map(|p| Box::new(p) as Box<dyn SerpProvider>)
            .context(
                "GSC provider requires GSC_ACCESS_TOKEN and GSC_SITE_URL environment variables",
            ),
        other => Err(anyhow::anyhow!(
            "unknown search engine: {other} (use duckduckgo or gsc)"
        )),
    }
}

fn format_position(pos: Option<u16>) -> String {
    pos.map(|p| p.to_string())
        .unwrap_or_else(|| "-".to_string())
}

/// Entry point for `crawlkit rank <subcommand>`.
pub async fn run(command: RankCommands) -> Result<()> {
    match command {
        RankCommands::Add {
            domain,
            keywords,
            engine,
            device,
            locale,
            db,
        } => {
            if keywords.is_empty() {
                return Err(anyhow::anyhow!("at least one --keyword is required"));
            }
            let engine = engine.to_ascii_lowercase();
            if !matches!(engine.as_str(), "duckduckgo" | "gsc") {
                return Err(anyhow::anyhow!(
                    "unknown search engine: {engine} (use duckduckgo or gsc)"
                ));
            }
            let storage = open_db(db.as_deref())?;
            let project_id = storage.add_rank_project(&domain, &engine, &device, &locale)?;
            for keyword in &keywords {
                let kw_id = storage.add_rank_keyword(&project_id, keyword, None)?;
                println!("added keyword {keyword:?} ({kw_id}) to project {domain:?}");
            }
            println!(
                "project {domain:?} ready: engine={engine}, device={device}, locale={locale}, keywords={}",
                keywords.len()
            );
        }

        RankCommands::List { domain, format, db } => {
            let storage = open_db(db.as_deref())?;
            let projects = select_projects(&storage, domain.as_deref())?;

            struct ProjectView {
                project: crawlkit_engine::rank::RankProject,
                keywords: Vec<crawlkit_engine::rank::RankKeyword>,
            }
            let mut views = Vec::new();
            for project in projects {
                let keywords = storage.list_rank_keywords(&project.id)?;
                views.push(ProjectView { project, keywords });
            }

            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(
                        &views
                            .iter()
                            .map(|v| {
                                serde_json::json!({
                                    "domain": v.project.domain,
                                    "search_engine": v.project.search_engine,
                                    "device": v.project.device,
                                    "locale": v.project.locale,
                                    "keywords": v.keywords.iter().map(|k| serde_json::json!({
                                        "keyword": k.keyword,
                                        "target_url": k.target_url,
                                        "enabled": k.enabled,
                                    })).collect::<Vec<_>>(),
                                })
                            })
                            .collect::<Vec<_>>(),
                    )?;
                    println!("{json}");
                }
                "md" => {
                    println!("| Domain | Engine | Device | Locale | Keywords |");
                    println!("|--------|--------|--------|--------|----------|");
                    for v in &views {
                        println!(
                            "| {} | {} | {} | {} | {} |",
                            v.project.domain,
                            v.project.search_engine,
                            v.project.device,
                            v.project.locale,
                            v.keywords.len()
                        );
                    }
                    for v in &views {
                        if v.keywords.is_empty() {
                            continue;
                        }
                        println!("\n**{}**\n", v.project.domain);
                        println!("| Keyword | Target | Enabled |");
                        println!("|---------|--------|---------|");
                        for k in &v.keywords {
                            println!(
                                "| {} | {} | {} |",
                                k.keyword,
                                k.target_url.as_deref().unwrap_or("(project domain)"),
                                if k.enabled { "yes" } else { "no" }
                            );
                        }
                    }
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "unsupported format: {other} (use json or md)"
                    ))
                }
            }
        }

        RankCommands::Check {
            domain,
            delay_ms,
            db,
        } => {
            let storage = open_db(db.as_deref())?;
            let projects = select_projects(&storage, domain.as_deref())?;
            if projects.is_empty() {
                return Err(anyhow::anyhow!(
                    "no rank projects configured; use `crawlkit rank add` first"
                ));
            }

            use std::sync::atomic::{AtomicUsize, Ordering};
            let total = AtomicUsize::new(0);

            for project in projects {
                let provider = build_provider(&project.search_engine)?;
                let tracker = RankTracker::new()
                    .with_device(project.device.clone())
                    .with_locale(project.locale.clone());

                let keywords: Vec<_> = storage
                    .list_rank_keywords(&project.id)?
                    .into_iter()
                    .filter(|k| k.enabled)
                    .collect();

                for keyword in keywords {
                    let target = keyword.target_url.as_deref().unwrap_or(&project.domain);
                    let mut pos = tracker
                        .check_keyword(&keyword.keyword, Some(target), provider.as_ref())
                        .await
                        .with_context(|| format!("rank check failed for {:?}", keyword.keyword))?;
                    pos.keyword_id = keyword.id.clone();
                    storage
                        .record_rank_position(&pos)
                        .context("failed to record rank position")?;
                    total.fetch_add(1, Ordering::Relaxed);
                    println!(
                        "[{}] {:?} -> {} {}",
                        project.domain,
                        keyword.keyword,
                        format_position(pos.position),
                        pos.url.as_deref().unwrap_or("not found")
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            }
            println!("checked {} keyword(s)", total.load(Ordering::Relaxed));
        }

        RankCommands::History {
            domain,
            keyword,
            days,
            format,
            db,
        } => {
            let storage = open_db(db.as_deref())?;
            let projects = select_projects(&storage, Some(&domain))?;
            let project = &projects[0];

            let keywords = storage.list_rank_keywords(&project.id)?;
            let kw = keywords
                .iter()
                .find(|k| k.keyword.eq_ignore_ascii_case(&keyword))
                .context(format!(
                    "keyword {keyword:?} not found in project {domain:?}"
                ))?;

            let history = storage.get_rank_history(&kw.id, days)?;
            if history.is_empty() {
                println!("no history for {keyword:?} in the last {days} days");
                return Ok(());
            }

            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&history)?;
                    println!("{json}");
                }
                "md" => {
                    println!(
                        "# Rank history: {} @ {} (last {days} days)\n",
                        keyword, domain
                    );
                    println!("| Checked at | Position | URL | Source |");
                    println!("|------------|----------|-----|--------|");
                    for p in &history {
                        println!(
                            "| {} | {} | {} | {} |",
                            p.checked_at.format("%Y-%m-%d %H:%M UTC"),
                            format_position(p.position),
                            p.url.as_deref().unwrap_or("-"),
                            p.source
                        );
                    }
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "unsupported format: {other} (use json or md)"
                    ))
                }
            }
        }

        RankCommands::Trend {
            domain,
            days,
            format,
            db,
        } => {
            let storage = open_db(db.as_deref())?;
            let projects = select_projects(&storage, domain.as_deref())?;
            if projects.is_empty() {
                return Err(anyhow::anyhow!(
                    "no rank projects configured; use `crawlkit rank add` first"
                ));
            }

            #[derive(Serialize)]
            struct KeywordTrend {
                domain: String,
                keyword: String,
                points: usize,
                direction: crawlkit_engine::rank::RankTrendDirection,
                best_position: Option<u16>,
                avg_position: Option<f64>,
                change: i32,
            }

            let mut trends: Vec<KeywordTrend> = Vec::new();
            for project in &projects {
                for keyword in storage.list_rank_keywords(&project.id)? {
                    let history: Vec<RankPosition> = storage.get_rank_history(&keyword.id, days)?;
                    if history.len() < 2 {
                        continue;
                    }
                    let snapshots: Vec<RankSnapshot> = history
                        .iter()
                        .map(|p| RankSnapshot {
                            keyword_id: p.keyword_id.clone(),
                            checked_at: p.checked_at,
                            position: p.position,
                            source: p.source.clone(),
                        })
                        .collect();
                    let trend = analyze_rank_trend(&snapshots).with_context(|| {
                        format!("trend analysis failed for {:?}", keyword.keyword)
                    })?;
                    trends.push(KeywordTrend {
                        domain: project.domain.clone(),
                        keyword: keyword.keyword,
                        points: trend.points,
                        direction: trend.direction,
                        best_position: trend.best_position,
                        avg_position: trend.avg_position,
                        change: trend.change,
                    });
                }
            }

            if trends.is_empty() {
                println!("not enough history for trend analysis (need >= 2 checks per keyword)");
                return Ok(());
            }

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&trends)?);
                }
                "md" => {
                    println!("# Rank trends (last {days} days)\n");
                    println!("| Domain | Keyword | Checks | Direction | Best | Avg | Change |");
                    println!("|--------|---------|--------|-----------|------|-----|--------|");
                    for t in &trends {
                        println!(
                            "| {} | {} | {} | {:?} | {} | {} | {:+} |",
                            t.domain,
                            t.keyword,
                            t.points,
                            t.direction,
                            format_position(t.best_position),
                            t.avg_position
                                .map(|v| format!("{v:.1}"))
                                .unwrap_or_else(|| "-".to_string()),
                            t.change
                        );
                    }
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "unsupported format: {other} (use json or md)"
                    ))
                }
            }
        }
    }
    Ok(())
}
