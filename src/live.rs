/// Live engine entry point.
///
/// Orchestrates:
/// 1. Gamma API token ID resolution
/// 2. QuestDB ILP writer startup
/// 3. Stats printer
/// 4. Polymarket WebSocket ingestion loop (with auto-reconnect)
///
/// Graceful shutdown on Ctrl+C.
use std::sync::Arc;

use log::{error, info, warn};

use tokio::sync::mpsc;
use tokio::time::{self, Duration};

use crate::config::markets::MarketConfig;
use crate::engine::router::EventRouter;
use crate::engine::stats::{EngineStats, spawn_stats_printer};
use crate::gamma;
use crate::ingest::pipeline::create_pipeline;
use crate::ingest::writer::DbWriter;
use crate::markets::manager::MarketManager;

/// Base slugs we track — each becomes a WS subscription via token ID lookup.
const BASE_SLUGS: &[&str] = &[
    "btc-updown-5m-",
    "eth-updown-5m-",
    "sol-updown-5m-",
    "xrp-updown-5m-",
];

/// Capacity of the bounded ingestion channel (handles bursts).
const CHANNEL_CAPACITY: usize = 100_000;

/// QuestDB ILP batch tuning.
const BATCH_SIZE: usize = 5_000;
const FLUSH_INTERVAL_MS: u64 = 2;

pub async fn run() {
    // ── QuestDB connection config ──────────────────────────────────────────────
    let questdb_host = std::env::var("QUESTDB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let questdb_port: u16 = std::env::var("QUESTDB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9009);

    info!("[Live] QuestDB target: {}:{}", questdb_host, questdb_port);

    // ── Gamma API: Background Polling Loop ───────────────────────────────────
    info!("[Live] Starting Gamma API background poller for 5m markets...");
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("HTTP client build failed");

    let (sub_tx, sub_rx) = mpsc::channel(16);

    // ── Market manager setup ───────────────────────────────────────────────────
    let configs = vec![
        MarketConfig::new("BTC", "btc-updown-5m-"),
        MarketConfig::new("ETH", "eth-updown-5m-"),
        MarketConfig::new("SOL", "sol-updown-5m-"),
        MarketConfig::new("XRP", "xrp-updown-5m-"),
    ];

    let manager = std::sync::Arc::new(MarketManager::new(configs));
    let manager_clone = manager.clone();

    tokio::spawn(async move {
        // Initial fetch
        let mut interval = time::interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            let resolved = gamma::fetch_rotating_token_ids(&http, BASE_SLUGS).await;

            if resolved.is_empty() {
                // Completely normal for 5m markets to briefly disappear during rotation
                continue;
            }

            let mut asset_ids: Vec<String> = Vec::new();
            let now_us = crate::time::now_micros();

            for market in resolved {
                manager_clone.register_token_id(&market.yes_token_id, &market.slug, now_us);
                asset_ids.push(market.yes_token_id.clone());

                if let Some(no_id) = &market.no_token_id {
                    manager_clone.register_token_id(no_id, &market.slug, now_us);
                    asset_ids.push(no_id.clone());
                }
            }

            // Send full list of active tokens to the WS client to diff & subscribe
            let _ = sub_tx.send(asset_ids).await;
        }
    });

    // ── Ingestion pipeline: channel + QuestDB writer thread ───────────────────
    let (producer, rx) = create_pipeline(CHANNEL_CAPACITY);

    let db_writer = match DbWriter::new(
        rx,
        &questdb_host,
        questdb_port,
        BATCH_SIZE,
        FLUSH_INTERVAL_MS,
    ) {
        Ok(w) => w,
        Err(e) => {
            warn!(
                "[Live] QuestDB writer init failed: {}. Continuing without DB writes.",
                e
            );
            // We still run the WS loop — events will be dropped at the channel level
            // Return early here for now; alternatively, use a no-op writer
            error!(
                "[Live] Cannot proceed without QuestDB connection on {}:{}.",
                questdb_host, questdb_port
            );
            error!("[Live] Set QUESTDB_HOST / QUESTDB_PORT env vars or start QuestDB.");
            return;
        }
    };

    let _writer_handle = db_writer.start();
    info!(
        "[Live] QuestDB ILP writer started → {}:{}",
        questdb_host, questdb_port
    );

    // ── Stats printer ──────────────────────────────────────────────────────────
    let stats = Arc::new(EngineStats::default());
    spawn_stats_printer(Arc::clone(&stats));

    // ── Event router ───────────────────────────────────────────────────────────
    let router = EventRouter::new(
        std::sync::Arc::clone(&manager),
        producer,
        Arc::clone(&stats),
    );

    // ── WebSocket ingestion — runs until Ctrl+C ────────────────────────────────
    info!("[Live] Starting Polymarket WebSocket client...");

    tokio::select! {
        _ = crate::ws::client::run(sub_rx, router, Arc::clone(&stats)) => {
            // WS client loop only exits on unrecoverable error
            error!("[Live] WebSocket client exited unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("[Live] Ctrl+C received — shutting down gracefully");
        }
    }

    info!("[Live] Engine stopped.");
}
