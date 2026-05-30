//! Agent module — AI-powered autonomous trading brain.
//!
//! - `knowledge` — Transcript knowledge loading, storage, and dynamic selection
//! - `prompts` — Modular system prompt composition
//! - `provider` — OpenAI-compatible LLM HTTP client
//! - `context_builder` — Aggregates market data + insight + knowledge into LLM context
//! - `decision_parser` — Extracts structured TradeDecision from LLM responses
//! - `orchestrator` — Main decision loop with autonomy level control

pub mod context_builder;
pub mod decision_parser;
pub mod knowledge;
pub mod orchestrator;
pub mod prompts;
pub mod provider;
