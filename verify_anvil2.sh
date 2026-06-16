#!/bin/bash
echo "=== Test wallet ETH balance (Anvil fork) ==="
/home/spencer/.foundry/bin/cast balance --rpc-url http://127.0.0.1:8545 0x543CA0434B84aD38c858D2D178D2082521711fBC --ether
echo "=== Test wallet GRT balance (Anvil fork) ==="
/home/spencer/.foundry/bin/cast call --rpc-url http://127.0.0.1:8545 0x9623063377ad1b27544c965ccd7342f7ea7e88c7 'balanceOf(address)(uint256)' 0x543CA0434B84aD38c858D2D178D2082521711fBC
