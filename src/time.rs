use std::time::{SystemTime, UNIX_EPOCH};

/// Captures the current local timestamp in microseconds since the UNIX epoch.
/// Optimized for low-latency usage; avoids unnecessary allocations.
#[inline(always)]
pub fn now_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time went backwards")
        .as_micros() as i64
}

/// Normalizes an exchange timestamp to microseconds.
/// Detects if the timestamp is in milliseconds and converts it if necessary.
#[inline(always)]
pub fn normalize_exchange_ts(raw_ts: i64) -> i64 {
    // If the timestamp is less than 10^13 (approx. year 2286 in ms),
    // it's highly likely to be in milliseconds rather than microseconds.
    if raw_ts < 10_000_000_000_000 {
        raw_ts * 1000
    } else {
        raw_ts
    }
}

/// Helper function to adapt a microsecond timestamp for QuestDB compatibility.
///
/// Note: QuestDB expects timestamps as UNIX epoch offsets. By default, its
/// ILP (InfluxDB Line Protocol) over TCP uses nanoseconds, but if you define
/// a column as a designated timestamp, it operates in microseconds.
/// Passing microseconds directly is optimal when using the `questdb-rs` ILP client
/// as long as the semantics match your QuestDB table definition.
#[inline(always)]
pub fn micros_to_questdb_timestamp(micros: i64) -> i64 {
    // Simply returns the micros for use directly in a designated QuestDB microsecond timestamp column.
    // If nanoseconds were strictly required by a specific ILP client config, one could do `micros * 1000`.
    micros
}

/// A struct that groups a payload with its exchange and local microsecond timestamps.
/// Built as a generic container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampedEvent<T> {
    pub payload: T,
    pub exchange_ts: i64,
    pub local_ts: i64,
}

impl<T> TimestampedEvent<T> {
    /// Constructs a new TimestampedEvent by normalizing the raw exchange timestamp
    /// and capturing the very moment this call happens for `local_ts`.
    #[inline(always)]
    pub fn new(payload: T, raw_exchange_ts: i64) -> Self {
        Self {
            payload,
            exchange_ts: normalize_exchange_ts(raw_exchange_ts),
            local_ts: now_micros(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_micros() {
        let t1 = now_micros();
        let t2 = now_micros();
        assert!(t1 > 0);
        assert!(t2 >= t1);
    }

    #[test]
    fn test_normalize_exchange_ts_ms() {
        // Assume milliseconds: approx 2024
        let ms_val = 1_700_000_000_000;
        assert_eq!(normalize_exchange_ts(ms_val), 1_700_000_000_000_000);
    }

    #[test]
    fn test_normalize_exchange_ts_us() {
        // Already microseconds
        let us_val = 1_700_000_000_000_000;
        assert_eq!(normalize_exchange_ts(us_val), 1_700_000_000_000_000);
    }
}
