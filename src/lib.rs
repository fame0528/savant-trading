//! Savant Trading Engine — AI-native autonomous crypto trading system for Kraken exchange.
//!
//! Built in Rust with tokio async runtime. The AI agent (mimo v2.5 pro) IS the brain,
//! informed by 11 curated trading transcripts and live market insight data.
//!
//! # Architecture
//!
//! - `agent` — AI brain: knowledge injection, system prompts, LLM provider, decision engine
//! - `backtest` — Historical strategy validation and walk-forward optimization
//! - `core` — Configuration, types, errors, events
//! - `data` — Market data fetching and indicator calculations
//! - `insight` — Live market context (sentiment, funding, liquidation, flows, news, on-chain)
//! - `strategy` — Rule-based strategies (optional parallel signals)
//! - `risk` — Position sizing, stops, circuit breakers (independent of AI)
//! - `execution` — Paper trading and live execution
//! - `monitor` — Trade journal and performance metrics
//! - `vault` — Obsidian vault integration (Glass House)

pub mod agent;
pub mod backtest;
#[macro_use]
pub mod core;
pub mod data;
pub mod execution;
pub mod insight;
pub mod jury_state;
pub mod memory;
pub mod monitor;
pub mod risk;
pub mod sandbox;
pub mod security;
pub mod strategy;
pub mod tui;
pub mod vault;
