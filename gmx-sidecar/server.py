import os
import json
import asyncio
from pathlib import Path
from dotenv import load_dotenv

# Load .env from project root
env_path = Path(__file__).resolve().parent.parent / ".env"
load_dotenv(env_path)

from flask import Flask, request, jsonify
from web3 import Web3
from gmx_python_sdk.scripts.v2.order.create_increase_order import IncreaseOrder
from gmx_python_sdk.scripts.v2.order.create_decrease_order import DecreaseOrder
from gmx_python_sdk.scripts.v2.gmx_utils import ConfigManager

app = Flask(__name__)

# Config
RPC_URL = os.environ.get("GMX_RPC_URL", "https://arb1.arbitrum.io/rpc")
PRIVATE_KEY = os.environ.get("WALLET_PRIVATE_KEY", "")

if not PRIVATE_KEY:
    print("FATAL: WALLET_PRIVATE_KEY not set")
    exit(1)

if PRIVATE_KEY.startswith("0x"):
    PRIVATE_KEY = PRIVATE_KEY[2:]

w3 = Web3(Web3.HTTPProvider(RPC_URL))
account = w3.eth.account.from_key(PRIVATE_KEY)
WALLET = account.address

# Build ConfigManager
GMX_CONFIG = ConfigManager(chain="arbitrum")
GMX_CONFIG.set_rpc(RPC_URL)
GMX_CONFIG.set_chain_id(42161)
GMX_CONFIG.set_wallet_address(WALLET)
GMX_CONFIG.set_private_key(PRIVATE_KEY)

# GMX V2 Market addresses on Arbitrum
MARKETS = {
    "ETH/USD": {
        "market": "0x70d95587d40A2caf56bd97485aB3Eec10Bee6336",
        "index_token": "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",  # WETH
        "collateral": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",  # USDC
    },
    "BTC/USD": {
        "market": "0x47c031236e19d024b42f8AE6780E44A573170703",
        "index_token": "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f",  # WBTC
        "collateral": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    },
    "ARB/USD": {
        "market": "0xC25cEf6061Cf5dE5eb761b50E4743c1F5D7E5407",
        "index_token": "0x912CE59144191C1204E64559FE8253a0e49E6548",  # ARB
        "collateral": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    },
    "LINK/USD": {
        "market": "0x7f1fa204bb700853D36994DA19F830b6Ad18455C",
        "index_token": "0xf97f4df75117a78c1A5a0DBb814Af92458539FB4",  # LINK
        "collateral": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    },
    "SOL/USD": {
        "market": "0x09400D9DB990D5ed3f35D7be61DfAEB900Af03C9",
        "index_token": "0x2bcC6D6CdBbDC0a4071e48bb3B969b06B3330c07",  # SOL
        "collateral": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    },
    "PEPE/USD": {
        "market": "0x2b477989A149B17073D9C9C82eC9cB03591e20c6",
        "index_token": "0x25d887Ce7a35172C62FeBFD67a1856F20FaEbB00",  # PEPE
        "collateral": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    },
}

positions = {}


@app.route("/health")
def health():
    return jsonify({"status": "ok", "wallet": WALLET, "chain": 42161})


@app.route("/open", methods=["POST"])
def open_position():
    data = request.json
    pair = data.get("pair")
    side = data.get("side", "long")
    leverage = data.get("leverage", 5)
    collateral_usd = data.get("collateralUsd", 1)

    if pair not in MARKETS:
        return jsonify({"error": f"Unknown pair: {pair}. Available: {list(MARKETS.keys())}"}), 400

    market = MARKETS[pair]
    is_long = side == "long"

    # Convert collateral to raw amount (USDC has 6 decimals)
    collateral_raw = int(collateral_usd * 10**6)
    # Size delta in USD (scaled by 10^30 for GMX)
    size_delta = int(collateral_usd * leverage * 10**30)

    try:
        print(f"Opening {side} {pair} {leverage}x with ${collateral_usd}...")
        order = IncreaseOrder(
            config=GMX_CONFIG,
            market_key=market["market"],
            collateral_address=market["collateral"],
            index_token_address=market["index_token"],
            is_long=is_long,
            size_delta=size_delta,
            initial_collateral_delta_amount=str(collateral_raw),
            slippage_percent=0.01,
            swap_path=[],
            debug_mode=False,
        )
        order_id = f"gmx-{int(asyncio.get_event_loop().time() * 1000)}"
        positions[order_id] = {
            "pair": pair,
            "side": side,
            "leverage": leverage,
            "collateralUsd": collateral_usd,
        }
        print(f"Position opened: {order_id}")
        return jsonify({"success": True, "orderId": order_id, "pair": pair, "side": side, "leverage": leverage})
    except Exception as e:
        print(f"Open failed: {e}")
        return jsonify({"error": str(e)}), 500


@app.route("/close", methods=["POST"])
def close_position():
    data = request.json
    pair = data.get("pair")
    side = data.get("side", "long")

    if pair not in MARKETS:
        return jsonify({"error": f"Unknown pair: {pair}"}), 400

    market = MARKETS[pair]
    is_long = side == "long"

    try:
        print(f"Closing {side} {pair}...")
        # Get position size from on-chain
        order = DecreaseOrder(
            config=GMX_CONFIG,
            market_key=market["market"],
            collateral_address=market["collateral"],
            index_token_address=market["index_token"],
            is_long=is_long,
            size_delta_usd=0,  # Close full position
            initial_collateral_delta_amount=0,
            slippage_percent=0.01,
            debug_mode=False,
        )
        return jsonify({"success": True, "pair": pair, "side": side})
    except Exception as e:
        print(f"Close failed: {e}")
        return jsonify({"error": str(e)}), 500


@app.route("/positions")
def get_positions():
    return jsonify({"positions": {}, "tracked": positions})


if __name__ == "__main__":
    print(f"GMX Sidecar running on port 8081")
    print(f"Wallet: {WALLET}")
    print(f"Chain: Arbitrum (42161)")
    app.run(host="127.0.0.1", port=8081)
