/// Polymarket WebSocket message types.
///
/// All numeric values from Polymarket arrive as strings (e.g., "0.56", "1234.0")
/// to preserve precision — parse them as f64 explicitly at the routing layer.
use serde::{Deserialize, Serialize};

// ─── Subscription ──────────────────────────────────────────────────────────

/// Sent to the server immediately after connecting to subscribe to token streams.
#[derive(Debug, Serialize)]
pub struct SubscribeRequest {
    pub assets_ids: Vec<String>,
    #[serde(rename = "type")]
    pub msg_type: &'static str,
    pub custom_feature_enabled: bool,
}

impl SubscribeRequest {
    pub fn market(asset_ids: Vec<String>) -> Self {
        Self {
            assets_ids: asset_ids,
            msg_type: "market",
            custom_feature_enabled: true,
        }
    }
}

/// Sent to the server to unsubscribe from token streams.
#[derive(Debug, Serialize)]
pub struct UnsubscribeRequest {
    pub assets_ids: Vec<String>,
    pub action: &'static str,
    #[serde(rename = "type")]
    pub msg_type: &'static str,
}

impl UnsubscribeRequest {
    pub fn market(asset_ids: Vec<String>) -> Self {
        Self {
            assets_ids: asset_ids,
            action: "unsubscribe",
            msg_type: "market",
        }
    }
}

// ─── Inbound message envelope ────────────────────────────────────────────────

/// Top-level message from the Polymarket market channel WebSocket.
/// We first deserialize into this to determine event_type, then branch.
#[derive(Debug, Deserialize)]
pub struct RawWsMessage {
    pub event_type: Option<String>,
    #[serde(flatten)]
    pub payload: serde_json::Value,
}

// ─── Order level ─────────────────────────────────────────────────────────────

/// A single price level as returned in book snapshots and price_change events.
/// Both `price` and `size` come as decimal strings from Polymarket.
#[derive(Debug, Deserialize, Clone)]
pub struct OrderLevel {
    pub price: String,
    pub size: String,
}

impl OrderLevel {
    pub fn price_f64(&self) -> f64 {
        self.price.parse().unwrap_or(0.0)
    }
    pub fn size_f64(&self) -> f64 {
        self.size.parse().unwrap_or(0.0)
    }
}

// ─── Book snapshot ───────────────────────────────────────────────────────────

/// Full order book snapshot — sent on subscribe and after each trade execution.
#[derive(Debug, Deserialize)]
pub struct BookSnapshot {
    pub asset_id: String,
    pub market: Option<String>,
    pub bids: Vec<OrderLevel>,
    pub asks: Vec<OrderLevel>,
    pub timestamp: Option<String>,
    pub hash: Option<String>,
}

// ─── Price change (incremental delta) ────────────────────────────────────────

/// A single level change within a `price_change` event.
#[derive(Debug, Deserialize, Clone)]
pub struct LevelChange {
    pub price: String,
    pub size: String,
    /// "BUY" = bid side, "SELL" = ask side
    pub side: String,
}

impl LevelChange {
    pub fn price_f64(&self) -> f64 {
        self.price.parse().unwrap_or(0.0)
    }
    pub fn size_f64(&self) -> f64 {
        self.size.parse().unwrap_or(0.0)
    }
    pub fn is_bid(&self) -> bool {
        self.side.eq_ignore_ascii_case("BUY")
    }
}

/// Incremental orderbook delta — one or more price levels changed.
#[derive(Debug, Deserialize)]
pub struct PriceChangeMsg {
    pub asset_id: String,
    pub changes: Vec<LevelChange>,
    pub timestamp: Option<String>,
    pub hash: Option<String>,
}

// ─── Trade ───────────────────────────────────────────────────────────────────

/// A matched trade fill on the exchange.
#[derive(Debug, Deserialize)]
pub struct LastTradePriceMsg {
    pub asset_id: String,
    pub price: String,
    pub size: Option<String>,
    pub side: Option<String>,
    pub timestamp: Option<String>,
}

impl LastTradePriceMsg {
    pub fn price_f64(&self) -> f64 {
        self.price.parse().unwrap_or(0.0)
    }
    pub fn size_f64(&self) -> f64 {
        self.size.as_deref().unwrap_or("0").parse().unwrap_or(0.0)
    }
}

// ─── Best bid/ask ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BestBidAskMsg {
    pub asset_id: String,
    pub best_bid: Option<String>,
    pub best_ask: Option<String>,
    pub timestamp: Option<String>,
}

// ─── Market lifecycle ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MarketResolvedMsg {
    pub asset_id: Option<String>,
    pub market: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewMarketMsg {
    pub asset_id: Option<String>,
    pub market: Option<String>,
    pub condition_id: Option<String>,
    pub slug: Option<String>,
}

// ─── Parsed enum ─────────────────────────────────────────────────────────────

/// Fully parsed and typed variant of an inbound Polymarket WebSocket message.
#[derive(Debug)]
pub enum PolymarketWsMsg {
    Book(BookSnapshot),
    PriceChange(PriceChangeMsg),
    LastTradePrice(LastTradePriceMsg),
    BestBidAsk(BestBidAskMsg),
    MarketResolved(MarketResolvedMsg),
    NewMarket(NewMarketMsg),
    TickSizeChange,
    Unknown(String),
}

impl PolymarketWsMsg {
    /// Parse a raw JSON text frame from Polymarket into a typed message.
    pub fn from_json(text: &str) -> Option<Self> {
        let raw: RawWsMessage = serde_json::from_str(text).ok()?;
        let event_type = raw.event_type.as_deref().unwrap_or("").to_lowercase();
        let payload = raw.payload;

        match event_type.as_str() {
            "book" => {
                let msg: BookSnapshot = serde_json::from_value(payload).ok()?;
                Some(PolymarketWsMsg::Book(msg))
            }
            "price_change" => {
                let msg: PriceChangeMsg = serde_json::from_value(payload).ok()?;
                Some(PolymarketWsMsg::PriceChange(msg))
            }
            "last_trade_price" => {
                let msg: LastTradePriceMsg = serde_json::from_value(payload).ok()?;
                Some(PolymarketWsMsg::LastTradePrice(msg))
            }
            "best_bid_ask" => {
                let msg: BestBidAskMsg = serde_json::from_value(payload).ok()?;
                Some(PolymarketWsMsg::BestBidAsk(msg))
            }
            "market_resolved" => {
                let msg: MarketResolvedMsg = serde_json::from_value(payload).ok()?;
                Some(PolymarketWsMsg::MarketResolved(msg))
            }
            "new_market" => {
                let msg: NewMarketMsg = serde_json::from_value(payload).ok()?;
                Some(PolymarketWsMsg::NewMarket(msg))
            }
            "tick_size_change" => Some(PolymarketWsMsg::TickSizeChange),
            _ => Some(PolymarketWsMsg::Unknown(event_type)),
        }
    }
}
