# Hyperliquid — v0.15.0+ Candidate Venue (Research, Not Action)

**Author:** Vera (sponsored by Spencer)
**Status:** Parked / dialog-only. No code change. Captured here so it survives in the long-term notes.

---

## TL;DR

Hyperliquid identified as a candidate v0.15.0+ execution venue. Spencer is in Kentucky; HL's TOS restricts US persons. The **actual blocker is "single chain"**, not "no HL." Adding HL is a 2-3 FID workstream (config + executor + liquidation module) **once parallel multi-chain ships.**

---

## Why HL is interesting (Spencer-aligned)

- Pure on-chain CLOB, sub-second finality (HyperBFT consensus, ~10ms median latency)
- Agent-wallet model: main wallet authorizes an agent key on-chain; agent can trade but **cannot withdraw** — best security model for autonomous agents
- WebSocket-first API, no polling
- Deterministic on-chain liquidations broadcast over API — perfect for liquidation-cascade strategies (FID-057-001)
- USDC native, no bridge token
- No KYC for protocol-level access; geoblock is on the **frontend**, not the protocol

## Why HL is **not v0.14.x**

- The engine is **single-chain-at-a-time** today (per-cycle `SAVANT_CHAIN` env var). Adding HL as a 6th chain in `config/default.toml` adds nothing if the engine still picks one chain per cycle.
- The actual unlock is **parallel multi-chain** via `tokio::spawn` per-chain sub-cycle. That's FID-169 (deferred to v0.15.0).
- HL doesn't change that — it's just one more chain in `[chains.*]` once the parallel loop exists.

## Legal reality (KY / US)

- **Hyperliquid's TOS explicitly restricts US persons.** Frontend is IP-geoblocked for US/Ontario/heavily-sanctioned nations.
- The protocol layer is permissionless. The frontend is not. The TOS question is enforced at the frontend layer.
- **Agent-wallet model does NOT bypass TOS.** It only limits the blast radius (trades only, no withdrawal). The act of trading from a US IP is the TOS violation, not the wallet structure.
- **Routes for US persons** (vetted by community, all TOS-grey):
  1. **VPN to a non-US jurisdiction to load the frontend** (Switzerland, Singapore). Authorize the agent wallet on-chain once. Then run the bot from a US IP. This is what most US-based algo traders do. TOS-violating but not enforced.
  2. **Community-run frontend hosted in a non-US jurisdiction.** HL Foundation has signaled support but hasn't shipped one yet. This is the cleanest route when available.
  3. **Non-US entity (Wyoming LLC, foreign entity) operates the agent wallet.** Cleanest legally. Common for serious US-based algo shops.
  4. **Wait for HL to actually decentralize frontend access** — they've talked about it for ~12 months, no delivery.

## What Gemini got wrong / glossed over

1. **"Technically yes, no KYC"** minimizes the TOS exposure. Not a "yeah but" — it's the real blocker.
2. **Funding costs on perps** (hourly). 0x spot has no funding. Current strategy (0.8-1.2% scalps, 5-min cycles) is funding-insensitive. But for any extension to 30-60 min hold times, funding bleed becomes a real P&L drag. Strategy soul needs a funding-aware check before HL is wired in. (Out-of-scope FID for v0.15.0+.)
3. **"Architecturally closest"** — dYdX v4, Aevo, edgeX, Paradex are alternatives. But none beat HL on latency + liquidity + agent-wallet security.
4. **"Unofficially hyperliquid-rust-sdk"** — Spencer should NOT use a community SDK. Build a custom lightweight client in `src/execution/hyperliquid/` to match Savant's quality bar.

## Path forward (when ready)

Treat as new workstream, not a quick add-on. Roughly:

- **v0.15.0 prerequisite**: FID-169 (parallel multi-chain engine loop). Without this, HL integration is cosmetic.
- **After FID-169 ships**:
  - FID-HL-1: Add `[chains.hyperliquid]` config + stub executor
  - FID-HL-2: Build `src/execution/hyperliquid/` (info endpoint, EIP-712 order signing, WebSocket, agent-wallet authorization)
  - FID-HL-3: Liquidation-cascade module integration (FID-057-001 + HL's deterministic on-chain liquidations)
  - FID-HL-4: Funding-aware hold-time check in strategy soul
  - FID-HL-5: Legal/entity decision — Spencer's call on KY route

## Decision

**No action this session. Parked until v0.15.0.**

---

*Captured 2026-06-17 01:07 EST. Spencer: "simply opening dialog." Not a FID per Spencer's request — no FIDs for parked research.*
