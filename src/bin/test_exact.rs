use serde_json::Value;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    // We try to see if Polymarket Gamma supports an exact match on `slug` when we don't include pagination limits
    // Since the API ignores `slug_contains`, does it strictly enforce `slug=`?
    let url = "https://gamma-api.polymarket.com/events?slug=btc-updown-5m-1772334000";
    let resp = client.get(url).send().await?.text().await?;
    println!("GET {}", url);
    if let Ok(json) = serde_json::from_str::<Value>(&resp) {
        if let Some(arr) = json.as_array() {
            println!("  Found {} events", arr.len());
            for e in arr {
                println!("  - {}", e["slug"]);
            }
        }
    }

    Ok(())
}
