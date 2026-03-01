use clap::{Parser, ValueEnum};

/// Polymarket HFT Engine
#[derive(Parser, Debug)]
#[command(name = "pa_bot", about = "Polymarket HFT Engine")]
pub struct Args {
    /// Runtime mode: 'live' connects to Polymarket WebSocket + QuestDB.
    /// 'demo' runs an offline simulation.
    #[arg(long, default_value = "live")]
    pub mode: Mode,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    /// Live WebSocket ingestion from Polymarket + QuestDB write
    Live,
    /// Offline simulation / demo (no network, no DB)
    Demo,
}
