use crate::config::markets::MarketConfig;
use crate::markets::lifecycle::ActiveMarketTracker;
use crate::metrics::summary::MarketSummary;
use dashmap::DashMap;
use log::{info, warn};
use std::sync::Arc;

/// Thread-safe manager for tracking rotating Polymarket IDs dynamically.
#[derive(Clone)]
pub struct MarketManager {
    configs: Arc<Vec<MarketConfig>>,
    pub active_markets: Arc<DashMap<String, ActiveMarketTracker>>,
}

impl MarketManager {
    pub fn new(configs: Vec<MarketConfig>) -> Self {
        Self {
            configs: Arc::new(configs),
            active_markets: Arc::new(DashMap::new()),
        }
    }

    /// Dynamically identifies and spins up a tracking engine for a new 5m market
    pub fn on_market_discovered(&self, active_slug: &str, timestamp_us: i64) {
        if !self.active_markets.contains_key(active_slug) {
            for config in self.configs.iter() {
                if config.matches_slug(active_slug) {
                    let tracker = ActiveMarketTracker::new(
                        config.symbol.clone(),
                        active_slug.to_string(),
                        timestamp_us,
                    );
                    self.active_markets.insert(active_slug.to_string(), tracker);
                    info!("Discovered & mounted new 5m market: {} mapped to {}", active_slug, config.symbol);
                    break;
                }
            }
        }
    }

    /// Computes summary statistics and marks a market as fully resolved.
    /// Returns the generated statistical MarketSummary dataset row for QuestDB modeling
    pub fn on_market_resolved(&self, active_slug: &str, end_timestamp_us: i64) -> Option<MarketSummary> {
        if let Some(mut tracker) = self.active_markets.get_mut(active_slug) {
            if tracker.is_resolved {
                return None; // Ensure idempotency
            }
            tracker.is_resolved = true;
            info!("Market fully resolved: {}", active_slug);
            Some(tracker.finalize(end_timestamp_us))
        } else {
            warn!("Tried resolving unknown or untracked market: {}", active_slug);
            None
        }
    }
}
