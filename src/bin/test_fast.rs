use serde_json::Value;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    // Test 1: slug=btc-updown-5m
    let url = "https://gamma-api.polymarket.com/events?slug=btc-updown-5m";
    let resp = client.get(url).send().await?.text().await?;
    println!("GET events?slug=btc-updown-5m");
    if let Ok(json) = serde_json::from_str::<Value>(&resp) {
        if let Some(arr) = json.as_array() {
            println!("  Found {} events", arr.len());
            for e in arr.iter().take(2) {
                println!("  - {}", e["slug"]);
            }
        }
    }

    // Test 2: markets?slug=btc-updown-5m
    let url2 = "https://gamma-api.polymarket.com/markets?slug=btc-updown-5m";
    let resp2 = client.get(url2).send().await?.text().await?;
    println!("\nGET markets?slug=btc-updown-5m");
    if let Ok(json2) = serde_json::from_str::<Value>(&resp2) {
        if let Some(arr) = json2.as_array() {
            println!("  Found {} markets", arr.len());
            for m in arr.iter().take(2) {
                println!("  - {}", m["slug"]);
            }
        }
    }

    // Test 3: markets?asset_slug=btc
    let url3 =
        "https://gamma-api.polymarket.com/markets?active=true&closed=false&limit=100&offset=0";
    // We already know this is too broad.

    Ok(())
}
