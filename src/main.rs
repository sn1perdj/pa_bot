pub mod cli;
pub mod config;
pub mod demo;
pub mod engine;
pub mod gamma;
pub mod ingest;
pub mod live;
pub mod markets;
pub mod metrics;
pub mod orderbook;
pub mod time;
pub mod ws;

use clap::Parser;
use cli::{Args, Mode};
use log::info;

#[tokio::main]
async fn main() {
    // Initialize structured logging from RUST_LOG env var (default: info)
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Args::parse();

    match args.mode {
        Mode::Live => {
            info!("=== pa_bot — Live Mode ===");
            info!("Set QUESTDB_HOST / QUESTDB_PORT to override defaults (127.0.0.1:9009)");
            live::run().await;
        }
        Mode::Demo => {
            info!("=== pa_bot — Demo Mode ===");
            demo::run();
        }
    }
}
