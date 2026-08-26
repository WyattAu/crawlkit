use crate::http::HttpClient;
use crate::storage_trait::StorageBackend;
use crate::queue::QueueEntry;
use std::sync::Arc;
use std::time::Duration;

/// Freshness classification for incremental crawls.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Freshness {
    /// First time this URL has been seen in an incremental crawl.
    New,
    /// Previously seen; content changed since the last crawl.
    Modified,
    /// Incremental crawling disabled — no classification performed.
    Unconditional,
}

/// The outcome of a single worker fetch.
///
/// Replaces the former magic-string encoding of 304 responses and carries
/// the freshness classification needed for incremental-crawl statistics.
pub(crate) enum FetchOutcome {
    /// Page fetched successfully.
    Fetched {
        result: crate::FetchResult,
        freshness: Freshness,
    },
    /// Server answered 304 Not Modified. `page_id` identifies the stored page
    /// whose `fetched_at` timestamp should be refreshed.
    NotModified { page_id: Option<String> },
    /// Fetch failed after retries.
    Failed(crate::CrawlError),
}

/// A completed fetch, as delivered from a worker task to the main loop.
pub(crate) struct FetchedPage {
    pub(crate) entry: QueueEntry,
    pub(crate) robots_raw: String,
    pub(crate) fetch_time: Duration,
    pub(crate) outcome: FetchOutcome,
}

/// Execute a single fetch, applying conditional-request logic when the crawl
/// is incremental. Owns all state needed by the worker task.
pub(crate) async fn execute_fetch(
    client: Arc<HttpClient>,
    storage: Arc<dyn StorageBackend>,
    crawl_id: String,
    entry: QueueEntry,
    incremental: bool,
    force: bool,
) -> FetchOutcome {
    if !incremental || force {
        return match client.fetch(&entry.url).await {
            Ok(result) => FetchOutcome::Fetched {
                result,
                freshness: Freshness::Unconditional,
            },
            Err(e) => FetchOutcome::Failed(e),
        };
    }

    // Look up cached validators on the blocking pool so worker threads never
    // stall on SQLite reads.
    let url_string = entry.url.to_string();
    let (previous, cross_previous) = tokio::task::spawn_blocking(move || {
        let previous = storage
            .get_page_conditional(&crawl_id, &url_string)
            .ok()
            .flatten();
        let cross_previous = storage.get_latest_conditional(&url_string).ok().flatten();
        (previous, cross_previous)
    })
    .await
    .unwrap_or((None, None));

    // Prefer the same-crawl record (it carries the page_id needed for 304
    // updates); fall back to the cross-crawl record for headers only.
    let (existing_etag, existing_lm) = if let Some((_, ref etag, ref lm)) = previous {
        (etag.as_deref(), lm.as_deref())
    } else if let Some((ref etag, ref lm)) = cross_previous {
        (etag.as_deref(), lm.as_deref())
    } else {
        (None, None)
    };

    match client
        .fetch_conditional(&entry.url, existing_etag, existing_lm)
        .await
    {
        Ok(r) if r.status_code == 304 => FetchOutcome::NotModified {
            page_id: previous.map(|(id, _, _)| id),
        },
        Ok(r) => FetchOutcome::Fetched {
            result: r,
            freshness: if previous.is_some() {
                Freshness::Modified
            } else {
                Freshness::New
            },
        },
        Err(e) => FetchOutcome::Failed(e),
    }
}
