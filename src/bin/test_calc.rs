use serde_json::Value;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let now = chrono::Utc::now().timestamp();
    let interval = 5 * 60;

    // Test current, next, and next+1 buckets just to see which one is "active" on Polymarket
    for offset in [0, 5, 10, 15] {
        let bucket = ((now + (offset * 60)) / interval) * interval;
        let slug = format!("btc-updown-5m-{}", bucket);

        let url = format!("https://gamma-api.polymarket.com/events?slug={}", slug);
        let resp = client.get(&url).send().await?.text().await?;

        if let Ok(json) = serde_json::from_str::<Value>(&resp) {
            if let Some(arr) = json.as_array() {
                if !arr.is_empty() {
                    let e = &arr[0];
                    println!(
                        "FOUND {}: active={} closed={}",
                        slug, e["active"], e["closed"]
                    );
                } else {
                    println!("MISSING {}", slug);
                }
            }
        }
    }
    Ok(())
}
