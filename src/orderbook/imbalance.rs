use super::book::OrderBook;

/// Computes the orderbook imbalance based on the top 10 levels.
/// Formula: (bid_vol - ask_vol) / (bid_vol + ask_vol)
#[inline(always)]
pub fn imbalance(book: &OrderBook) -> f64 {
    let mut bid_vol = 0.0;
    let mut ask_vol = 0.0;
    
    // Sum top 10 bids (iterating reversed for descending order)
    for (_, vol) in book.bids.iter().rev().take(10) {
        bid_vol += vol;
    }
    
    // Sum top 10 asks (iterating normally for ascending order)
    for (_, vol) in book.asks.iter().take(10) {
        ask_vol += vol;
    }
    
    let total_vol = bid_vol + ask_vol;
    if total_vol > 0.0 {
        (bid_vol - ask_vol) / total_vol
    } else {
        0.0
    }
}

/// Helper to get the highest bid (best_bid)
#[inline(always)]
pub fn best_bid(book: &OrderBook) -> Option<f64> {
    book.bids.iter().next_back().map(|(price, _)| price.into_inner())
}

/// Helper to get the lowest ask (best_ask)
#[inline(always)]
pub fn best_ask(book: &OrderBook) -> Option<f64> {
    book.asks.iter().next().map(|(price, _)| price.into_inner())
}

/// Helper to compute the spread
#[inline(always)]
pub fn spread(book: &OrderBook) -> Option<f64> {
    match (best_bid(book), best_ask(book)) {
        (Some(b), Some(a)) => Some(a - b),
        _ => None,
    }
}

/// Helper to compute the mid price
#[inline(always)]
pub fn mid_price(book: &OrderBook) -> Option<f64> {
    match (best_bid(book), best_ask(book)) {
        (Some(b), Some(a)) => Some((a + b) / 2.0),
        _ => None,
    }
}

/// Helper to calculate microprice based on top level only
/// Formula: (best_bid * ask_vol + best_ask * bid_vol) / (bid_vol + ask_vol)
#[inline(always)]
pub fn microprice(book: &OrderBook) -> Option<f64> {
    let top_bid = book.bids.iter().next_back();
    let top_ask = book.asks.iter().next();

    match (top_bid, top_ask) {
        (Some((b_price, b_vol)), Some((a_price, a_vol))) => {
            let total_vol = b_vol + a_vol;
            if total_vol > 0.0 {
                Some((b_price.into_inner() * a_vol + a_price.into_inner() * b_vol) / total_vol)
            } else {
                mid_price(book)
            }
        },
        _ => mid_price(book),
    }
}
