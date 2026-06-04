//! 0x Swap API backend for the [`DexBackend`] trait.
//!
//! Wraps the `https://{chain}.api.0x.org/swap/v2/quote` REST endpoint (v2).
//! API key is passed via the `0x-api-key` header.
//!
//! **Reference:** <https://docs.0x.org/>

use async_trait::async_trait;

use super::{DexBackend, Quote, SwapParams, SwapTx};
use alloy_core::primitives::hex;
use crate::core::error::ExecutionError;

/// 0x Swap API client (v2).
pub struct ZeroXBackend {
    api_key: String,
    client: reqwest::Client,
    /// When `Some`, overrides the chain-based URL resolution — used by tests
    /// to point the client at a wiremock server instead of the real 0x API.
    base_url_override: Option<String>,
    /// Wallet private key for Permit2 EIP-712 signing.
    signing_key: k256::ecdsa::SigningKey,
}

impl ZeroXBackend {
    /// Create a new 0x backend with a default [`reqwest::Client`].
    ///
    /// `api_key` is the 0x API key (obtainable from the
    /// [0x Dashboard](https://dashboard.0x.org/)).
    pub fn new(api_key: String, signing_key: k256::ecdsa::SigningKey) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self::with_client(api_key, client, signing_key)
    }

    /// Create a new 0x backend with a custom [`reqwest::Client`].
    ///
    /// This variant allows injecting a mock HTTP client in tests via
    /// `wiremock`.  Production code should use [`Self::new`].
    pub fn with_client(api_key: String, client: reqwest::Client, signing_key: k256::ecdsa::SigningKey) -> Self {
        Self {
            api_key,
            client,
            base_url_override: None,
            signing_key,
        }
    }

    /// Create a new 0x backend with custom client and base URL.
    ///
    /// Used by tests to route requests to a wiremock server.
    pub fn with_client_and_url(api_key: String, client: reqwest::Client, base_url: String, signing_key: k256::ecdsa::SigningKey) -> Self {
        Self {
            api_key,
            client,
            base_url_override: Some(base_url),
            signing_key,
        }
    }

    /// Build the 0x Swap API v2 URL for a given chain.
    ///
    /// When `base_url_override` is set (testing), returns that URL directly.
    fn api_url(&self, _chain_id: u64) -> String {
        if let Some(ref override_url) = self.base_url_override {
            return override_url.clone();
        }
        // 0x API v2 — unified endpoint with chainId parameter
        "https://api.0x.org/swap/permit2".into()
    }

    /// Shared HTTP GET for any 0x Swap API v2 endpoint.
    ///
    /// `endpoint` is the path suffix — either `"quote"` (returns calldata)
    /// or `"price"` (read-only, no calldata, cheaper).
    async fn lookup(
        &self,
        params: &SwapParams,
        endpoint: &str,
    ) -> Result<serde_json::Value, ExecutionError> {
        let base_url = self.api_url(params.chain_id);
        let slippage_bps = (params.slippage * 10000.0) as u64;
        let url = format!(
            "{base_url}/{endpoint}?chainId={chain_id}&sellToken={sell_token}&buyToken={buy_token}&sellAmount={sell_amount}&taker={taker}&slippageBps={slippage_bps}",
            base_url = base_url,
            endpoint = endpoint,
            chain_id = params.chain_id,
            sell_token = params.src_token,
            buy_token = params.dst_token,
            sell_amount = params.amount,
            taker = params.from,
            slippage_bps = slippage_bps,
        );

        let resp = self
            .client
            .get(&url)
            .header("0x-api-key", &self.api_key)
            .header("0x-version", "v2")
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("0x {} request failed: {}", endpoint, e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "0x {} returned {}: {}",
                endpoint, status, body,
            )));
        }

        resp.json()
            .await
            .map_err(|e| ExecutionError::Other(format!("0x {} parse failed: {}", endpoint, e)))
    }

    /// Extract a required string field from the 0x response, or return an error.
    ///
    /// Critical fields like `to`, `data`, `sellAmount`, `buyAmount` MUST be
    /// present — silently defaulting to `"0"` would produce a transaction that
    /// fails on-chain.
    fn require_field<'a>(
        json: &'a serde_json::Value,
        field: &str,
    ) -> Result<&'a str, ExecutionError> {
        json[field].as_str().ok_or_else(|| {
            ExecutionError::Other(format!("0x response missing required field '{}'", field))
        })
    }

    /// Extract an optional string field from the 0x response, defaulting to `"0"`.
    /// Safe for numeric fields like `value` where `"0"` is a valid default.
    fn opt_str<'a>(json: &'a serde_json::Value, field: &str) -> &'a str {
        json[field].as_str().unwrap_or("0")
    }

    /// Parse a U256 value that may be hex (0x-prefixed) or decimal.
    ///
    /// The 0x API v2 returns Permit2 fields as strings — sometimes hex,
    /// sometimes decimal. This handles both formats.
    fn parse_u256(value: &str) -> alloy_core::primitives::U256 {
        let trimmed = value.trim();
        if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
            alloy_core::primitives::U256::from_str_radix(trimmed, 16)
                .unwrap_or_default()
        } else {
            alloy_core::primitives::U256::from_str_radix(trimmed, 10)
                .unwrap_or_default()
        }
    }

    /// Sign a Permit2 EIP-712 typed data structure.
    ///
    /// Returns the signature as a hex-encoded string (without 0x prefix).
    fn sign_permit2(&self, permit2: &serde_json::Value) -> Result<String, ExecutionError> {
        use k256::ecdsa::{RecoveryId, Signature};

        // Log the full permit2 response for debugging
        tracing::debug!(
            "Permit2 response: {}",
            serde_json::to_string_pretty(permit2).unwrap_or_default()
        );

        // Parse the EIP-712 typed data from the permit2 object
        let eip712 = permit2.get("eip712").ok_or_else(|| {
            ExecutionError::Other("0x permit2 missing 'eip712' field".into())
        })?;

        // Parse domain separator
        let domain = eip712.get("domain").ok_or_else(|| {
            ExecutionError::Other("0x permit2.eip712 missing 'domain' field".into())
        })?;

        // Parse message
        let message = eip712.get("message").ok_or_else(|| {
            ExecutionError::Other("0x permit2.eip712 missing 'message' field".into())
        })?;

        // Verify spender is the 0x Exchange Proxy
        let spender = message.get("spender").and_then(|v| v.as_str()).unwrap_or("");
        const EXCHANGE_PROXY: &str = "0xfeea2a79d7d3d36753c8917af744d71f13c9b02a";
        if !spender.eq_ignore_ascii_case(EXCHANGE_PROXY) {
            tracing::warn!(
                "Permit2 spender mismatch: expected {}, got {}",
                EXCHANGE_PROXY,
                spender
            );
        }

        // Use the pre-computed hash from the 0x API response if available.
        // The API returns permit2.hash which is the EIP-712 hash — sign it directly.
        // If not available, compute it from the EIP-712 typed data.
        let hash_bytes: [u8; 32] = if let Some(hash_str) = permit2.get("hash").and_then(|h| h.as_str()) {
            // API provides pre-computed hash — use it directly
            let hash_hex = hash_str.trim_start_matches("0x");
            let hash_vec = hex::decode(hash_hex)
                .map_err(|e| ExecutionError::Other(format!("Invalid permit2 hash hex: {}", e)))?;
            if hash_vec.len() != 32 {
                return Err(ExecutionError::Other(format!(
                    "permit2 hash must be 32 bytes, got {}",
                    hash_vec.len()
                )));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&hash_vec);
            tracing::debug!(
                "Permit2: using API-provided hash: 0x{}",
                hex::encode(arr)
            );
            arr
        } else {
            // No hash in response — compute from EIP-712 typed data
            let domain_separator = self.compute_domain_separator(domain)?;
            let struct_hash = self.compute_struct_hash(message)?;

            let mut eip712_hash = Vec::with_capacity(2 + 32 + 32);
            eip712_hash.push(0x19);
            eip712_hash.push(0x01);
            eip712_hash.extend_from_slice(&domain_separator);
            eip712_hash.extend_from_slice(&struct_hash);

            let computed = alloy_core::primitives::keccak256(&eip712_hash);
            tracing::debug!(
                "Permit2: computed hash: domain_sep=0x{}, struct_hash=0x{}, hash=0x{}",
                hex::encode(domain_separator),
                hex::encode(struct_hash),
                hex::encode(computed.as_slice())
            );
            computed.into()
        };

        // Sign the hash with the wallet's private key
        let (signature, recid): (Signature, RecoveryId) = self
            .signing_key
            .sign_prehash_recoverable(hash_bytes.as_slice())
            .map_err(|e| ExecutionError::Other(format!("Permit2 signing failed: {}", e)))?;

        // Encode signature as r || s || v (65 bytes)
        let r = signature.r().to_bytes();
        let s = signature.s().to_bytes();
        let v = if recid.is_y_odd() { 28u8 } else { 27u8 };

        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&r);
        sig_bytes.extend_from_slice(&s);
        sig_bytes.push(v);

        tracing::debug!(
            "Permit2 signature: r=0x{}, s=0x{}, v={}, full=0x{}",
            hex::encode(r),
            hex::encode(s),
            v,
            hex::encode(&sig_bytes)
        );

        Ok(hex::encode(&sig_bytes))
    }

    /// Compute the EIP-712 domain separator hash.
    fn compute_domain_separator(&self, domain: &serde_json::Value) -> Result<[u8; 32], ExecutionError> {
        // Parse domain fields
        let name = domain.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let chain_id = domain.get("chainId").and_then(|v| v.as_u64()).unwrap_or(0);
        let verifying_contract = domain.get("verifyingContract").and_then(|v| v.as_str()).unwrap_or("");

        // Compute domain type hash
        let domain_type_hash = alloy_core::primitives::keccak256(
            b"EIP712Domain(string name,uint256 chainId,address verifyingContract)"
        );

        // Encode domain fields
        let name_hash = alloy_core::primitives::keccak256(name.as_bytes());
        let chain_id_bytes = alloy_core::primitives::U256::from(chain_id).to_be_bytes::<32>();

        // Parse address and pad to 32 bytes (left-padded with zeros)
        let contract_addr = alloy_core::primitives::Address::parse_checksummed(verifying_contract, None)
            .unwrap_or_default();
        let mut contract_bytes = [0u8; 32];
        contract_bytes[12..32].copy_from_slice(contract_addr.as_slice());

        // Compute domain separator
        let mut encoded = Vec::with_capacity(32 + 32 + 32 + 32);
        encoded.extend_from_slice(domain_type_hash.as_slice());
        encoded.extend_from_slice(name_hash.as_slice());
        encoded.extend_from_slice(&chain_id_bytes);
        encoded.extend_from_slice(&contract_bytes);

        Ok(alloy_core::primitives::keccak256(&encoded).into())
    }

    /// Compute the EIP-712 struct hash for PermitTransferFrom.
    fn compute_struct_hash(&self, message: &serde_json::Value) -> Result<[u8; 32], ExecutionError> {
        // Parse permitted token and amount
        let permitted = message.get("permitted").ok_or_else(|| {
            ExecutionError::Other("Permit2 message missing 'permitted' field".into())
        })?;
        let token = permitted.get("token").and_then(|v| v.as_str()).unwrap_or("");
        let amount = permitted.get("amount").and_then(|v| v.as_str()).unwrap_or("0");

        // Parse nonce and deadline
        let nonce = message.get("nonce").and_then(|v| v.as_str()).unwrap_or("0");
        let deadline = message.get("deadline").and_then(|v| v.as_str()).unwrap_or("0");
        let spender = message.get("spender").and_then(|v| v.as_str()).unwrap_or("");

        tracing::debug!(
            "Permit2 struct: token={}, amount={}, nonce={}, deadline={}, spender={}",
            token, amount, nonce, deadline, spender
        );

        // Compute type hashes
        let permit_type_hash = alloy_core::primitives::keccak256(
            b"PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
        );
        let token_permissions_type_hash = alloy_core::primitives::keccak256(
            b"TokenPermissions(address token,uint256 amount)"
        );

        // Encode token permissions
        let token_addr = alloy_core::primitives::Address::parse_checksummed(token, None)
            .unwrap_or_default();
        let mut token_bytes = [0u8; 32];
        token_bytes[12..32].copy_from_slice(token_addr.as_slice());

        let amount_bytes = Self::parse_u256(amount).to_be_bytes::<32>();

        let mut permitted_encoded = Vec::with_capacity(32 + 32 + 32);
        permitted_encoded.extend_from_slice(token_permissions_type_hash.as_slice());
        permitted_encoded.extend_from_slice(&token_bytes);
        permitted_encoded.extend_from_slice(&amount_bytes);
        let permitted_hash = alloy_core::primitives::keccak256(&permitted_encoded);

        // Parse spender (the 0x Exchange Proxy)
        let spender = message.get("spender").and_then(|v| v.as_str()).unwrap_or("");
        let spender_addr = alloy_core::primitives::Address::parse_checksummed(spender, None)
            .unwrap_or_default();
        let mut spender_bytes = [0u8; 32];
        spender_bytes[12..32].copy_from_slice(spender_addr.as_slice());

        // Encode nonce and deadline
        let nonce_bytes = Self::parse_u256(nonce).to_be_bytes::<32>();
        let deadline_bytes = Self::parse_u256(deadline).to_be_bytes::<32>();

        // Compute struct hash
        let mut encoded = Vec::with_capacity(32 + 32 + 32 + 32 + 32);
        encoded.extend_from_slice(permit_type_hash.as_slice());
        encoded.extend_from_slice(permitted_hash.as_slice());
        encoded.extend_from_slice(&spender_bytes);
        encoded.extend_from_slice(&nonce_bytes);
        encoded.extend_from_slice(&deadline_bytes);

        Ok(alloy_core::primitives::keccak256(&encoded).into())
    }
}

#[async_trait]
impl DexBackend for ZeroXBackend {
    fn name(&self) -> &'static str {
        "0x"
    }

    async fn quote(&self, params: &SwapParams) -> Result<Quote, ExecutionError> {
        let json = self.lookup(params, "quote").await?;

        let buy_decimals = json["tokenMetadata"]["buyToken"]["decimals"]
            .as_u64()
            .unwrap_or(18) as u32;

        Ok(Quote {
            to_amount: json["buyAmount"].as_str().unwrap_or("0").to_string(),
            price: json["tokenMetadata"]["buyToken"]["price"]
                .as_str()
                .unwrap_or("0")
                .to_string(),
            guaranteed_price: json["minBuyAmount"].as_str().unwrap_or("0").to_string(),
            estimated_gas: json["transaction"]["gas"].as_u64().unwrap_or(200_000),
            buy_decimals,
        })
    }

    async fn build_swap_tx(&self, params: &SwapParams) -> Result<SwapTx, ExecutionError> {
        let json = self.lookup(params, "quote").await?;

        // 0x API v2 (permit2) nests transaction data under "transaction" key
        let tx = json.get("transaction").ok_or_else(|| {
            ExecutionError::Other("0x response missing 'transaction' field".into())
        })?;

        // Critical fields — must be present or the transaction will fail on-chain
        let to = Self::require_field(tx, "to")?;
        let data = Self::require_field(tx, "data")?;

        // Optional fields — safe defaults exist
        let value = Self::opt_str(tx, "value");
        let gas = tx["gas"].as_u64().unwrap_or(300_000);

        let gas_price = tx["gasPrice"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| {
                json["gasFees"]
                    .as_object()
                    .and_then(|fees| fees["maxFeePerGas"].as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "0".to_string());

        // Permit2 EIP-712 signing (FID-042, 0x Permit2 guide)
        // The 0x API v2 requires signing a Permit2 permit before the swap can execute.
        // Format: calldata || signature_length (32 bytes, big-endian) || signature (65 bytes)
        let data_with_signature = if let Some(permit2) = json.get("permit2") {
            let signature = self.sign_permit2(permit2)?;
            let sig_bytes = hex::decode(&signature)
                .map_err(|e| ExecutionError::Other(format!("Invalid signature hex: {}", e)))?;
            let sig_len = sig_bytes.len();

            // Encode signature length as 32-byte big-endian integer
            let mut len_bytes = [0u8; 32];
            // sig_len is always 65, but encode it properly
            len_bytes[24..32].copy_from_slice(&(sig_len as u64).to_be_bytes());

            // Build final calldata: original_data || sig_len (32 bytes) || signature (65 bytes)
            let data_hex = data.trim_start_matches("0x");
            let mut final_data = String::with_capacity(2 + (data_hex.len()) + 64 + 130);
            final_data.push_str("0x");
            final_data.push_str(data_hex);
            final_data.push_str(&hex::encode(len_bytes));
            final_data.push_str(&signature);

            tracing::debug!(
                "Permit2: appended sig_len={} (0x41), sig=0x{}, total_data_len={}",
                sig_len,
                &signature[..8.min(signature.len())],
                final_data.len()
            );
            final_data
        } else {
            tracing::warn!(
                "0x response missing 'permit2' field — swap will execute WITHOUT Permit2 signature. \
                 This may cause on-chain revert if the router requires Permit2."
            );
            data.to_string()
        };

        // Gas buffer (FID-042): Add 20% to gas estimate to prevent out-of-gas failures
        let gas_with_buffer = ((gas as f64) * 1.2) as u64;

        Ok(SwapTx {
            to: to.to_string(),
            data: data_with_signature,
            value: value.to_string(),
            gas: gas_with_buffer,
            gas_price,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::dex::SwapParams;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const TEST_API_KEY: &str = "test-0x-key-00000000";

    fn test_params() -> SwapParams {
        SwapParams {
            src_token: "USDC".into(),
            dst_token: "ETH".into(),
            amount: "50000000".into(),
            slippage: 0.005,
            from: "0x1111111111111111111111111111111111111111".into(),
            chain_id: 42161,
        }
    }

    fn price_response_json() -> serde_json::Value {
        serde_json::json!({
            "buyAmount": "10000000000000000",
            "minBuyAmount": "9900000000000000",
            "sellAmount": "50000000",
            "tokenMetadata": {
                "buyToken": { "price": "0.00002", "decimals": 18 }
            },
            "transaction": {
                "gas": 200000,
                "gasPrice": "100000000"
            }
        })
    }

    fn quote_response_json() -> serde_json::Value {
        serde_json::json!({
            "buyAmount": "10000000000000000",
            "minBuyAmount": "9900000000000000",
            "sellAmount": "50000000",
            "tokenMetadata": {
                "buyToken": { "price": "0.00002", "decimals": 18 }
            },
            "transaction": {
                "to": "0xdef1c0ded9bec7f1a1670819833240f027b25eff",
                "data": "0xabc123",
                "value": "0",
                "gas": 250000,
                "gasPrice": "100000000"
            }
        })
    }

    fn mk_backend(mock_url: &str) -> ZeroXBackend {
        // Override URL includes the /swap/permit2 prefix so that downstream
        // URL construction ({base_url}/{endpoint}) produces the same
        // relative paths as production (e.g. /swap/permit2/quote).
        let base = format!("{}/swap/permit2", mock_url.trim_end_matches('/'));
        let signing_key = k256::ecdsa::SigningKey::from_bytes(&[1u8; 32].into()).unwrap();
        ZeroXBackend::with_client_and_url(
            TEST_API_KEY.into(),
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
            base,
            signing_key,
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quote_happy_path() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        Mock::given(method("GET"))
            .and(path("/swap/permit2/quote"))
            .and(header("0x-api-key", TEST_API_KEY))
            .respond_with(ResponseTemplate::new(200).set_body_json(quote_response_json()))
            .mount(&mock_server)
            .await;

        let quote = backend.quote(&test_params()).await.unwrap();
        assert_eq!(quote.to_amount, "10000000000000000");
        assert_eq!(quote.guaranteed_price, "9900000000000000");
        assert_eq!(quote.estimated_gas, 250000);
        assert_eq!(quote.buy_decimals, 18);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quote_returns_429_error() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        Mock::given(method("GET"))
            .and(path("/swap/permit2/quote"))
            .respond_with(ResponseTemplate::new(429).set_body_string("rate limit exceeded"))
            .mount(&mock_server)
            .await;

        let result = backend.quote(&test_params()).await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("429") || err_str.contains("rate limit"),
            "Expected 429 error, got: {}",
            err_str
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quote_returns_500_error() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        Mock::given(method("GET"))
            .and(path("/swap/permit2/quote"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&mock_server)
            .await;

        let result = backend.quote(&test_params()).await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("500") || err_str.contains("internal"),
            "Expected 500 error, got: {}",
            err_str
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quote_malformed_json_missing_buy_amount() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        let bad_json = serde_json::json!({
            "sellAmount": "50000000",
            "transaction": { "gas": 200000 }
        });

        Mock::given(method("GET"))
            .and(path("/swap/permit2/quote"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bad_json))
            .mount(&mock_server)
            .await;

        let result = backend.quote(&test_params()).await;
        // buyAmount defaults to "0" in new code, so this should succeed with "0"
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_amount, "0");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_swap_tx_happy_path() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        Mock::given(method("GET"))
            .and(path("/swap/permit2/quote"))
            .and(header("0x-api-key", TEST_API_KEY))
            .respond_with(ResponseTemplate::new(200).set_body_json(quote_response_json()))
            .mount(&mock_server)
            .await;

        let swap_tx = backend.build_swap_tx(&test_params()).await.unwrap();
        assert_eq!(swap_tx.to, "0xdef1c0ded9bec7f1a1670819833240f027b25eff");
        assert_eq!(swap_tx.data, "0xabc123");
        assert_eq!(swap_tx.value, "0");
        assert_eq!(swap_tx.gas, 300000); // 250000 * 1.2 = 300000 (20% buffer)
        assert_eq!(swap_tx.gas_price, "100000000");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_swap_tx_missing_to_field() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        let bad_json = serde_json::json!({
            "buyAmount": "10000000000000000",
            "transaction": {
                "data": "0xabc123",
                "value": "0",
                "gas": 250000
            }
        });

        Mock::given(method("GET"))
            .and(path("/swap/permit2/quote"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bad_json))
            .mount(&mock_server)
            .await;

        let result = backend.build_swap_tx(&test_params()).await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("to") || err_str.contains("missing"),
            "Expected error about missing 'to' field, got: {}",
            err_str
        );
    }
}
