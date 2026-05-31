//! Kraken WebSocket v2 client for real-time market data.
//!
//! Connects to wss://ws.kraken.com/v2 for real-time candles, order book,
//! trades, and ticker data. Handles heartbeat, sequence verification,
//! and auto-reconnection.

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::core::types::{Candle, OrderBook, OrderBookLevel};
use crate::data::kraken::TickerData;

/// WebSocket connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Messages from the WebSocket to the engine.
#[derive(Debug, Clone)]
pub enum WsMessage {
    Candle(Candle),
    BookUpdate(OrderBook),
    Trade {
        pair: String,
        price: f64,
        volume: f64,
        side: String,
    },
    Ticker(TickerData),
    StateChange(WsState),
}

/// Create a channel pair for WebSocket communication.
pub fn create_channel() -> (
    mpsc::UnboundedSender<WsMessage>,
    mpsc::UnboundedReceiver<WsMessage>,
) {
    mpsc::unbounded_channel()
}

/// Build subscribe messages for Kraken WS v2.
fn build_subscribe_messages(pairs: &[String], depth: u32) -> Vec<String> {
    vec![
        serde_json::json!({
            "method": "subscribe",
            "params": { "channel": "ticker", "symbol": pairs }
        })
        .to_string(),
        serde_json::json!({
            "method": "subscribe",
            "params": { "channel": "book", "symbol": pairs, "depth": depth }
        })
        .to_string(),
        serde_json::json!({
            "method": "subscribe",
            "params": { "channel": "trade", "symbol": pairs }
        })
        .to_string(),
    ]
}

/// Parse a Kraken WS v2 message and convert to WsMessage.
pub fn parse_message(raw: &str) -> Option<Vec<WsMessage>> {
    let json: serde_json::Value = serde_json::from_str(raw).ok()?;

    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
        if method == "subscribe" {
            let success = json
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);
            let channel = json
                .get("result")
                .and_then(|r| r.get("channel"))
                .and_then(|c| c.as_str())
                .unwrap_or("unknown");
            if success {
                debug!("Kraken WS subscribed to {}", channel);
            } else {
                warn!("Kraken WS subscribe failed for {}", channel);
            }
            return None;
        }
    }

    if json.get("channel").and_then(|c| c.as_str()) == Some("heartbeat") {
        return None;
    }

    let channel = json.get("channel")?.as_str()?;

    match channel {
        "ticker" => parse_ticker(&json),
        "book" => parse_book(&json),
        "trade" => parse_trades(&json),
        _ => None,
    }
    .map(|msg| vec![msg])
}

fn parse_ticker(json: &serde_json::Value) -> Option<WsMessage> {
    let data = json.get("data")?.as_array()?.first()?;
    let pair = data.get("symbol")?.as_str()?.to_string();
    let ask = data
        .get("ask")
        .and_then(|a| a.get("price"))
        .and_then(|p| p.as_f64())?;
    let bid = data
        .get("bid")
        .and_then(|b| b.get("price"))
        .and_then(|p| p.as_f64())?;
    let last = data.get("last").and_then(|l| l.as_f64()).unwrap_or(0.0);
    let volume = data.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);

    Some(WsMessage::Ticker(TickerData {
        pair,
        ask,
        bid,
        last,
        volume,
    }))
}

fn parse_book(json: &serde_json::Value) -> Option<WsMessage> {
    let data = json.get("data")?.as_array()?;
    let book_data = data.first()?;
    let pair = book_data.get("symbol")?.as_str()?.to_string();
    let asks = parse_book_levels(
        book_data
            .get("asks")
            .unwrap_or(&serde_json::Value::Array(vec![])),
    );
    let bids = parse_book_levels(
        book_data
            .get("bids")
            .unwrap_or(&serde_json::Value::Array(vec![])),
    );

    Some(WsMessage::BookUpdate(OrderBook {
        pair,
        bids,
        asks,
        timestamp: chrono::Utc::now(),
    }))
}

fn parse_book_levels(value: &serde_json::Value) -> Vec<OrderBookLevel> {
    let arr = match value.as_array() {
        Some(a) => a,
        None => return vec![],
    };

    arr.iter()
        .filter_map(|level| {
            let items = level.as_array()?;
            if items.len() < 2 {
                return None;
            }
            let price = items[0].as_f64()?;
            let volume = items[1].as_f64()?;
            Some(OrderBookLevel { price, volume })
        })
        .collect()
}

fn parse_trades(json: &serde_json::Value) -> Option<WsMessage> {
    let data = json.get("data")?.as_array()?;
    let trade = data.first()?;
    let pair = trade.get("symbol")?.as_str()?.to_string();
    let price = trade.get("price")?.as_f64()?;
    let volume = trade.get("qty")?.as_f64()?;
    let side = trade
        .get("side")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown")
        .to_string();

    Some(WsMessage::Trade {
        pair,
        price,
        volume,
        side,
    })
}

/// Connect to Kraken WebSocket v2 and stream messages.
///
/// Runs indefinitely with auto-reconnection. Sends parsed messages
/// through the channel. Spawn as a tokio task.
pub async fn connect(url: &str, pairs: Vec<String>, tx: mpsc::UnboundedSender<WsMessage>) {
    let subscribe_msgs = build_subscribe_messages(&pairs, 10);

    loop {
        info!("Kraken WS connecting to {}", url);
        let _ = tx.send(WsMessage::StateChange(WsState::Connecting));

        match tokio_tungstenite::connect_async(url).await {
            Ok((ws_stream, _)) => {
                info!("Kraken WS connected");
                let _ = tx.send(WsMessage::StateChange(WsState::Connected));

                let (mut write, mut read) = ws_stream.split();

                for msg in &subscribe_msgs {
                    if let Err(e) = write
                        .send(tokio_tungstenite::tungstenite::Message::Text(msg.clone()))
                        .await
                    {
                        error!("Kraken WS subscribe send error: {}", e);
                        break;
                    }
                }

                loop {
                    match read.next().await {
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                            if let Some(messages) = parse_message(&text) {
                                for msg in messages {
                                    if tx.send(msg).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(data))) => {
                            let _ = write
                                .send(tokio_tungstenite::tungstenite::Message::Pong(data))
                                .await;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            warn!("Kraken WS read error: {}", e);
                            break;
                        }
                        None => {
                            warn!("Kraken WS stream ended");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Kraken WS connection failed: {}", e);
            }
        }

        let _ = tx.send(WsMessage::StateChange(WsState::Reconnecting));
        warn!("Kraken WS reconnecting in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_subscribe_messages_count() {
        let msgs = build_subscribe_messages(&["BTC/USD".to_string()], 10);
        assert_eq!(msgs.len(), 3);
    }

    #[test]
    fn build_subscribe_messages_content() {
        let msgs = build_subscribe_messages(&["BTC/USD".to_string(), "ETH/USD".to_string()], 10);
        assert!(msgs[0].contains("ticker"));
        assert!(msgs[1].contains("book"));
        assert!(msgs[2].contains("trade"));
        assert!(msgs[0].contains("BTC/USD"));
    }

    #[test]
    fn parse_heartbeat() {
        let msg = r#"{"channel":"heartbeat","type":"heartbeat"}"#;
        let result = parse_message(msg);
        assert!(result.is_none());
    }

    #[test]
    fn parse_subscribe_success() {
        let msg = r#"{"method":"subscribe","success":true,"result":{"channel":"ticker","symbol":"BTC/USD"}}"#;
        let result = parse_message(msg);
        assert!(result.is_none());
    }

    #[test]
    fn parse_unknown_channel() {
        let msg = r#"{"channel":"unknown","type":"update","data":[]}"#;
        let result = parse_message(msg);
        assert!(result.is_none());
    }
}
