#!/bin/bash
# FID-154: Anvil health check + (re)start + prefund.
# Called from start-anvil.bat on Windows. Designed to be idempotent: if Anvil
# is already healthy, exits 0 immediately. If not, kills stale, restarts,
# prefunds, verifies.
#
# FID-157 (2026-06-15): added optional --fork-block-number argument. Pass
# "current" or omit for the latest block; pass an integer to fork at a
# specific block. Default wait timeout extended to 120s for big backfills.

set -e

export PATH="$HOME/.foundry/bin:$PATH"

WALLET=0x543CA0434B84aD38c858D2D178D2082521711fBC
USDC=0xaf88d065e77c8cC2239327C5EDb3A432268e5831
RPC=http://127.0.0.1:8545
FORK_URL=https://arb1.arbitrum.io/rpc
ANVIL_BIN=$HOME/.foundry/bin/anvil

# Fork-block override (FID-157):
#   "current" = use the latest mainnet block (default)
#   <int>     = fork at that specific block
FORK_BLOCK="${1:-current}"

# 1) Health check: is the existing port-8545 listener actually a working Anvil
#    AND is it at the block we want? If user passed a different block, force a refork.
if [ "$FORK_BLOCK" = "current" ]; then
    if cast chain-id --rpc-url "$RPC" >/dev/null 2>&1; then
        echo "[Anvil] Already running and responsive on port 8545."
        exit 0
    fi
else
    # User specified a block — always refork even if a listener exists.
    if cast chain-id --rpc-url "$RPC" >/dev/null 2>&1; then
        echo "[Anvil] Port 8545 responsive but block $FORK_BLOCK requested. Reforking."
    fi
fi

# 2) Kill any stale listener on 8545
echo "[Anvil] Killing stale listener on port 8545..."
fuser -k 8545/tcp 2>/dev/null || true
sleep 3

# 3) Start Anvil as a backgrounded, detached process (setsid)
if [ "$FORK_BLOCK" = "current" ]; then
    echo "[Anvil] Starting fork of Arbitrum One at latest block..."
    setsid "$ANVIL_BIN" --fork-url "$FORK_URL" --port 8545 --silent > /tmp/anvil.log 2>&1 &
else
    echo "[Anvil] Starting fork of Arbitrum One at block $FORK_BLOCK..."
    setsid "$ANVIL_BIN" --fork-url "$FORK_URL" --port 8545 --fork-block-number "$FORK_BLOCK" --silent > /tmp/anvil.log 2>&1 &
fi
ANVIL_PID=$!
disown 2>/dev/null || true
echo "[Anvil] Launched (PID $ANVIL_PID). Waiting for readiness..."

# 4) Poll for readiness (up to 120 seconds — big forks need time)
COUNT=0
while [ $COUNT -lt 120 ]; do
    if cast chain-id --rpc-url "$RPC" >/dev/null 2>&1; then
        echo "[Anvil] Ready after $((COUNT + 1)) seconds."
        break
    fi
    sleep 1
    COUNT=$((COUNT + 1))
done

if [ $COUNT -ge 120 ]; then
    echo "[Anvil] ERROR: did not respond within 120 seconds."
    echo "[Anvil] Log: $(cat /tmp/anvil.log 2>/dev/null | tail -10)"
    exit 1
fi

# 5) Prefund wallet: 10 ETH + 50 USDC
echo "[Anvil] Prefunding wallet (10 ETH + 50 USDC)..."

# ETH: 10 ETH = 0x8AC7230489E80000 (wei)
cast rpc anvil_setBalance "$WALLET" 0x8AC7230489E80000 --rpc-url "$RPC" >/dev/null

# USDC: storage slot for balances[wallet] in FiatTokenV2 is keccak256(wallet . uint256(9))
# Compute slot and set the value to 50e6 = 0x02FAF080
SLOT=$(cast index address "$WALLET" 9)
cast rpc anvil_setStorageAt "$USDC" "$SLOT" 0x0000000000000000000000000000000000000000000000000000000002FAF080 --rpc-url "$RPC" >/dev/null

# 6) Verify the prefund worked
USDC_BAL=$(cast call "$USDC" 'balanceOf(address)(uint256)' "$WALLET" --rpc-url "$RPC" 2>/dev/null || echo "0")
ETH_BAL=$(cast balance "$WALLET" --rpc-url "$RPC" --ether 2>/dev/null || echo "0")

if [ "$USDC_BAL" = "50000000" ] || [ "$USDC_BAL" = "50000000 [5e7]" ]; then
    echo "[Anvil] Prefund verified: $ETH_BAL ETH, $USDC_BAL raw USDC."
else
    echo "[Anvil] WARNING: USDC balance is '$USDC_BAL' (expected 50000000). Prefund may have failed."
    exit 1
fi

echo "[Anvil] Ready for engine startup."
exit 0
