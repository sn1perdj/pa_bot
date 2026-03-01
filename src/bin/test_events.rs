use serde_json::Value;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    // We try to use the gamma api to filter by prefix via GraphQL optionally,
    // or we check if there's an undocumented active endpoint.
    let url = "https://gamma-api.polymarket.com/events?limit=50&active=true&closed=false&tag_id=21"; // 21 is crypto tag

    let resp = client.get(url).send().await?.text().await?;
    println!("Events tag_id=21");
    if let Ok(json) = serde_json::from_str::<Value>(&resp) {
        if let Some(arr) = json.as_array() {
            println!("  Found {} events", arr.len());
            for e in arr {
                if let Some(slug) = e["slug"].as_str() {
                    if slug.contains("btc") && slug.contains("5m") {
                        println!("  *** FOUND IT: {}", slug);
                    }
                }
            }
        }
    }

    Ok(())
}
