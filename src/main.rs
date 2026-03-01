pub mod config;
pub mod ingest;
pub mod markets;
pub mod metrics;
pub mod orderbook;
pub mod time;

use config::markets::MarketConfig;
use log::info;
use markets::manager::MarketManager;
use std::time::Instant;
use time::now_micros;

fn main() {
    // Initialize env_logger to see log output
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("--- Polymarket HFT Engine Demo ---");

    // Initialize market configurations dynamically
    let configs = vec![
        MarketConfig::new("BTC", "btc-updown-5m-"),
        MarketConfig::new("ETH", "eth-updown-5m-"),
        MarketConfig::new("SOL", "sol-updown-5m-"),
        MarketConfig::new("XRP", "xrp-updown-5m-"),
    ];

    let manager = MarketManager::new(configs);

    // 1. Market rotation simulation
    info!("Simulating market discovery...");
    let start_ts = now_micros();
    manager.on_market_discovered("btc-updown-5m-1772334000", start_ts);
    manager.on_market_discovered("eth-updown-5m-1772334000", start_ts);

    // 2. Orderbook delta simulation (10,000 updates)
    info!("Simulating 10,000 L2 orderbook deltas on BTC...");
    if let Some(mut btc_tracker) = manager.active_markets.get_mut("btc-updown-5m-1772334000") {
        let sim_start = Instant::now();
        let mut bid_price = 60000.0;
        let mut ask_price = 60001.0;

        for i in 1..=10_000 {
            let is_bid = i % 2 == 0;
            // Every 10th update removes a level, else adds size
            let size = if i % 10 == 0 { 0.0 } else { 0.1 };
            
            let price = if is_bid {
                bid_price -= 0.01;
                bid_price
            } else {
                ask_price += 0.01;
                ask_price
            };

            // Apply L2 tick directly
            btc_tracker.book.apply_update(is_bid, price, size, i as u64);
            btc_tracker.apply_price_update(price);

            // Simulate microstructure tick updates
            if i % 100 == 0 {
                btc_tracker.microstructure.on_trade(1.0, false);
                btc_tracker.microstructure.update_tick(now_micros(), i as u64);
            }
        }

        let elapsed = sim_start.elapsed();
        info!(
            "Processed 10,000 orderbook updates in {:?} ({:.0} ops/sec)",
            elapsed,
            10_000.0 / elapsed.as_secs_f64()
        );

        // 3. Imbalance Metrics 
        let imbalance = orderbook::imbalance::imbalance(&btc_tracker.book);
        let microprice = orderbook::imbalance::microprice(&btc_tracker.book);
        info!("Current BTC Imbalance (Top 10): {:.4}", imbalance);
        info!("Current BTC Microprice: {:?}", microprice);
    }

    // 4. Market Summary computation test (resolution)
    info!("Simulating 5-minute market resolution...");
    let end_ts = start_ts + 300_000_000; // 5 mins later
    if let Some(summary) = manager.on_market_resolved("btc-updown-5m-1772334000", end_ts) {
        info!("Generated Final Metrics Box: {:#?}", summary);
    }
}
