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
        stop_loss: Option<f64>,
    ) -> Result<Order, crate::core::error::ExecutionError>;

    async fn close_position(
        &mut self,
        position_id: &str,
    ) -> Result<Order, crate::core::error::ExecutionError>;

    fn open_positions(&self) -> Vec<&Position>;

    fn balance(&self) -> f64;
}
