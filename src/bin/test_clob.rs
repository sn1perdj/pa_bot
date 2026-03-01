use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    // The Polymarket frontend uses an Algolia equivalent or their own custom GraphQL endpoint for search.
    // Wait, let's just query their actual Clob API for active markets:
    // The CLOB API is separate from Gamma.
    let url = "https://clob.polymarket.com/markets";
    let resp = client.get(url).send().await?.text().await?;
    println!("GET {}", url);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&resp) {
        if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
            println!("  Found {} clob markets", arr.len());
            for m in arr.iter().take(2) {
                println!("  - {:?}", m["market"]);
            }
        } else {
            println!("Response keys: {:?}", json.as_object().unwrap().keys());
        }
    }

    Ok(())
}
