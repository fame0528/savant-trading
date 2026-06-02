use async_trait::async_trait;

use crate::core::types::{Order, Position, Side};

#[async_trait]
pub trait ExecutionEngine: Send + Sync {
    async fn place_order(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        price: Option<f64>,
    ) -> Result<Order, crate::core::error::ExecutionError>;

    async fn close_position(
        &mut self,
        position_id: &str,
    ) -> Result<Order, crate::core::error::ExecutionError>;

    fn open_positions(&self) -> Vec<&Position>;

    fn balance(&self) -> f64;

    // ---- Optional lifecycle methods with safe defaults ----

    /// Kill switch — cancel all orders and close all positions.
    /// Default: no-op (safe for backends that don't support it).
    async fn kill(&mut self) -> Result<(), crate::core::error::ExecutionError> {
        Ok(())
    }

    /// Sync balance from the exchange (e.g. from exchange API).
    /// Default: no-op (uses locally tracked balance).
    async fn sync_balance(&mut self) -> Result<(), crate::core::error::ExecutionError> {
        Ok(())
    }

    /// Place a stop-loss order on the exchange for a tracked position.
    /// Default: no-op (stops are monitored client-side).
    async fn place_stop_loss(&mut self, _position_id: &str) -> Result<(), crate::core::error::ExecutionError> {
        Ok(())
    }
}
