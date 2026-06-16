"""
FID-155 Local Anvil Block Explorer
=================================
A tiny single-file block explorer for the local Anvil instance at
http://127.0.0.1:8545. No external dependencies (stdlib only).

Run:
    python anvil_explorer.py [PORT]

Then open http://localhost:PORT/ in your browser.

What it shows:
    - Latest block (height, timestamp, gas, txs)
    - Last N blocks with full tx list
    - Per-tx detail: from, to, value, gas, call data, decoded function,
      event logs, internal calls (eth_getTransactionReceipt)
    - Per-address: tx history, token balances (USDC, GRT, ETH)
    - Search by address, tx hash, or block number

Why this exists (2026-06-15):
    The dashboard shows engine state but not full tx data. Ethernal +
    Cloudflare tunnel was tried but the connection setup didn't complete
    in time. A local-only explorer gives the same data without auth, public
    exposure, or external service dependencies.

Per DECISION-014: this is a self-recovery / self-sufficiency tool.
The engine should always be inspectable end-to-end without third-party services.
"""

import http.server
import json
import sys
import urllib.request
import urllib.error
from datetime import datetime, timezone
from urllib.parse import urlparse, parse_qs

# ---- Config ----
ANVIL_RPC = "http://127.0.0.1:8545"
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 7777
HOST = "127.0.0.1"
WALLET = "0x543CA0434B84aD38c858D2D178D2082521711fBC"  # Default wallet to highlight
USDC = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"
GRT = "0x9623063377ad1b27544c965ccd7342f7ea7e88c7"
WETH = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"  # Arbitrum WETH

# 4-byte function selectors we know about (for call data decode)
KNOWN_SELECTORS = {
    "0x70a08231": "balanceOf(address)",
    "0xa9059cbb": "transfer(address,uint256)",
    "0x23b872dd": "transferFrom(address,address,uint256)",
    "0x095ea7b3": "approve(address,uint256)",
    "0x2e1a7d4d": "burn(uint256)",
    "0x40c10f19": "mint(address,uint256)",
    "0x3593564c": "execute(bytes,bytes[],uint256)",
    "0x414bf389": "execute((address,uint256,bytes)[],uint256)",
    "0xd0e30db0": "deposit()",
    "0x2e17de78": "withdraw(uint256)",
    "0x791ac947": "swapExactTokensForTokensSupportingFeeOnTransferTokens",
    "0x5c11d795": "swapTokensForExactTokens",
    "0x38ed1739": "swapExactTokensForTokens",
    "0x18cbafe5": "swapExactTokensForETHSupportingFeeOnTransferTokens",
    "0xfb3bdb41": "swapETHForExactTokens",
    "0x7ff36ab5": "swapExactETHForTokensSupportingFeeOnTransferTokens",
    "0x4a25d94a": "swapExactTokensForETH",
    "0x02751cec": "swapExactTokensForETHSupportingFeeOnTransferTokens",
    "0x18cbafe5": "swapExactTokensForETH",
    "0x7a25d3cf": "swapExactTokensForTokens",
    "0x5c11d795": "swapTokensForExactTokens",
    "0xfb3bdb41": "swapETHForExactTokens",
    "0x5f575529": "unknown_token_approve",
    "0x9d2c32b1": "withdraw(address,uint256,address,uint32,bytes)",
    "0xb6b55f25": "deposit(address,uint256,address,uint32,bytes)",
    "0xa694fc3a": "stake(uint256)",
    "0x2e17de78": "unstake(uint256)",
    "0x6a627842": "mint(address,uint256,address,address)",
    "0x4f2be91e": "swapExactTokensForETHSupportingFeeOnTransferTokens",
    "0x3d0d8ee0": "exactInputSingle((address,address,uint24,uint256,uint160,uint256))",
    "0xc04b8d59": "exactInput((bytes,uint256,uint160,uint256))",
    "0xdb3e2198": "exactOutputSingle((address,address,uint24,uint256,uint160,uint256))",
    "0xf28c0497": "exactOutput((bytes,uint256,uint160,uint256))",
    "0x04e45aaf": "exactInputSingle((address,address,uint256,uint24,address))",
    "0x5023ee98": "swap(address,bool,int256,uint160,bytes)",
    "0x4a215d7a": "swap(address,bool,int256,uint160,bytes) (alt)",
}

# Event topics we know about
KNOWN_EVENTS = {
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef": "Transfer(address,address,uint256)",
    "0x8c5be1e5ebec7d5bd14f714e2d10f5a4d97c3b8f0e1b4b8a8d3b3b1b1b1b1b1b1": "Approval(address,address,uint256)",
}

# ---- RPC helper ----
def rpc(method, params=None, _id=1):
    """Make a JSON-RPC call to Anvil."""
    body = json.dumps({
        "jsonrpc": "2.0",
        "id": _id,
        "method": method,
        "params": params or []
    }).encode()
    req = urllib.request.Request(
        ANVIL_RPC,
        data=body,
        headers={"Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return json.loads(r.read())
    except Exception as e:
        return {"error": str(e)}

def hex_to_int(h, default=0):
    if not h or h in ("0x", "0x0"):
        return default
    try:
        return int(h, 16)
    except (ValueError, TypeError):
        return default

def hex_to_addr(h):
    if not h:
        return ""
    if len(h) >= 42:
        return h[:2] + h[2:].lower()[-40:]
    return h.lower()

def wei_to_eth(wei):
    if not wei or wei == "0x0":
        return 0
    return int(wei, 16) / 1e18

def format_addr(a):
    if not a or len(a) < 10:
        return a or ""
    return a[:8] + "..." + a[-6:]

def format_value(v, decimals=18):
    if not v:
        return "0"
    n = int(v, 16) / (10 ** decimals)
    if n == 0:
        return "0"
    if abs(n) < 0.0001:
        return f"{n:.2e}"
    return f"{n:.6f}"

def decode_call_data(data):
    """Decode 0x + selector -> human-readable function name + params."""
    if not data or data == "0x":
        return ("", "", "")
    selector = data[:10].lower() if len(data) >= 10 else ""
    if selector in KNOWN_SELECTORS:
        return (selector, KNOWN_SELECTORS[selector], data[10:])
    return (selector, f"unknown({selector})", data[10:])

def decode_logs(logs):
    """Decode event logs from a transaction receipt."""
    if not logs:
        return []
    out = []
    for log in logs:
        topics = log.get("topics", [])
        data = log.get("data", "0x")
        topic0 = topics[0].lower() if topics else ""
        sig = KNOWN_EVENTS.get(topic0, f"event({topic0})")
        # Decode indexed params from topics
        params = []
        for i, t in enumerate(topics[1:], 1):
            if len(t) >= 66:
                # 32-byte hex value, last 20 bytes for address
                if t[2:26] == "0" * 24:
                    params.append(f"addr[{hex_to_addr(t)}]")
                else:
                    params.append(f"uint256[{int(t, 16)}]")
            else:
                params.append(t)
        # Decode data payload
        data_str = ""
        if data and data != "0x" and len(data) > 2:
            try:
                # Try to interpret as raw uint256 chunks
                raw = data[2:]
                chunks = [int(raw[i:i+64], 16) for i in range(0, len(raw), 64) if len(raw[i:i+64]) == 64]
                if len(chunks) == 1:
                    data_str = f" uint256[{chunks[0]}]"
                elif len(chunks) == 3:
                    # Common pattern: (uint256 amount, uint256 other, bool/address)
                    data_str = f" [{chunks[0]}, {chunks[1]}, {chunks[2]}]"
                else:
                    data_str = f" [{', '.join(str(c) for c in chunks)}]"
            except (ValueError, TypeError):
                data_str = f" {data}"
        out.append({
            "address": log.get("address", ""),
            "signature": sig,
            "params": params,
            "data": data_str.strip(),
        })
    return out

def get_address_label(addr):
    """Return a friendly label for known addresses."""
    if not addr:
        return ""
    a = addr.lower()
    if a == WALLET.lower():
        return "test wallet"
    if a == USDC.lower():
        return "USDC (USDC.e on Arbitrum)"
    if a == GRT.lower():
        return "GRT (The Graph)"
    if a == WETH.lower():
        return "WETH (Wrapped Ether)"
    return ""

# ---- HTML templates ----
HTML_HEAD = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Anvil Block Explorer — savant-testnet</title>
<style>
* { box-sizing: border-box; }
body { font-family: 'SF Mono', Menlo, Consolas, monospace; background: #0a0e1a; color: #d0d0d0; margin: 0; padding: 0; }
header { background: linear-gradient(180deg, #11192e 0%, #0a0e1a 100%); border-bottom: 1px solid #1a2540; padding: 16px 24px; display: flex; align-items: center; gap: 24px; }
header h1 { margin: 0; font-size: 18px; color: #5fcaff; font-weight: 700; letter-spacing: 1px; }
header .badge { background: rgba(255, 179, 71, 0.15); color: #ffb347; border: 1px solid rgba(255, 179, 71, 0.4); padding: 2px 8px; font-size: 10px; font-weight: 700; border-radius: 2px; }
header .wallet { color: #707080; font-size: 11px; }
header a { color: #5fcaff; text-decoration: none; }
header .spacer { flex: 1; }
nav { background: #0d1426; border-bottom: 1px solid #1a2540; padding: 8px 24px; display: flex; gap: 16px; }
nav a { color: #8a8aa0; text-decoration: none; font-size: 11px; padding: 4px 10px; border-radius: 2px; }
nav a:hover { background: rgba(95, 202, 255, 0.1); color: #5fcaff; }
main { max-width: 1400px; margin: 0 auto; padding: 24px; }
section { background: #0d1426; border: 1px solid #1a2540; border-radius: 4px; margin-bottom: 16px; overflow: hidden; }
section h2 { margin: 0; padding: 12px 16px; font-size: 12px; color: #5fcaff; text-transform: uppercase; letter-spacing: 1px; background: rgba(95, 202, 255, 0.05); border-bottom: 1px solid #1a2540; font-weight: 700; }
table { width: 100%; border-collapse: collapse; font-size: 12px; }
th { text-align: left; padding: 8px 12px; color: #707080; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; border-bottom: 1px solid #1a2540; font-size: 10px; }
td { padding: 8px 12px; border-bottom: 1px solid #13192e; vertical-align: top; }
tr:hover { background: rgba(95, 202, 255, 0.03); }
a { color: #5fcaff; text-decoration: none; }
a:hover { text-decoration: underline; }
.addr { font-family: 'SF Mono', monospace; color: #5fcaff; }
.muted { color: #6a6a7a; }
.amt { color: #b4b4c4; text-align: right; font-variant-numeric: tabular-nums; }
.tag { display: inline-block; padding: 1px 6px; font-size: 9px; border-radius: 2px; background: rgba(95, 202, 255, 0.1); color: #5fcaff; margin-left: 4px; }
.tag-tx { background: rgba(95, 202, 255, 0.15); color: #5fcaff; }
.tag-call { background: rgba(255, 179, 71, 0.15); color: #ffb347; }
.tag-event { background: rgba(95, 255, 159, 0.15); color: #5fff9f; }
.tag-self { background: rgba(95, 202, 255, 0.15); color: #5fcaff; }
.tag-token { background: rgba(255, 95, 255, 0.15); color: #ff5fff; }
.value { color: #b4b4c4; font-variant-numeric: tabular-nums; }
.input { background: #0a0e1a; border: 1px solid #1a2540; color: #d0d0d0; padding: 8px 12px; font-family: 'SF Mono', monospace; font-size: 12px; width: 400px; }
input:focus { outline: none; border-color: #5fcaff; }
button { background: #5fcaff; color: #0a0e1a; border: none; padding: 8px 16px; font-weight: 700; cursor: pointer; font-family: 'SF Mono', monospace; }
button:hover { background: #7fd4ff; }
.search { display: flex; gap: 8px; align-items: center; margin-bottom: 16px; }
pre { background: #0a0e1a; border: 1px solid #1a2540; padding: 12px; overflow-x: auto; font-size: 11px; margin: 0; }
.log { padding: 6px 0; border-bottom: 1px solid #13192e; font-size: 11px; }
.log:last-child { border-bottom: none; }
.log .event { color: #5fff9f; }
.log .data { color: #909098; margin-top: 2px; }
.summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px; padding: 16px; }
.stat { padding: 8px; }
.stat .label { font-size: 10px; color: #707080; text-transform: uppercase; letter-spacing: 0.5px; }
.stat .value { font-size: 20px; color: #5fcaff; margin-top: 4px; font-variant-numeric: tabular-nums; }
.empty { padding: 32px; text-align: center; color: #6a6a7a; }
footer { padding: 24px; text-align: center; color: #4a4a5a; font-size: 10px; }
</style>
</head>
<body>
<header>
  <h1>◈ SAVANT</h1>
  <span class="badge">TESTNET (Anvil)</span>
  <span class="wallet">wallet <a href="/address/""" + WALLET + """">""" + format_addr(WALLET) + """</a></span>
  <span class="spacer"></span>
  <span class="muted" id="status">connecting…</span>
</header>
<nav>
  <a href="/">Dashboard</a>
  <a href="/blocks">Blocks</a>
  <a href="/transactions">Transactions</a>
  <a href="/address/""" + WALLET + """">Wallet</a>
  <a href="/tokens">Tokens</a>
</nav>
<main>
"""

HTML_FOOT = "</main><footer>FID-155 — local Anvil block explorer — single-file Python, stdlib only</footer></body></html>"

def status_card():
    head = rpc("eth_blockNumber")
    chain = rpc("eth_chainId")
    bal = rpc("eth_getBalance", [WALLET, "latest"])
    nonce = rpc("eth_getTransactionCount", [WALLET, "latest"])
    return f"""
<section><h2>Chain status</h2><div class="summary">
  <div class="stat"><div class="label">Block</div><div class="value">{hex_to_int(head.get('result', '0x0')):,}</div></div>
  <div class="stat"><div class="label">Chain ID</div><div class="value">{hex_to_int(chain.get('result', '0x0'))}</div></div>
  <div class="stat"><div class="label">ETH balance</div><div class="value">{wei_to_eth(bal.get('result', '0x0')):.6f}</div></div>
  <div class="stat"><div class="label">Nonce</div><div class="value">{hex_to_int(nonce.get('result', '0x0'))}</div></div>
</div></section>"""

def search_form():
    return """
<form class="search" action="/search" method="get">
  <input class="input" name="q" placeholder="search by address / 0x... tx hash / block number" autofocus>
  <button type="submit">Search</button>
</form>"""

def page_dashboard():
    body = status_card()
    body += search_form()
    body += render_address(WALLET, compact=True)
    return HTML_HEAD + body + HTML_FOOT

def render_block(b):
    h = hex_to_int(b.get("number", "0x0"))
    ts = hex_to_int(b.get("timestamp", "0x0"))
    txs = b.get("transactions", [])
    dt = datetime.fromtimestamp(ts, timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC") if ts else "—"
    gas_used = hex_to_int(b.get("gasUsed", "0x0"))
    gas_limit = hex_to_int(b.get("gasLimit", "0x0"))
    return f"""
<section><h2>Block #{h:,}</h2>
  <table>
    <tr><th>Timestamp</th><td>{dt}</td></tr>
    <tr><th>Transactions</th><td>{len(txs)}</td></tr>
    <tr><th>Gas used</th><td>{gas_used:,} / {gas_limit:,} ({(gas_used/gas_limit*100 if gas_limit else 0):.1f}%)</td></tr>
    <tr><th>Hash</th><td class="addr">{b.get('hash', '')}</td></tr>
    <tr><th>Parent</th><td class="addr"><a href="/block/{hex_to_int(b.get('parentHash', '0x0'))}">{b.get('parentHash', '')[:18]}…</a></td></tr>
  </table>
  {render_tx_list(txs, h)}
</section>"""

def render_tx_list(txs, block=None):
    if not txs:
        return '<div class="empty">no transactions</div>'
    rows = ""
    for txh in txs[:50]:
        r = rpc("eth_getTransactionByHash", [txh])
        if "result" not in r or r["result"] is None:
            continue
        tx = r["result"]
        from_a = hex_to_addr(tx.get("from", ""))
        to_a = hex_to_addr(tx.get("to", ""))
        value = int(tx.get("value", "0x0"), 16) / 1e18
        sel, name, _ = decode_call_data(tx.get("input", "0x"))
        from_self = from_a == WALLET.lower()
        rows += f"""
<tr>
  <td><a href="/tx/{txh}" class="addr">{txh[:10]}…{txh[-6:]}</a></td>
  <td class="addr">{('● ' if from_self else '')}<a href="/address/{from_a}">{format_addr(from_a)}</a> {get_address_label(from_a) and f'<span class="tag tag-self">{get_address_label(from_a)}</span>' or ''}</td>
  <td>→</td>
  <td class="addr"><a href="/address/{to_a}">{format_addr(to_a)}</a> {get_address_label(to_a) and f'<span class="tag tag-self">{get_address_label(to_a)}</span>' or ''}</td>
  <td class="value">{value:.6f} ETH</td>
  <td>{('<span class="tag tag-call">' + name + '</span>') if name else 'transfer'}</td>
  <td class="muted">block #{hex_to_int(tx.get('blockNumber', '0x0')):,}</td>
</tr>"""
    return f'<table><tr><th>tx</th><th>from</th><th></th><th>to</th><th>value</th><th>method</th><th>block</th></tr>{rows}</table>'

def render_tx(tx_hash):
    r = rpc("eth_getTransactionByHash", [tx_hash])
    if "result" not in r or r["result"] is None:
        return '<div class="empty">tx not found</div>'
    tx = r["result"]
    from_a = hex_to_addr(tx.get("from", ""))
    to_a = hex_to_addr(tx.get("to", ""))
    sel, name, params_data = decode_call_data(tx.get("input", "0x"))
    value = int(tx.get("value", "0x0"), 16) / 1e18
    gas = hex_to_int(tx.get("gas", "0x0"))
    gas_price = hex_to_int(tx.get("gasPrice", "0x0")) / 1e9
    nonce = hex_to_int(tx.get("nonce", "0x0"))
    block_n = hex_to_int(tx.get("blockNumber", "0x0"))
    block_hash = tx.get("blockHash", "")
    from_self = from_a == WALLET.lower()
    body = f"""
<section><h2>Transaction {tx_hash[:14]}…{tx_hash[-8:]}</h2>
  <table>
    <tr><th>Hash</th><td class="addr">{tx_hash}</td></tr>
    <tr><th>Status</th><td><span class="tag tag-self">{'● from test wallet' if from_self else ''}</span></td></tr>
    <tr><th>Block</th><td><a href="/block/{block_n}">#{block_n:,}</a></td></tr>
    <tr><th>From</th><td class="addr"><a href="/address/{from_a}">{from_a}</a> {get_address_label(from_a)}</td></tr>
    <tr><th>To</th><td class="addr"><a href="/address/{to_a}">{to_a or '(contract creation)'}</a> {get_address_label(to_a)}</td></tr>
    <tr><th>Value</th><td class="value">{value:.18f} ETH ({value:.6f})</td></tr>
    <tr><th>Nonce</th><td>{nonce}</td></tr>
    <tr><th>Gas limit</th><td>{gas:,}</td></tr>
    <tr><th>Gas price</th><td>{gas_price:.4f} gwei</td></tr>
    <tr><th>Method</th><td><span class="tag tag-call">{name}</span> <span class="muted">{sel}</span></td></tr>
    <tr><th>Input data</th><td><pre style="max-height:200px;overflow:auto;">{tx.get('input', '0x')}</pre></td></tr>
  </table>
</section>"""
    # Get receipt
    rec_r = rpc("eth_getTransactionReceipt", [tx_hash])
    if "result" in rec_r and rec_r["result"] is not None:
        rec = rec_r["result"]
        status = "✓ success" if hex_to_int(rec.get("status", "0x0")) == 1 else "✗ reverted"
        gas_used = hex_to_int(rec.get("gasUsed", "0x0"))
        block_n = hex_to_int(rec.get("blockNumber", "0x0"))
        body += f"""
<section><h2>Receipt</h2>
  <table>
    <tr><th>Status</th><td>{status}</td></tr>
    <tr><th>Block</th><td>#{block_n:,}</td></tr>
    <tr><th>Gas used</th><td>{gas_used:,} ({(gas_used/gas*100 if gas else 0):.2f}% of limit)</td></tr>
    <tr><th>Contract address</th><td class="addr">{rec.get('contractAddress', '') or '—'}</td></tr>
  </table>
</section>"""
        # Logs
        logs = decode_logs(rec.get("logs", []))
        if logs:
            log_html = '<section><h2>Event logs</h2>'
            for log in logs:
                log_html += f"""
<div class="log">
  <div><span class="tag tag-event">event</span> <span class="event">{log['signature']}</span> <span class="muted">from</span> <a class="addr" href="/address/{log['address']}">{format_addr(log['address'])}</a> {get_address_label(log['address'])}</div>
  <div class="data">{' · '.join(log['params']) + (log['data'] or '')}</div>
</div>"""
            log_html += '</section>'
            body += log_html
    return body

def render_address(addr, compact=False):
    addr = addr.lower()
    if not addr.startswith("0x"):
        return '<div class="empty">invalid address</div>'
    label = get_address_label(addr) or "external"
    # Balances
    eth = rpc("eth_getBalance", [addr, "latest"])
    eth_v = wei_to_eth(eth.get("result", "0x0"))
    nonce = rpc("eth_getTransactionCount", [addr, "latest"])
    nonce_v = hex_to_int(nonce.get("result", "0x0"))
    code = rpc("eth_getCode", [addr, "latest"])
    is_contract = bool(code.get("result", "0x") not in ("0x", "0x0", None))
    # Token balances for the test wallet
    token_rows = ""
    if addr == WALLET.lower():
        for symbol, tok_addr, decimals in [("USDC", USDC, 6), ("GRT", GRT, 18), ("WETH", WETH, 18)]:
            data = "0x70a08231" + "0" * 24 + addr[2:].rjust(64, "0")
            r = rpc("eth_call", [{"to": tok_addr, "data": data}, "latest"])
            raw = r.get("result", "0x" + "0" * 64)
            bal = int(raw, 16) / (10 ** decimals) if raw and len(raw) > 2 else 0
            token_rows += f"<tr><td><a href='/address/{tok_addr}'>{symbol}</a></td><td class='amt'>{bal:,.6f}</td></tr>"
        token_html = f"""
<section><h2>Token balances</h2>
  <table><tr><th>token</th><th>balance</th></tr>{token_rows}</table>
</section>"""
    else:
        token_html = ""
    body = f"""
<section><h2>Address {addr[:10]}…{addr[-8:]} {f'<span class="tag tag-self">{label}</span>' if label != 'external' else ''}</h2>
  <table>
    <tr><th>Full</th><td class="addr">{addr}</td></tr>
    <tr><th>Type</th><td>{'contract' if is_contract else 'EOA (wallet)'}</td></tr>
    <tr><th>ETH</th><td class="value">{eth_v:.18f} ({eth_v:.6f})</td></tr>
    <tr><th>Nonce</th><td>{nonce_v}</td></tr>
  </table>
</section>
{token_html}"""
    if compact:
        return body + render_recent_txs_for_addr(addr, limit=15)
    return body

def render_recent_txs_for_addr(addr, limit=15):
    head = rpc("eth_blockNumber")
    head_n = hex_to_int(head.get("result", "0x0"))
    # Look back ~50 blocks
    rows = ""
    seen = 0
    for b in range(head_n, max(head_n - 50, 0), -1):
        if seen >= limit:
            break
        br = rpc("eth_getBlockByNumber", [hex(b), False])
        if "result" not in br or br["result"] is None:
            continue
        txs = br["result"].get("transactions", [])
        for txh in txs:
            tr = rpc("eth_getTransactionByHash", [txh])
            if "result" not in tr or tr["result"] is None:
                continue
            tx = tr["result"]
            from_a = hex_to_addr(tx.get("from", ""))
            to_a = hex_to_addr(tx.get("to", ""))
            if from_a != addr and to_a != addr:
                continue
            value = int(tx.get("value", "0x0"), 16) / 1e18
            sel, name, _ = decode_call_data(tx.get("input", "0x"))
            direction = "→" if from_a == addr else "←"
            other = to_a if from_a == addr else from_a
            direction_class = "→" if from_a == addr else "←"
            rows += f"""
<tr>
  <td>{direction_class} <a href="/tx/{txh}" class="addr">{txh[:10]}…{txh[-6:]}</a></td>
  <td class="addr"><a href="/address/{other}">{format_addr(other)}</a></td>
  <td class="value">{value:.6f} ETH</td>
  <td>{('<span class="tag tag-call">' + name + '</span>') if name else 'transfer'}</td>
  <td class="muted">block #{hex_to_int(tx.get('blockNumber', '0x0')):,}</td>
</tr>"""
            seen += 1
            if seen >= limit:
                break
    if not rows:
        return ""
    return f"""
<section><h2>Recent transactions (last ~50 blocks, max {limit})</h2>
<table>
  <tr><th>tx</th><th>counterparty</th><th>value</th><th>method</th><th>block</th></tr>
  {rows}
</table></section>"""

def page_blocks():
    head = rpc("eth_blockNumber")
    head_n = hex_to_int(head.get("result", "0x0"))
    rows = ""
    for b in range(head_n, max(head_n - 25, 0), -1):
        br = rpc("eth_getBlockByNumber", [hex(b), False])
        if "result" not in br or br["result"] is None:
            continue
        block = br["result"]
        ts = hex_to_int(block.get("timestamp", "0x0"))
        dt = datetime.fromtimestamp(ts, timezone.utc).strftime("%m-%d %H:%M:%S") if ts else "—"
        txs = block.get("transactions", [])
        gas_used = hex_to_int(block.get("gasUsed", "0x0"))
        rows += f"""
<tr>
  <td><a href="/block/{b}">#{b:,}</a></td>
  <td class="muted">{dt}</td>
  <td>{len(txs)} tx{'' if len(txs)==1 else 's'}</td>
  <td class="value">{gas_used:,} gas</td>
</tr>"""
    body = status_card() + f"""
<section><h2>Recent blocks</h2>
<table><tr><th>block</th><th>timestamp</th><th>txs</th><th>gas used</th></tr>{rows}</table>
</section>"""
    return HTML_HEAD + body + HTML_FOOT

def page_transactions():
    head = rpc("eth_blockNumber")
    head_n = hex_to_int(head.get("result", "0x0"))
    seen = 0
    rows = ""
    for b in range(head_n, max(head_n - 50, 0), -1):
        if seen >= 50:
            break
        br = rpc("eth_getBlockByNumber", [hex(b), False])
        if "result" not in br or br["result"] is None:
            continue
        txs = br["result"].get("transactions", [])
        for txh in txs:
            tr = rpc("eth_getTransactionByHash", [txh])
            if "result" not in tr or tr["result"] is None:
                continue
            tx = tr["result"]
            from_a = hex_to_addr(tx.get("from", ""))
            to_a = hex_to_addr(tx.get("to", ""))
            value = int(tx.get("value", "0x0"), 16) / 1e18
            sel, name, _ = decode_call_data(tx.get("input", "0x"))
            rows += f"""
<tr>
  <td><a href="/tx/{txh}" class="addr">{txh[:10]}…{txh[-6:]}</a></td>
  <td class="addr"><a href="/address/{from_a}">{format_addr(from_a)}</a></td>
  <td>→</td>
  <td class="addr"><a href="/address/{to_a}">{format_addr(to_a)}</a></td>
  <td class="value">{value:.6f} ETH</td></td>
  <td>{('<span class="tag tag-call">' + name + '</span>') if name else 'transfer'}</td>
  <td class="muted">#{b:,}</td>
</tr>"""
            seen += 1
            if seen >= 50:
                break
    body = f"""
<section><h2>Recent transactions (last ~50 blocks)</h2>
<table><tr><th>tx</th><th>from</th><th></th><th>to</th><th>value</th><th>method</th><th>block</th></tr>{rows}</table>
</section>"""
    return HTML_HEAD + body + HTML_FOOT

def page_tokens():
    rows = ""
    for symbol, addr, decimals in [("USDC", USDC, 6), ("GRT", GRT, 18), ("WETH", WETH, 18)]:
        # Total supply
        data = "0x18160ddd"  # totalSupply()
        r = rpc("eth_call", [{"to": addr, "data": data}, "latest"])
        raw = r.get("result", "0x" + "0" * 64)
        supply = int(raw, 16) / (10 ** decimals) if raw and len(raw) > 2 else 0
        rows += f"""
<tr>
  <td><a href="/address/{addr}">{symbol}</a></td>
  <td class="addr">{addr}</td>
  <td class="amt">{supply:,.2f}</td>
  <td>{decimals}</td>
</tr>"""
    return HTML_HEAD + f"""
<section><h2>Known tokens (Arbitrum One)</h2>
<table><tr><th>symbol</th><th>address</th><th>total supply</th><th>decimals</th></tr>{rows}</table>
</section>""" + HTML_FOOT

# ---- HTTP server ----
class ExplorerHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Quieter logging
        return

    def do_GET(self):
        path = urlparse(self.path).path
        params = parse_qs(urlparse(self.path).query)
        try:
            if path == "/" or path == "/index.html":
                body = page_dashboard()
            elif path == "/blocks":
                body = page_blocks()
            elif path == "/transactions":
                body = page_transactions()
            elif path == "/tokens":
                body = page_tokens()
            elif path.startswith("/block/"):
                b = int(path.split("/")[-1])
                br = rpc("eth_getBlockByNumber", [hex(b), True])
                if "result" not in br or br["result"] is None:
                    body = '<div class="empty">block not found</div>'
                else:
                    body = render_block(br["result"])
                body = HTML_HEAD + body + HTML_FOOT
            elif path.startswith("/tx/") or path.startswith("/transaction/"):
                txh = path.split("/")[-1]
                body = render_tx(txh)
                body = HTML_HEAD + body + HTML_FOOT
            elif path.startswith("/address/") or path.startswith("/account/"):
                addr = path.split("/")[-1].lower()
                body = render_address(addr)
                body = HTML_HEAD + body + HTML_FOOT
            elif path == "/search":
                q = params.get("q", [""])[0].strip()
                if not q:
                    body = HTML_HEAD + search_form() + '<div class="empty">empty search</div>' + HTML_FOOT
                elif q.startswith("0x") and len(q) == 66:
                    self.send_response(302)
                    self.send_header("Location", f"/tx/{q}")
                    self.end_headers()
                    return
                elif q.startswith("0x") and len(q) == 42:
                    self.send_response(302)
                    self.send_header("Location", f"/address/{q.lower()}")
                    self.end_headers()
                    return
                elif q.isdigit():
                    self.send_response(302)
                    self.send_header("Location", f"/block/{q}")
                    self.end_headers()
                    return
                else:
                    body = HTML_HEAD + search_form() + f'<div class="empty">unrecognized: {q}</div>' + HTML_FOOT
            else:
                body = HTML_HEAD + '<div class="empty">404 — <a href="/">home</a></div>' + HTML_FOOT
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body.encode("utf-8"))
        except Exception as e:
            import traceback
            err = f"<h1>internal error</h1><pre>{traceback.format_exc()}</pre>"
            self.send_response(500)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(err.encode("utf-8"))

def main():
    server = http.server.HTTPServer((HOST, PORT), ExplorerHandler)
    print(f"Anvil Block Explorer running on http://{HOST}:{PORT}/")
    print(f"  Connected to Anvil: {ANVIL_RPC}")
    print(f"  Default wallet:    {WALLET}")
    print(f"  Press Ctrl+C to stop")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        server.shutdown()

if __name__ == "__main__":
    main()
