export PATH="$HOME/.foundry/bin:$PATH"
WALLET=0x543CA0434B84aD38c858D2D178D2082521711fBC
USDC=0xaf88d065e77c8cC2239327C5EDb3A432268e5831
RPC=http://127.0.0.1:8545

# Top up ETH to 10
cast rpc anvil_setBalance "$WALLET" 0x8AC7230489E80000 --rpc-url "$RPC" >/dev/null

# Top up USDC to 50 (50 * 10^6 = 0x02FAF080)
SLOT=$(cast index address "$WALLET" 9)
cast rpc anvil_setStorageAt "$USDC" "$SLOT" 0x0000000000000000000000000000000000000000000000000000000002FAF080 --rpc-url "$RPC" >/dev/null

echo "=== Verification ==="
echo "ETH: $(cast balance "$WALLET" --rpc-url "$RPC" --ether)"
echo "USDC raw: $(cast call "$USDC" 'balanceOf(address)(uint256)' "$WALLET" --rpc-url "$RPC")"
echo "GRT raw: $(cast call 0x9623063377ad1b27544c965ccd7342f7ea7e88c7 'balanceOf(address)(uint256)' "$WALLET" --rpc-url "$RPC")"
