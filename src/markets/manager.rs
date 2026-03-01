use crate::config::markets::MarketConfig;
use crate::markets::lifecycle::ActiveMarketTracker;
use crate::metrics::summary::MarketSummary;
use dashmap::DashMap;
use log::{info, warn};
use std::sync::Arc;

/// Thread-safe manager for tracking rotating Polymarket IDs dynamically.
/// Supports both slug-based discovery (demo) and token ID registration (live).
#[derive(Clone)]
pub struct MarketManager {
    configs: Arc<Vec<MarketConfig>>,
    pub active_markets: Arc<DashMap<String, ActiveMarketTracker>>,
    /// Maps token_id → slug so the router can resolve asset_id → tracker
    token_to_slug: Arc<DashMap<String, String>>,
}

impl MarketManager {
    pub fn new(configs: Vec<MarketConfig>) -> Self {
        Self {
            configs: Arc::new(configs),
            active_markets: Arc::new(DashMap::new()),
            token_to_slug: Arc::new(DashMap::new()),
        }
    }

    /// Register a Polymarket token ID (from Gamma API) so the router can resolve
    /// asset_id → config_symbol. Creates the market tracker immediately.
    pub fn register_token_id(&self, token_id: &str, slug: &str, timestamp_us: i64) {
        self.token_to_slug
            .insert(token_id.to_string(), slug.to_string());
        // Use token_id as the key in active_markets so the router can look it up directly
        self.on_market_discovered(token_id, timestamp_us);
    }

    /// Dynamically identifies and spins up a tracking engine for a new 5m market.
    /// Key can be either a slug or a token_id if registered via `register_token_id`.
    pub fn on_market_discovered(&self, active_slug: &str, timestamp_us: i64) {
        if self.active_markets.contains_key(active_slug) {
            return;
        }

        // Check if this is a registered token_id → resolve the slug first
        let lookup_slug = if let Some(slug) = self.token_to_slug.get(active_slug) {
            slug.clone()
        } else {
            active_slug.to_string()
        };

        for config in self.configs.iter() {
            if config.matches_slug(&lookup_slug) || config.matches_slug(active_slug) {
                let tracker = ActiveMarketTracker::new(
                    config.symbol.clone(),
                    active_slug.to_string(),
                    timestamp_us,
                );
                self.active_markets.insert(active_slug.to_string(), tracker);
                info!(
                    "[Manager] Discovered & mounted new 5m market: {} mapped to {}",
                    active_slug, config.symbol
                );
                return;
            }
        }
    }

    /// Computes summary statistics and marks a market as fully resolved.
    /// Returns the generated statistical MarketSummary dataset row for QuestDB modeling.
    pub fn on_market_resolved(
        &self,
        active_slug: &str,
        end_timestamp_us: i64,
    ) -> Option<MarketSummary> {
        if let Some(mut tracker) = self.active_markets.get_mut(active_slug) {
            if tracker.is_resolved {
                return None; // Ensure idempotency
            }
            tracker.is_resolved = true;
            info!("[Manager] Market fully resolved: {}", active_slug);
            Some(tracker.finalize(end_timestamp_us))
        } else {
            warn!(
                "[Manager] Tried resolving unknown or untracked market: {}",
                active_slug
            );
            None
        }
    }

    /// Returns all currently tracked token IDs (for re-subscription after rotation).
    pub fn get_active_token_ids(&self) -> Vec<String> {
        self.token_to_slug.iter().map(|e| e.key().clone()).collect()
    }
}
