use log::info;
use std::sync::Arc;
/// Performance statistics counters for the live ingestion pipeline.
/// Uses atomic integers for lock-free updates from the WS hot path.
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time;

#[derive(Default)]
pub struct EngineStats {
    pub trades: AtomicU64,
    pub book_updates: AtomicU64,
    pub feature_updates: AtomicU64,
    pub parse_errors: AtomicU64,
}

impl EngineStats {
    pub fn inc_trades(&self) {
        self.trades.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_book_updates(&self) {
        self.book_updates.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_feature_updates(&self) {
        self.feature_updates.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_parse_errors(&self) {
        self.parse_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (u64, u64, u64, u64) {
        (
            self.trades.load(Ordering::Relaxed),
            self.book_updates.load(Ordering::Relaxed),
            self.feature_updates.load(Ordering::Relaxed),
            self.parse_errors.load(Ordering::Relaxed),
        )
    }
}

/// Spawns a background task that prints engine stats every 5 seconds.
pub fn spawn_stats_printer(stats: Arc<EngineStats>) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let (trades, books, features, errors) = stats.snapshot();
            info!(
                "[STATS] trades={} book_updates={} feature_updates={} parse_errors={}",
                trades, books, features, errors
            );
        }
    });
}
