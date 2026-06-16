#!/bin/bash
set -e

export PATH="$HOME/.foundry/bin:$PATH"

WALLET=0x543CA0434B84aD38c858D2D178D2082521711fBC
USDC=0xaf88d065e77c8cC2239327C5EDb3A432268e5831
GRT=0x9623063377ad1b27544c965ccd7342f7ea7e88c7
RPC=http://127.0.0.1:8545

echo "=== Killing any existing Anvil on port 8545 ==="
FUSER=$(fuser 8545/tcp 2>/dev/null || true)
if [ -n "$FUSER" ]; then
  kill $FUSER 2>/dev/null || true
  sleep 2
  echo "Killed stale process on port 8545"
fi

echo "=== Starting Anvil fork of Arbitrum One ==="
setsid anvil --fork-url https://arb1.arbitrum.io/rpc --port 8545 --silent > /tmp/anvil.log 2>&1 &
PID=$!
sleep 12

echo "Anvil started (PID: $PID)"
# Verify Anvil is actually listening
cast block-number --rpc-url "$RPC" || { echo "ERROR: Anvil not responding"; exit 1; }

# Compute USDC balanceOf storage slot for our wallet
# FiatTokenV2 uses slot 9 for _balances mapping
# key = keccak256(abi.encode(address, uint256(9)))
echo "=== Computing USDC storage slot ==="
SLOT=$(cast index address "$WALLET" 9)
echo "Slot: $SLOT"

# Give wallet 10 ETH for gas (our wallet only has 0.0009 ETH on mainnet)
echo "=== Setting wallet ETH to 10 ==="
cast rpc anvil_setBalance "$WALLET" 0x8AC7230489E80000 --rpc-url "$RPC"

# Set USDC balance to 50 USDC (50 * 10^6 = 50000000 = 0x02FAF080)
echo "=== Setting USDC balance to 50 ==="
cast rpc anvil_setStorageAt "$USDC" "$SLOT" 0x0000000000000000000000000000000000000000000000000000000002FAF080 --rpc-url "$RPC"
echo "USDC balance set."

echo ""
echo "=== Verifying balances ==="
echo "--- ETH (ether) ---"
cast balance "$WALLET" --rpc-url "$RPC" --ether
echo "--- USDC (raw uint256) ---"
cast call "$USDC" 'balanceOf(address)(uint256)' "$WALLET" --rpc-url "$RPC"
echo "--- GRT (raw uint256) ---"
cast call "$GRT" 'balanceOf(address)(uint256)' "$WALLET" --rpc-url "$RPC"
echo "--- Chain ID ---"
cast chain-id --rpc-url "$RPC"
echo "--- Block Number ---"
cast block-number --rpc-url "$RPC"

echo ""
echo "=== Fork is running on port 8545 ==="
echo "Anvil PID: $PID"
echo "To stop: kill $PID"
echo "=== DONE ==="
