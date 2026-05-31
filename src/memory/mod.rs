//! Persistent memory system for the Savant trading agent.
//!
//! 4-tier memory hierarchy:
//! 1. Working Memory — current evaluation cycle (in-memory prompt)
//! 2. Core Memory — SOUL.md + knowledge units (persistent identity)
//! 3. Episodic Memory — SQLite WAL ledger of every decision + market snapshot
//! 4. Semantic Memory — extracted patterns, edge decay alerts, replay lessons

pub mod calibration;
pub mod context;
pub mod cusum;
pub mod episodic;
pub mod replay;
