/// Tracks high-frequency state changes for a given market,
/// useful to detect anomalies, fast markets, pauses, and whale activity.
#[derive(Debug, Clone, Default)]
pub struct MicrostructureMetrics {
    pub cumulative_volume: f64,
    pub delta_volume: f64,
    pub last_tick_time: Option<i64>,       // microsecond timestamp
    pub time_since_last_tick: Option<i64>, // microseconds
    pub sequence_number: u64,
    pub market_paused: bool,
    pub liquidity_shock: bool,
    pub aggressive_trade_impact: f64,
}

impl MicrostructureMetrics {
    #[inline(always)]
    pub fn update_tick(&mut self, current_time_us: i64, seq_num: u64) {
        if let Some(last) = self.last_tick_time {
            let elapsed = current_time_us.saturating_sub(last);
            self.time_since_last_tick = Some(elapsed);

            // Basic heuristic for a Polymarket pause: no tick for > 15 seconds (15_000_000 us)
            self.market_paused = elapsed > 15_000_000;
        }
        self.last_tick_time = Some(current_time_us);
        self.sequence_number = seq_num;

        // Reset per-tick triggers that shouldn't leak between events
        self.liquidity_shock = false;
        self.delta_volume = 0.0;
        self.aggressive_trade_impact = 0.0;
    }

    /// Records a new trade happening at the current tick sequence
    #[inline(always)]
    pub fn on_trade(&mut self, size: f64, is_whale: bool) {
        self.cumulative_volume += size;
        self.delta_volume += size;

        if is_whale {
            self.aggressive_trade_impact += size;
        }
    }

    /// Flag a sudden disappearance of liquidity levels this tick
    #[inline(always)]
    pub fn detect_liquidity_shock(&mut self, removed_levels_count: usize) {
        if removed_levels_count > 5 {
            self.liquidity_shock = true;
        }
    }
}
