/// Live event router — bridges the Polymarket WebSocket message stream to the engine.
///
/// Owns:
/// - `MarketManager` for lifecycle tracking and feature generation
/// - `IngestionProducer` for non-blocking push to the QuestDB writer
/// - `Arc<EngineStats>` for lock-free telemetry
///
/// Hot path: called on every WebSocket frame. No blocking ops, no Mutex.
use std::sync::Arc;

use log::{debug, info};

use crate::engine::stats::EngineStats;
use crate::ingest::pipeline::IngestionProducer;
use crate::ingest::{FeatureEvent, IngestEvent, OrderbookEvent, TradeEvent};
use crate::markets::manager::MarketManager;
use crate::orderbook::imbalance;
use crate::time::now_micros;
use crate::ws::messages::{BookSnapshot, LastTradePriceMsg, PolymarketWsMsg, PriceChangeMsg};

pub struct EventRouter {
    manager: Arc<MarketManager>,
    producer: IngestionProducer,
    stats: Arc<EngineStats>,
}

impl EventRouter {
    pub fn new(
        manager: Arc<MarketManager>,
        producer: IngestionProducer,
        stats: Arc<EngineStats>,
    ) -> Self {
        Self {
            manager,
            producer,
            stats,
        }
    }

    /// Dispatch a parsed Polymarket message to the appropriate handler.
    /// Called directly from the WS receive loop — must never block.
    #[inline(always)]
    pub fn dispatch(&mut self, msg: PolymarketWsMsg) {
        match msg {
            PolymarketWsMsg::Book(snap) => self.on_book_snapshot(snap),
            PolymarketWsMsg::PriceChange(delta) => self.on_price_change(delta),
            PolymarketWsMsg::LastTradePrice(trade) => self.on_trade(trade),
            PolymarketWsMsg::MarketResolved(r) => {
                let slug = r.market.or(r.asset_id).unwrap_or_default();
                info!("[Router] Market resolved event received for: {}", slug);
                let now = now_micros();
                if let Some(summary) = self.manager.on_market_resolved(&slug, now) {
                    info!("[Router] Market summary: {:#?}", summary);
                }
            }
            PolymarketWsMsg::NewMarket(m) => {
                let slug = m.slug.or(m.condition_id).unwrap_or_default();
                if !slug.is_empty() {
                    info!("[Router] New market event: {}", slug);
                    self.manager.on_market_discovered(&slug, now_micros());
                }
            }
            PolymarketWsMsg::BestBidAsk(_) => {
                // Best bid/ask is informational only — the book events are authoritative
                debug!("[Router] Received best_bid_ask (skipped — book is authoritative)");
            }
            PolymarketWsMsg::TickSizeChange => {
                debug!("[Router] Received tick_size_change");
            }
            PolymarketWsMsg::Unknown(t) => {
                debug!("[Router] Unknown event_type: {}", t);
            }
        }
    }

    /// Handle a full book snapshot — resets the L2 book and emits best bid/ask.
    fn on_book_snapshot(&mut self, snap: BookSnapshot) {
        let local_ts = now_micros();
        let asset_id = &snap.asset_id;

        // Ensure market is tracked
        self.manager.on_market_discovered(asset_id, local_ts);

        if let Some(mut tracker) = self.manager.active_markets.get_mut(asset_id) {
            // Reset the book to apply full snapshot
            tracker.book.reset();

            let mut seq = tracker.book.sequence_number;
            for level in &snap.bids {
                seq += 1;
                tracker
                    .book
                    .apply_update(true, level.price_f64(), level.size_f64(), seq);
            }
            for level in &snap.asks {
                seq += 1;
                tracker
                    .book
                    .apply_update(false, level.price_f64(), level.size_f64(), seq);
            }
            tracker.book.sequence_number = seq;

            self.emit_orderbook_event(
                &tracker.config_symbol.clone(),
                asset_id,
                local_ts,
                &tracker.book,
            );
            self.stats.inc_book_updates();
        }
    }

    /// Handle an incremental price_change delta — applies each changed level.
    fn on_price_change(&mut self, delta: PriceChangeMsg) {
        let local_ts = now_micros();
        let asset_id = &delta.asset_id.clone();

        self.manager.on_market_discovered(asset_id, local_ts);

        if let Some(mut tracker) = self.manager.active_markets.get_mut(asset_id) {
            let removed_count = delta.changes.iter().filter(|c| c.size_f64() == 0.0).count();

            for change in &delta.changes {
                let seq = tracker.book.sequence_number + 1;
                tracker.book.apply_update(
                    change.is_bid(),
                    change.price_f64(),
                    change.size_f64(),
                    seq,
                );

                // Track the mid price for excursion analysis
                if let Some(mid) = imbalance::mid_price(&tracker.book) {
                    tracker.apply_price_update(mid);
                }
            }

            tracker.microstructure.detect_liquidity_shock(removed_count);
            let final_seq = tracker.book.sequence_number;
            tracker.microstructure.update_tick(local_ts, final_seq);

            self.emit_orderbook_event(
                &tracker.config_symbol.clone(),
                asset_id,
                local_ts,
                &tracker.book,
            );
            self.emit_feature_event(
                &tracker.config_symbol.clone(),
                asset_id,
                local_ts,
                &tracker.book,
            );
            self.stats.inc_book_updates();
            self.stats.inc_feature_updates();
        }
    }

    /// Handle a last_trade_price event — emits a TradeEvent to QuestDB.
    fn on_trade(&mut self, trade: LastTradePriceMsg) {
        let local_ts = now_micros();
        let asset_id = &trade.asset_id.clone();

        // Map asset_id → symbol via manager
        let symbol = self
            .manager
            .active_markets
            .get(asset_id)
            .map(|t| t.config_symbol.clone())
            .unwrap_or_else(|| asset_id.clone());

        // Update microstructure volume tracking
        if let Some(mut tracker) = self.manager.active_markets.get_mut(asset_id) {
            tracker
                .microstructure
                .on_trade(trade.size_f64(), trade.size_f64() > 1000.0);
            tracker.apply_price_update(trade.price_f64());
        }

        let side = trade.side.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        self.producer.push(IngestEvent::Trade(TradeEvent {
            symbol,
            price: trade.price_f64(),
            size: trade.size_f64(),
            side,
            exchange_ts: 0, // Polymarket doesn't provide exchange_ts on WS trades
            local_ts,
        }));

        self.stats.inc_trades();
    }

    /// Emit an orderbook top-of-book event (best bid/ask) to QuestDB.
    #[inline(always)]
    fn emit_orderbook_event(
        &self,
        symbol: &str,
        asset_id: &str,
        local_ts: i64,
        book: &crate::orderbook::book::OrderBook,
    ) {
        let bid = imbalance::best_bid(book).unwrap_or(0.0);
        let ask = imbalance::best_ask(book).unwrap_or(0.0);

        if bid > 0.0 || ask > 0.0 {
            self.producer.push(IngestEvent::Orderbook(OrderbookEvent {
                symbol: symbol.to_string(),
                bid_price: bid,
                ask_price: ask,
                exchange_ts: 0,
                local_ts,
            }));
            let _ = asset_id; // suppress lint
        }
    }

    /// Emit computed microstructure features to QuestDB.
    #[inline(always)]
    fn emit_feature_event(
        &self,
        symbol: &str,
        asset_id: &str,
        local_ts: i64,
        book: &crate::orderbook::book::OrderBook,
    ) {
        let imbal = imbalance::imbalance(book);
        let _ = (asset_id, book); // suppress lint

        self.producer.push(IngestEvent::Feature(FeatureEvent {
            symbol: symbol.to_string(),
            feature_name: "imbalance".to_string(),
            feature_value: imbal,
            local_ts,
        }));

        if let Some(mp) = imbalance::microprice(book) {
            self.producer.push(IngestEvent::Feature(FeatureEvent {
                symbol: symbol.to_string(),
                feature_name: "microprice".to_string(),
                feature_value: mp,
                local_ts,
            }));
        }
    }
}
