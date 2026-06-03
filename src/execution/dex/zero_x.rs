//! 0x Swap API backend for the [`DexBackend`] trait.
//!
//! Wraps the `https://{chain}.api.0x.org/swap/v2/quote` REST endpoint (v2).
//! API key is passed via the `0x-api-key` header.
//!
//! **Reference:** <https://docs.0x.org/>

use async_trait::async_trait;

use super::{DexBackend, Quote, SwapParams, SwapTx};
use crate::core::error::ExecutionError;

/// 0x Swap API client (v2).
pub struct ZeroXBackend {
    api_key: String,
    client: reqwest::Client,
    /// When `Some`, overrides the chain-based URL resolution — used by tests
    /// to point the client at a wiremock server instead of the real 0x API.
    base_url_override: Option<String>,
}

impl ZeroXBackend {
    /// Create a new 0x backend with a default [`reqwest::Client`].
    ///
    /// `api_key` is the 0x API key (obtainable from the
    /// [0x Dashboard](https://dashboard.0x.org/)).
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self::with_client(api_key, client)
    }

    /// Create a new 0x backend with a custom [`reqwest::Client`].
    ///
    /// This variant allows injecting a mock HTTP client in tests via
    /// `wiremock`.  Production code should use [`Self::new`].
    pub fn with_client(api_key: String, client: reqwest::Client) -> Self {
        Self {
            api_key,
            client,
            base_url_override: None,
        }
    }

    /// Create a new 0x backend with custom client and base URL.
    ///
    /// Used by tests to route requests to a wiremock server.
    pub fn with_client_and_url(api_key: String, client: reqwest::Client, base_url: String) -> Self {
        Self {
            api_key,
            client,
            base_url_override: Some(base_url),
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
}

#[async_trait]
impl DexBackend for ZeroXBackend {
    fn name(&self) -> &'static str {
        "0x"
    }

    async fn quote(&self, params: &SwapParams) -> Result<Quote, ExecutionError> {
        let json = self.lookup(params, "quote").await?;

        Ok(Quote {
            to_amount: json["buyAmount"].as_str().unwrap_or("0").to_string(),
            price: json["tokenMetadata"]["buyToken"]["price"]
                .as_str()
                .unwrap_or("0")
                .to_string(),
            guaranteed_price: json["minBuyAmount"].as_str().unwrap_or("0").to_string(),
            estimated_gas: json["transaction"]["gas"].as_u64().unwrap_or(200_000),
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

        Ok(SwapTx {
            to: to.to_string(),
            data: data.to_string(),
            value: value.to_string(),
            gas,
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
                "buyToken": { "price": "0.00002" }
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
                "buyToken": { "price": "0.00002" }
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
        ZeroXBackend::with_client_and_url(
            TEST_API_KEY.into(),
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
            base,
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
        assert_eq!(swap_tx.gas, 250000);
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
