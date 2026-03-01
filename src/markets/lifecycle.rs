use crate::metrics::{microstructure::MicrostructureMetrics, summary::MarketSummary};
use crate::orderbook::{book::OrderBook, imbalance::{best_ask, best_bid, mid_price}};

/// Maintains the internal state of a single actively tracked 5-minute market.
#[derive(Debug, Clone, Default)]
pub struct ActiveMarketTracker {
    pub config_symbol: String,
    pub active_slug: String,
    
    // Core engine state tracking
    pub book: OrderBook,
    pub microstructure: MicrostructureMetrics,
    
    // Lifecycle metrics
    pub start_time_us: i64,
    pub is_resolved: bool,
    
    // Tracking excursion limits
    pub initial_mid_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_price: Option<f64>,
}

impl ActiveMarketTracker {
    pub fn new(symbol: String, active_slug: String, start_time_us: i64) -> Self {
        Self {
            config_symbol: symbol,
            active_slug,
            book: OrderBook::new(),
            microstructure: MicrostructureMetrics::default(),
            start_time_us,
            is_resolved: false,
            initial_mid_price: None,
            max_price: None,
            min_price: None,
        }
    }
    
    /// Called when the market resolves. Generates the final metrics block.
    pub fn finalize(&self, end_time_us: i64) -> MarketSummary {
        let max_adverse = match (self.initial_mid_price, self.min_price) {
            (Some(initial), Some(min)) => ((initial - min) / initial).abs(),
            _ => 0.0,
        };
        
        let max_favorable = match (self.initial_mid_price, self.max_price) {
            (Some(initial), Some(max)) => ((max - initial) / initial).abs(),
            _ => 0.0,
        };

        MarketSummary {
            symbol: self.config_symbol.clone(),
            active_slug: self.active_slug.clone(),
            final_best_bid: best_bid(&self.book),
            final_best_ask: best_ask(&self.book),
            final_mid_price: mid_price(&self.book),
            total_volume: self.microstructure.cumulative_volume,
            volatility_estimate: max_adverse + max_favorable, // Simple proxy heuristic for volatility
            max_adverse_excursion: max_adverse,
            max_favorable_excursion: max_favorable,
            time_to_first_spike_us: None, // Used in advanced modeling later
            time_to_max_imbalance_us: None, // Used in advanced modeling later
            lifetime_duration_us: end_time_us.saturating_sub(self.start_time_us),
        }
    }

    /// Process a new orderbook tick (trade or delta) and apply state bounds safely
    #[inline(always)]
    pub fn apply_price_update(&mut self, price: f64) {
        if self.initial_mid_price.is_none() {
            self.initial_mid_price = Some(price);
        }
        
        // Track the boundaries mapped to max adverse or max favorable excursion
        match self.max_price {
            Some(max) if price > max => self.max_price = Some(price),
            None => self.max_price = Some(price),
            _ => {}
        }

        match self.min_price {
            Some(min) if price < min => self.min_price = Some(price),
            None => self.min_price = Some(price),
            _ => {}
        }
    }
}
