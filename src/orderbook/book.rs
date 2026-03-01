use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

/// High-performance L2 OrderBook based on BTreeMap.
/// `f64` values must be wrapped in `OrderedFloat` for total ordering.
#[derive(Debug, Clone, Default)]
pub struct OrderBook {
    // Stored in standard ascending order. We iterate via `.rev()` to get highest bid.
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    // Stored in standard ascending order. Ascending iteration naturally yields lowest ask.
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    /// Tracks the update sequence number to safely ignore out-of-order deltas
    pub sequence_number: u64,
}

impl OrderBook {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles an incremental delta update natively.
    /// If `size == 0.0`, the level is removed from the book.
    /// Returns `true` if the update was applied, or `false` if it was out-of-order and rejected.
    #[inline(always)]
    pub fn apply_update(&mut self, is_bid: bool, price: f64, size: f64, seq_num: u64) -> bool {
        if seq_num <= self.sequence_number {
            // Reject strictly out-of-order updates gracefully
            return false;
        }
        self.sequence_number = seq_num;

        let key = OrderedFloat(price);
        
        if is_bid {
            if size == 0.0 {
                self.bids.remove(&key);
            } else {
                self.bids.insert(key, size);
            }
        } else {
            if size == 0.0 {
                self.asks.remove(&key);
            } else {
                self.asks.insert(key, size);
            }
        }
        
        true
    }
}
