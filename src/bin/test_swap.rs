//! Dry-run swap test — verifies Permit2 signing and eth_call without broadcasting.
//!
//! Usage: cargo run --bin test_swap

use savant_trading::execution::dex::zero_x::ZeroXBackend;
use savant_trading::execution::dex::{SwapParams, DexBackend};
use alloy_core::primitives::Address;
use alloy_core::hex;
use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ZEROEX_API_KEY")?;
    let wallet_key = std::env::var("WALLET_PRIVATE_KEY")?;
    let rpc_url = std::env::var("ARBITRUM_RPC_URL")
        .unwrap_or_else(|_| "https://arb1.arbitrum.io/rpc".to_string());

    // Parse signing key
    let key_hex = wallet_key.trim_start_matches("0x");
    let key_bytes = hex::decode(key_hex)?;
    let signing_key = SigningKey::from_bytes(key_bytes.as_slice().into())?;

    // Derive wallet address
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false).to_bytes().to_vec();
    let hash = Keccak256::digest(&encoded[1..]);
    let addr_bytes: [u8; 20] = hash[12..32].try_into()?;
    let wallet_address = Address::from(addr_bytes);

    println!("Wallet: {:#x}", wallet_address);
    println!("RPC: {}", rpc_url);

    // Create 0x backend
    let backend = ZeroXBackend::new(api_key, signing_key);

    // Test swap: $5 USDC → ETH
    let usdc_address = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
    let weth_address = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1";
    let amount_wei = "5000000"; // $5 USDC (6 decimals)

    let params = SwapParams {
        src_token: usdc_address.to_string(),
        dst_token: weth_address.to_string(),
        amount: amount_wei.to_string(),
        slippage: 0.005,
        from: format!("{:#x}", wallet_address),
        chain_id: 42161,
        sell_entire_balance: false,
    };

    println!("\n=== STEP 1: Quote ===");
    match backend.quote(&params).await {
        Ok(quote) => {
            println!("Quote OK:");
            println!("  buy_amount: {}", quote.to_amount);
            println!("  price: {}", quote.price);
            println!("  estimated_gas: {}", quote.estimated_gas);
            println!("  buy_decimals: {}", quote.buy_decimals);
        }
        Err(e) => {
            println!("Quote FAILED: {}", e);
            return Err(e.into());
        }
    }

    println!("\n=== STEP 2: Build Swap TX ===");
    match backend.build_swap_tx(&params).await {
        Ok(swap_tx) => {
            println!("Build OK:");
            println!("  to: {}", swap_tx.to);
            println!("  data_len: {} bytes", swap_tx.data.len());
            println!("  value: {}", swap_tx.value);
            println!("  gas: {}", swap_tx.gas);
            println!("  gas_price: {}", swap_tx.gas_price);

            // Decode calldata to check Permit2 signature
            let data = swap_tx.data.trim_start_matches("0x");
            let data_bytes = hex::decode(data)?;
            println!("  calldata_bytes: {} bytes", data_bytes.len());
            println!("  calldata_first_4: {}", hex::encode(&data_bytes[..4]));
            
            // Last 65 bytes should be the signature
            if data_bytes.len() > 65 {
                let sig_start = data_bytes.len() - 65;
                let r = &data_bytes[sig_start..sig_start+32];
                let s = &data_bytes[sig_start+32..sig_start+64];
                let v = data_bytes[sig_start+64];
                println!("  signature_r: 0x{}", hex::encode(r));
                println!("  signature_s: 0x{}", hex::encode(s));
                println!("  signature_v: {}", v);
                println!("  calldata_without_sig: {} bytes", sig_start);
            }
        }
        Err(e) => {
            println!("Build FAILED: {}", e);
            return Err(e.into());
        }
    }

    println!("\n=== STEP 3: eth_call dry-run ===");
    let client = reqwest::Client::new();

    // Get the swap tx data
    let swap_tx = backend.build_swap_tx(&params).await?;
    let data_hex = if swap_tx.data.starts_with("0x") {
        swap_tx.data.clone()
    } else {
        format!("0x{}", swap_tx.data)
    };

    // Also try without the last 65 bytes (Permit2 signature)
    let data_bytes = hex::decode(swap_tx.data.trim_start_matches("0x"))?;
    let data_without_sig = if data_bytes.len() > 65 {
        hex::encode(&data_bytes[..data_bytes.len()-65])
    } else {
        hex::encode(&data_bytes)
    };
    let data_without_sig_hex = format!("0x{}", data_without_sig);

    // Format value with 0x prefix
    let value_hex = if swap_tx.value.starts_with("0x") {
        swap_tx.value.clone()
    } else {
        format!("0x{}", swap_tx.value)
    };

    // Test 1: eth_call WITH signature
    println!("\n=== STEP 3a: eth_call WITH Permit2 signature ===");
    let call_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "from": format!("{:#x}", wallet_address),
            "to": swap_tx.to,
            "data": data_hex,
            "value": value_hex,
            "gas": "0x1000000"
        }, "latest"],
        "id": 1
    });

    let resp = client.post(&rpc_url)
        .json(&call_body)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    if let Some(error) = json.get("error") {
        println!("WITH sig FAILED: {}", error);
    } else {
        let result = json["result"].as_str().unwrap_or("0x");
        println!("WITH sig OK: {}", result);
    }

    // Test 2: eth_call WITHOUT signature
    println!("\n=== STEP 3b: eth_call WITHOUT Permit2 signature ===");
    let call_body2 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "from": format!("{:#x}", wallet_address),
            "to": swap_tx.to,
            "data": data_without_sig_hex,
            "value": value_hex,
            "gas": "0x1000000"
        }, "latest"],
        "id": 2
    });

    let resp2 = client.post(&rpc_url)
        .json(&call_body2)
        .send()
        .await?;

    let json2: serde_json::Value = resp2.json().await?;

    if let Some(error) = json2.get("error") {
        println!("WITHOUT sig FAILED: {}", error);
    } else {
        let result = json2["result"].as_str().unwrap_or("0x");
        println!("WITHOUT sig OK: {}", result);
    }

    println!("\n=== SUMMARY ===");
    println!("If eth_call succeeded, the Permit2 signing is correct.");
    println!("If it failed, check:");
    println!("  1. Permit2 signature encoding (r, s, v values)");
    println!("  2. Signature appended to calldata correctly");
    println!("  3. USDC Permit2 approval on-chain");

    Ok(())
}
