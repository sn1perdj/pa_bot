/// Polymarket WebSocket client with auto-reconnect and PING keepalive.
///
/// Connects to `wss://ws-subscriptions-clob.polymarket.com/ws/market`,
/// subscribes to the given token IDs, and routes each incoming message
/// through the `EventRouter`.
///
/// Reconnect strategy: exponential backoff starting at 1s, capped at 30s.
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde_json;
use std::collections::HashSet;
use tokio::sync::mpsc;
use tokio::time::{self, interval};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::engine::router::EventRouter;
use crate::engine::stats::EngineStats;
use crate::ws::messages::{PolymarketWsMsg, SubscribeRequest, UnsubscribeRequest};

const WS_ENDPOINT: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
const PING_INTERVAL_SECS: u64 = 10;
const INITIAL_RECONNECT_DELAY_SECS: u64 = 1;
const MAX_RECONNECT_DELAY_SECS: u64 = 30;

/// Run the Polymarket WebSocket client loop. This function never returns
/// under normal operation — it reconnects automatically on disconnection.
pub async fn run(
    mut sub_rx: mpsc::Receiver<Vec<String>>,
    mut router: EventRouter,
    stats: Arc<EngineStats>,
) {
    let mut backoff = INITIAL_RECONNECT_DELAY_SECS;
    let mut current_subscriptions: HashSet<String> = HashSet::new();

    loop {
        info!("[WS] Connecting to Polymarket WebSocket: {}", WS_ENDPOINT);

        match connect_async(WS_ENDPOINT).await {
            Ok((ws_stream, _response)) => {
                info!("[WS] Connection established");
                backoff = INITIAL_RECONNECT_DELAY_SECS; // Reset backoff on success

                let (mut write, mut read) = ws_stream.split();

                // Re-subscribe to any existing tokens we had before disconnecting
                if !current_subscriptions.is_empty() {
                    let sub_req =
                        SubscribeRequest::market(current_subscriptions.iter().cloned().collect());
                    let sub_json =
                        serde_json::to_string(&sub_req).expect("Subscription serialization failed");
                    if let Err(e) = write.send(Message::Text(sub_json.into())).await {
                        error!("[WS] Failed to send initial subscription: {}", e);
                        continue;
                    }
                    info!(
                        "[WS] Re-subscribed to {} token streams",
                        current_subscriptions.len()
                    );
                }

                // Ping timer
                let mut ping_timer = interval(Duration::from_secs(PING_INTERVAL_SECS));
                ping_timer.tick().await; // Consume immediate first tick

                loop {
                    tokio::select! {
                        // 1. Dynamic subscription updates
                        Some(new_tokens) = sub_rx.recv() => {
                            let new_set: HashSet<String> = new_tokens.into_iter().collect();

                            // Find tokens to unsubscribe from
                            let to_unsubscribe: Vec<String> = current_subscriptions
                                .difference(&new_set)
                                .cloned()
                                .collect();

                            // Find tokens to subscribe to
                            let to_subscribe: Vec<String> = new_set
                                .difference(&current_subscriptions)
                                .cloned()
                                .collect();

                            if !to_unsubscribe.is_empty() {
                                let unsub_req = UnsubscribeRequest::market(to_unsubscribe.clone());
                                if let Ok(unsub_json) = serde_json::to_string(&unsub_req) {
                                    if let Err(e) = write.send(Message::Text(unsub_json.into())).await {
                                        error!("[WS] Failed to send unsubscribe: {}", e);
                                        break; // force reconnect
                                    }
                                    info!("[WS] Unsubscribed from {} old tokens", to_unsubscribe.len());
                                }
                            }

                            if !to_subscribe.is_empty() {
                                let sub_req = SubscribeRequest::market(to_subscribe.clone());
                                if let Ok(sub_json) = serde_json::to_string(&sub_req) {
                                    if let Err(e) = write.send(Message::Text(sub_json.into())).await {
                                        error!("[WS] Failed to send subscribe: {}", e);
                                        break; // force reconnect
                                    }
                                    info!("[WS] Subscribed to {} new tokens", to_subscribe.len());
                                }
                            }

                            current_subscriptions = new_set;
                        }

                        // 2. Ping timer
                        _ = ping_timer.tick() => {
                            // Polymarket expects a plain "PING" text frame
                            if let Err(e) = write.send(Message::Text("PING".into())).await {
                                error!("[WS] Ping failed — reconnecting: {}", e);
                                break;
                            }
                        }

                        msg = read.next() => {
                            match msg {
                                None => {
                                    warn!("[WS] Stream closed by server — reconnecting");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("[WS] Read error — reconnecting: {}", e);
                                    break;
                                }
                                Some(Ok(Message::Text(text))) => {
                                    // Skip plain PONG replies
                                    if text.trim() == "PONG" {
                                        continue;
                                    }

                                    match PolymarketWsMsg::from_json(&text) {
                                        Some(parsed) => {
                                            router.dispatch(parsed);
                                        }
                                        None => {
                                            stats.inc_parse_errors();
                                            warn!("[WS] Could not parse message: {:.120}", text);
                                        }
                                    }
                                }
                                Some(Ok(Message::Binary(_))) => {
                                    warn!("[WS] Unexpected binary frame — skipping");
                                }
                                Some(Ok(Message::Ping(data))) => {
                                    // Respond to WS-level PINGs (distinct from Polymarket's text PING)
                                    let _ = write.send(Message::Pong(data)).await;
                                }
                                Some(Ok(Message::Close(_))) => {
                                    info!("[WS] Received Close frame — reconnecting");
                                    break;
                                }
                                Some(Ok(_)) => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("[WS] Connection failed: {} — retrying in {}s", e, backoff);
            }
        }

        // Exponential backoff before reconnecting
        time::sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(MAX_RECONNECT_DELAY_SECS);
    }
}
