use serde_json::Value;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    // We try to use the gamma api with search queries which works much faster
    let qs = ["btc up or down 5m", "btc updown 5m", "btc-updown-5m"];

    for q in qs {
        let url = format!(
            "https://gamma-api.polymarket.com/events?query={}&active=true&closed=false",
            q.replace(" ", "%20")
        );
        let resp = client.get(&url).send().await?.text().await?;
        println!("Query: {}", q);
        if let Ok(json) = serde_json::from_str::<Value>(&resp) {
            if let Some(arr) = json.as_array() {
                println!("  Found {} events", arr.len());
                for e in arr {
                    if let Some(slug) = e["slug"].as_str() {
                        println!("  - {}", slug);
                    }
                }
            }
        }
    }

    Ok(())
}
