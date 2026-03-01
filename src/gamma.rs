/// Gamma REST API client for resolving active Polymarket token IDs.
///
/// Polymarket uses "token IDs" (ERC-1155 token identifiers) to identify
/// YES/NO sides of each market on the WebSocket. We resolve them from the
/// Gamma API by searching for markets matching our base slugs.
use log::{info, warn};
use serde::Deserialize;

const GAMMA_API: &str = "https://gamma-api.polymarket.com";

/// A token within a market (YES or NO side)
#[derive(Debug, Deserialize, Clone)]
pub struct MarketToken {
    pub token_id: String,
    pub outcome: String,
}

/// Condensed Gamma market response
#[derive(Debug, Deserialize, Clone)]
pub struct GammaMarket {
    pub condition_id: Option<String>,
    pub slug: Option<String>,
    pub question: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    // The old endpoint returned tokens array
    pub tokens: Option<Vec<MarketToken>>,
}

/// Resolved token IDs for a market, with the slug used to match it back to our config.
#[derive(Debug, Clone)]
pub struct ResolvedMarket {
    pub slug: String,
    pub condition_id: String,
    pub yes_token_id: String,
    pub no_token_id: Option<String>,
}

/// Fetch active markets from Gamma API for each base prefix (e.g., "btc-updown-5m-")
/// and extract token IDs. We fetch all active markets and filter locally.
pub async fn fetch_rotating_token_ids(
    client: &reqwest::Client,
    base_prefixes: &[&str],
) -> Vec<ResolvedMarket> {
    let mut resolved = Vec::new();

    // 5-minute markets always end precisely at the UNIX timestamp of their 5-minute boundary
    let now = chrono::Utc::now().timestamp();
    let interval = 5 * 60;

    // We check the current 5m bucket, and the +5m bucket to find the one actively being traded.
    // Polymarket tends to spin them up explicitly ending on these buckets.
    let buckets = [
        (now / interval) * interval,
        ((now + interval) / interval) * interval,
    ];

    for &prefix in base_prefixes {
        for bucket in buckets {
            let exact_slug = format!("{}{}", prefix, bucket);
            let url = format!("{}/events?slug={}", GAMMA_API, exact_slug);

            info!("[Gamma] Requesting: {}", url);

            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(_) => continue,
            };

            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = json.as_array() {
                    for event in arr {
                        if let Some(markets) = event["markets"].as_array() {
                            for market in markets {
                                let active = market["active"].as_bool().unwrap_or(false);
                                let closed = market["closed"].as_bool().unwrap_or(true);

                                if active && !closed {
                                    if let Some(slug) = market["slug"].as_str() {
                                        if let Some(condition_id) = market["conditionId"].as_str() {
                                            // Extract token IDs safely
                                            if let Some(clobs_str) = market["clobTokenIds"].as_str()
                                            {
                                                if let Ok(tokens) =
                                                    serde_json::from_str::<Vec<String>>(clobs_str)
                                                {
                                                    if !tokens.is_empty() {
                                                        let rm = ResolvedMarket {
                                                            slug: slug.to_string(),
                                                            condition_id: condition_id.to_string(),
                                                            yes_token_id: tokens[0].clone(),
                                                            no_token_id: tokens.get(1).cloned(),
                                                        };
                                                        info!(
                                                            "[Gamma] Resolved active 5m market: {} → condition={} yes_token={}...",
                                                            rm.slug,
                                                            rm.condition_id,
                                                            &rm.yes_token_id
                                                                [..8.min(rm.yes_token_id.len())]
                                                        );
                                                        resolved.push(rm);

                                                        // Stop searching for this prefix once we find an active market
                                                        break;
                                                    } else {
                                                        warn!(
                                                            "[Gamma] Tokens array empty for {}",
                                                            slug
                                                        );
                                                    }
                                                } else {
                                                    warn!(
                                                        "[Gamma] Failed to parse clobTokenIds array for {}",
                                                        slug
                                                    );
                                                }
                                            } else {
                                                warn!(
                                                    "[Gamma] Missing clobTokenIds string for {}",
                                                    slug
                                                );
                                            }
                                        } else {
                                            warn!("[Gamma] Missing conditionId for {}", slug);
                                        }
                                    }
                                } else {
                                    warn!(
                                        "[Gamma] Found {} but it's active={} closed={}",
                                        market["slug"], active, closed
                                    );
                                }
                            }
                        } else {
                            warn!(
                                "[Gamma] event['markets'] is not an array for {}",
                                exact_slug
                            );
                        }
                    }
                } else {
                    warn!("[Gamma] response is not an array for {}", exact_slug);
                }
            } else {
                warn!("[Gamma] JSON parse failed entirely for {}", exact_slug);
            }
        }
    }

    resolved
}
