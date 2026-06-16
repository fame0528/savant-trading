#!/bin/bash
echo "=== Anvil responsiveness ==="
/home/spencer/.foundry/bin/cast chain-id --rpc-url http://127.0.0.1:8545
echo "=== Block number ==="
/home/spencer/.foundry/bin/cast block-number --rpc-url http://127.0.0.1:8545
echo "=== Test wallet USDC balance (Anvil fork) ==="
/home/spencer/.foundry/bin/cast call --rpc-url http://127.0.0.1:8545 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 'balanceOf(address)(uint256)' 0x543CA0434B84aD38c858D2D178D2082521711fBC
echo "=== Test wallet ETH balance (Anvil fork) ==="
/home/spencer/.foundry/bin/cast balance --rpc-url http://127.0.0.1:8545 0x543CA0434B84aD38c858D2D178D2082521711fBC
