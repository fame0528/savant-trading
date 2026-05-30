use tokio::sync::broadcast;
use tracing::warn;

use crate::core::types::TradingEvent;

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<TradingEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: TradingEvent) {
        if let Err(e) = self.sender.send(event) {
            warn!("EventBus send failed (no subscribers): {}", e);
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TradingEvent> {
        self.sender.subscribe()
    }
}
