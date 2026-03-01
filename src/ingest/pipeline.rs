use crossbeam::channel::{bounded, Receiver, Sender};
use log::warn;
use super::IngestEvent;

/// Decoupled ingestion producer meant to be used inside the WebSocket hot path.
#[derive(Clone)]
pub struct IngestionProducer {
    tx: Sender<IngestEvent>,
}

impl IngestionProducer {
    /// Non-blocking push for HFT event submission without ever waiting on the DB.
    #[inline(always)]
    pub fn push(&self, event: IngestEvent) {
        if let Err(e) = self.tx.try_send(event) {
            match e {
                crossbeam::channel::TrySendError::Full(_) => {
                    warn!("Ingestion channel full! Dropping event to maintain WebSocket throughput.");
                }
                crossbeam::channel::TrySendError::Disconnected(_) => {
                    warn!("Ingestion channel disconnected! DB writer thread is likely down.");
                }
            }
        }
    }
}

/// Creates a high-performance ingestion channel pipeline.
/// - The `Producer` should be moved to the WebSocket receive loop.
/// - The `Receiver` should be moved to the Dedicated DB Writer thread.
/// 
/// `capacity`: 100,000 to handle bursts without blocking.
pub fn create_pipeline(capacity: usize) -> (IngestionProducer, Receiver<IngestEvent>) {
    let (tx, rx) = bounded(capacity);
    (IngestionProducer { tx }, rx)
}
