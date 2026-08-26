use std::sync::atomic::{AtomicUsize, Ordering};

/// Relaxed-ordering counter helpers. Counters are statistics only; they never
/// gate correctness, so `Relaxed` ordering is sufficient and cheapest.
pub(crate) fn bump(counter: &AtomicUsize) {
    counter.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn bump_by(counter: &AtomicUsize, n: usize) {
    counter.fetch_add(n, Ordering::Relaxed);
}

pub(crate) fn current(counter: &AtomicUsize) -> usize {
    counter.load(Ordering::Relaxed)
}

/// Interior-mutable crawl counters shared across the fetch pipeline.
#[derive(Default)]
pub(crate) struct CrawlCounters {
    pub(crate) pages_crawled: AtomicUsize,
    pub(crate) pages_stored: AtomicUsize,
    pub(crate) issues_found: AtomicUsize,
    pub(crate) skipped_external: AtomicUsize,
    pub(crate) skipped_robots: AtomicUsize,
    pub(crate) skipped_duplicate: AtomicUsize,
    pub(crate) pages_unchanged: AtomicUsize,
    pub(crate) pages_modified: AtomicUsize,
    pub(crate) pages_new: AtomicUsize,
}

/// Plain-value snapshot of [`CrawlCounters`] for reporting.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CounterSnapshot {
    pub(crate) pages_crawled: usize,
    pub(crate) pages_stored: usize,
    pub(crate) issues_found: usize,
    pub(crate) skipped_external: usize,
    pub(crate) skipped_robots: usize,
    pub(crate) skipped_duplicate: usize,
    pub(crate) pages_unchanged: usize,
    pub(crate) pages_modified: usize,
    pub(crate) pages_new: usize,
}

impl CrawlCounters {
    pub(crate) fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            pages_crawled: current(&self.pages_crawled),
            pages_stored: current(&self.pages_stored),
            issues_found: current(&self.issues_found),
            skipped_external: current(&self.skipped_external),
            skipped_robots: current(&self.skipped_robots),
            skipped_duplicate: current(&self.skipped_duplicate),
            pages_unchanged: current(&self.pages_unchanged),
            pages_modified: current(&self.pages_modified),
            pages_new: current(&self.pages_new),
        }
    }
}
