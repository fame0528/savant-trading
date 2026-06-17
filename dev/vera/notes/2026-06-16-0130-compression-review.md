# Compression Review - 2026-06-16 01:30 EST

## Context
Spencer asked me to review openclaw and hermes-agent for context compression strategies after seeing 25+ Delta-compression warnings per cycle in the engine log. Engine was at cycle 17, 0/17 trades, 0 trades over 2h.

## Files read
- openclaw: src/agents/compaction.ts (434 lines)
- openclaw: src/context-engine/delegate.ts (104 lines)
- hermes: agent/context_compressor.py (first 250 of 2182)
- ours: src/agent/context_state.rs (354 lines)

## Critical bug found
ContextState.previous_text is shared across all pairs in a cycle. Pair 2's compute_delta diffs against pair 1's prompt. Different pair = different content = ~95% diff = always Full + Anti-thrashing warning.

## Comparison table: us vs them
| Capability | us | openclaw | hermes |
| Hash-based delta | Y (broken) | Y (uses LLM) | Y (uses LLM) |
| LLM summarization | N | Y | Y |
| Token-budget aware | N | Y | Y |
| Chunked processing | N | Y | Y |
| Structured summary | N | Y | Y |
| Per-message token est | N | Y | Y |
| Image token accounting | N/A | N/A | Y |
| Tool output pruning | N | N | Y |
| Oversized retry | N | Y (3-stage) | Y |
| Path mention extraction | N | N/A | Y |

We are 3 years behind on context compression design.

## Plan for FID-164 tomorrow
- Workstream 1 (20 min): Per-pair state HashMap in ContextState
- Workstream 2 (30 min): Token-based detection, adaptive threshold, smarter anti-thrashing
- Workstream 3 (FID-165 separate): LLM summarization port

## Open items not in FID-164
- Multi-chain expansion (Spencer's chain coverage observation)
- LLM latency spike (170s cycle 17) 
- Strategy re-tuning for illiquid DEX micro-caps (separate conversation)

## Engine state at 0130 EST
- Engine running, 0/17 trades, conviction 0.000 for every pass
- The LLM is correctly seeing no setup. The data shows vol=0, RSI extremes on dead micro-caps
- Data integrity fix (FID-163) is necessary but not sufficient
- Real problem: strategy tuned for liquid majors, pointed at illiquid micro-caps
