use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let now = chrono::Utc::now().timestamp();
    // 5 minutes in seconds
    let interval = 5 * 60;

    // The previous 5m bucket, the current 5m bucket, and the next 5m bucket.
    // 1772334000 was Sunday, March 1, 2026 4:20:00 AM GMT.
    // Let's see what the current time buckets are:
    for offset_minutes in [-10, -5, 0, 5, 10, 15] {
        let bucket = ((now + (offset_minutes * 60)) / interval) * interval;
        println!("Offset {}m -> Timestamp: {}", offset_minutes, bucket);
    }

    Ok(())
}
