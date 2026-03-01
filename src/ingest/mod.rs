pub mod pipeline;
pub mod writer;

// You can use the TimestampedEvent from the time module, or keep events raw here
// and wrap them as needed. For now, we define standard domain events.

#[derive(Debug, Clone)]
pub struct TradeEvent {
    pub symbol: String,
    pub price: f64,
    pub size: f64,
    pub side: String,
    pub exchange_ts: i64,
    pub local_ts: i64,
}

#[derive(Debug, Clone)]
pub struct OrderbookEvent {
    pub symbol: String,
    pub bid_price: f64,
    pub ask_price: f64,
    pub exchange_ts: i64,
    pub local_ts: i64,
}

#[derive(Debug, Clone)]
pub struct FeatureEvent {
    pub symbol: String,
    pub feature_name: String,
    pub feature_value: f64,
    pub local_ts: i64,
}

#[derive(Debug, Clone)]
pub enum IngestEvent {
    Trade(TradeEvent),
    Orderbook(OrderbookEvent),
    Feature(FeatureEvent),
}
