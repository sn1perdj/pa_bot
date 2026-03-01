/// A single summary record flushed when a 5-minute market fully resolves or expires.
/// Helps form the dataset for researching optimal entry and exit points.
#[derive(Debug, Clone, Default)]
pub struct MarketSummary {
    pub symbol: String,             // e.g., "BTC"
    pub active_slug: String,        // e.g., "btc-updown-5m-1772334000"
    
    // Final book metrics
    pub final_best_bid: Option<f64>,
    pub final_best_ask: Option<f64>,
    pub final_mid_price: Option<f64>,
    
    // Volume & Volatility
    pub total_volume: f64,
    pub volatility_estimate: f64,
    
    // Excursion analysis
    pub max_adverse_excursion: f64, // MAE
    pub max_favorable_excursion: f64, // MFE
    
    // Time-based indicators
    pub time_to_first_spike_us: Option<i64>,
    pub time_to_max_imbalance_us: Option<i64>,
    pub lifetime_duration_us: i64,
}
