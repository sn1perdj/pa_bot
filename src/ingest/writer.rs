use crossbeam::channel::{Receiver, RecvTimeoutError};
use log::{error, info};
use questdb::ingress::{Buffer, Sender, TimestampMicros};
use std::thread;
use std::time::{Duration, Instant};

use super::{FeatureEvent, IngestEvent, OrderbookEvent, TradeEvent};

pub struct DbWriter {
    rx: Receiver<IngestEvent>,
    sender: Sender,
    buffer: Buffer,
    batch_size_limit: usize,
    flush_interval: Duration,
}

impl DbWriter {
    /// Initializes a dedicated QuestDB ILP writer.
    /// `batch_size_limit`: Recommended 5,000 for high-frequency batched flush.
    /// `flush_interval_ms`: Recommended 2ms or less for latency-sensitive operation.
    pub fn new(
        rx: Receiver<IngestEvent>,
        host: &str,
        port: u16,
        batch_size_limit: usize,
        flush_interval_ms: u64,
    ) -> questdb::Result<Self> {
        let sender = questdb::ingress::SenderBuilder::new(host, port).connect()?;
        
        Ok(Self {
            rx,
            sender,
            buffer: Buffer::new(),
            batch_size_limit,
            flush_interval: Duration::from_millis(flush_interval_ms),
        })
    }

    /// Spawns the dedicated writer thread and returns the JoinHandle.
    pub fn start(mut self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            info!("QuestDB ILP writer thread started.");
            self.run_loop();
            info!("QuestDB ILP writer thread cleanly exited.");
        })
    }

    fn run_loop(&mut self) {
        let mut batch_count = 0;
        let mut last_flush = Instant::now();

        loop {
            // Determine time left until the forced flush
            let now = Instant::now();
            let elapsed = now.saturating_duration_since(last_flush);
            let timeout = if elapsed >= self.flush_interval {
                Duration::from_nanos(0)
            } else {
                self.flush_interval - elapsed
            };

            match self.rx.recv_timeout(timeout) {
                Ok(event) => {
                    self.process_event(event);
                    batch_count += 1;

                    if batch_count >= self.batch_size_limit {
                        self.flush_batch(batch_count);
                        batch_count = 0;
                        last_flush = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Flush interval reached but channel didn't yield an event for a bit
                    if batch_count > 0 {
                        self.flush_batch(batch_count);
                        batch_count = 0;
                        last_flush = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    info!(
                        "Ingestion pipeline shut down. Flushing remaining {} events...",
                        batch_count
                    );
                    if batch_count > 0 {
                        self.flush_batch(batch_count);
                    }
                    break;
                }
            }
        }
    }

    #[inline(always)]
    fn process_event(&mut self, event: IngestEvent) {
        let res = match event {
            IngestEvent::Trade(t) => self.write_trade(t),
            IngestEvent::Orderbook(o) => self.write_orderbook(o),
            IngestEvent::Feature(f) => self.write_feature(f),
        };

        if let Err(e) = res {
            error!("Failed to serialize event line: {}", e);
        }
    }

    #[inline(always)]
    fn write_trade(&mut self, t: TradeEvent) -> questdb::Result<()> {
        self.buffer
            .table("trades")?
            .symbol("symbol", &t.symbol)?
            .symbol("side", &t.side)?
            .column_f64("price", t.price)?
            .column_f64("size", t.size)?
            .column_i64("exchange_ts", t.exchange_ts)?
            .at(TimestampMicros::new(t.local_ts))
    }

    #[inline(always)]
    fn write_orderbook(&mut self, o: OrderbookEvent) -> questdb::Result<()> {
        self.buffer
            .table("orderbook")?
            .symbol("symbol", &o.symbol)?
            .column_f64("bid_price", o.bid_price)?
            .column_f64("ask_price", o.ask_price)?
            .column_i64("exchange_ts", o.exchange_ts)?
            .at(TimestampMicros::new(o.local_ts))
    }

    #[inline(always)]
    fn write_feature(&mut self, f: FeatureEvent) -> questdb::Result<()> {
        self.buffer
            .table("features")?
            .symbol("symbol", &f.symbol)?
            .symbol("feature_name", &f.feature_name)?
            .column_f64("feature_value", f.feature_value)?
            .at(TimestampMicros::new(f.local_ts))
    }

    fn flush_batch(&mut self, count: usize) {
        if let Err(e) = self.sender.flush(&mut self.buffer) {
            error!("Failed to flush {} events to QuestDB: {}. Buffer cleared to prevent staleness.", count, e);
            // Non-panic recovery: Clear the buffer to prevent memory leakage
            // and staleness dropping the failed batch.
            self.buffer.clear();
        } else {
            info!("Flushed batch of {} events to QuestDB", count);
        }
        // If successful, `sender.flush` automatically clears the buffer.
    }
}
