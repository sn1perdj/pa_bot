/// Configuration for tracking an aggregate series of rotating polymarket IDs
#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub symbol: String,      // e.g., "BTC"
    pub base_slug: String,   // e.g., "btc-updown-5m-"
}

impl MarketConfig {
    pub fn new(symbol: &str, base_slug: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            base_slug: base_slug.to_string(),
        }
    }

    /// Checks if a dynamically parsed active market ID matches this target.
    #[inline]
    pub fn matches_slug(&self, active_slug: &str) -> bool {
        active_slug.starts_with(&self.base_slug)
    }
}
