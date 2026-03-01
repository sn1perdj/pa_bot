use serde_json::Value;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    // We fetch an event that contains "BTC" and see if we can find the 5m market nested in it
    let url =
        "https://gamma-api.polymarket.com/events?slug=bitcoin&active=true&closed=false&limit=100";
    let resp = client.get(url).send().await?.text().await?;
    if let Ok(json) = serde_json::from_str::<Value>(&resp) {
        if let Some(arr) = json.as_array() {
            for e in arr {
                if let Some(slug) = e["slug"].as_str() {
                    println!("Event: {}", slug);
                }
            }
        }
    }

    // Try finding via markets?tag_id for crypto and sort by volume to find the big ones
    let url2 = "https://gamma-api.polymarket.com/markets?tag_slug=crypto&active=true&closed=false&limit=5&order=volume&ascending=false";
    let resp2 = client.get(url2).send().await?.text().await?;
    println!("\nTop 5 volume crypto markets:");
    if let Ok(json2) = serde_json::from_str::<Value>(&resp2) {
        if let Some(arr) = json2.as_array() {
            for m in arr {
                println!("  - {}", m["slug"]);
            }
        }
    }

    Ok(())
}
