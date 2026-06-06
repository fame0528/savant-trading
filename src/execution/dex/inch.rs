//! 1inch API backend for the [`DexBackend`] trait.
//!
//! Wraps the `https://api.1inch.dev/swap/v6.1/{chain_id}/quote` and
//! `/swap` REST endpoints.  API key is passed via `Authorization: Bearer`.
//!
//! Requires a 1inch Developer Portal account (<https://business.1inch.com/>)
//! — a free tier is available.
//!
//! **Reference:** <https://business.1inch.com/portal/documentation>

use async_trait::async_trait;

use super::{DexBackend, Quote, SwapParams, SwapTx};
use crate::core::error::ExecutionError;

/// 1inch Swap API client (v6.1).
pub struct InchBackend {
    api_key: String,
    client: reqwest::Client,
    /// When `Some`, overrides the chain-based URL resolution — used by tests
    /// to point the client at a wiremock server instead of the real 1inch API.
    base_url_override: Option<String>,
}

impl InchBackend {
    /// Create a new 1inch backend with a default [`reqwest::Client`].
    ///
    /// `api_key` is obtained from the [1inch Developer Portal](https://business.1inch.com/).
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self::with_client(api_key, client)
    }

    /// Create a new 1inch backend with a custom [`reqwest::Client`].
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

    /// Create a new 1inch backend with custom client and base URL.
    ///
    /// Used by tests to route requests to a wiremock server.
    pub fn with_client_and_url(api_key: String, client: reqwest::Client, base_url: String) -> Self {
        Self {
            api_key,
            client,
            base_url_override: Some(base_url),
        }
    }

    fn api_url(&self, chain_id: u64) -> String {
        if let Some(ref override_url) = self.base_url_override {
            return override_url.clone();
        }
        format!("https://api.1inch.dev/swap/v6.1/{}", chain_id)
    }
}

#[async_trait]
impl DexBackend for InchBackend {
    fn name(&self) -> &'static str {
        "1inch"
    }

    async fn quote(&self, params: &SwapParams) -> Result<Quote, ExecutionError> {
        let slippage_pct = params.slippage * 100.0;
        let url = format!(
            "{}/quote?src={}&dst={}&amount={}&slippage={}",
            self.api_url(params.chain_id),
            params.src_token,
            params.dst_token,
            params.amount,
            slippage_pct,
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("1inch quote request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "1inch quote returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("1inch quote parse failed: {}", e)))?;

        // 1inch returns dstAmount as string or number
        let to_amount = json["dstAmount"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| json["dstAmount"].as_i64().map(|v| v.to_string()))
            .ok_or_else(|| ExecutionError::Other("1inch response missing 'dstAmount'".into()))?;

        let src_amount = json["srcAmount"]
            .as_str()
            .ok_or_else(|| ExecutionError::Other("1inch response missing 'srcAmount'".into()))?;

        let est_gas = json["tx"]
            .as_object()
            .and_then(|tx| tx.get("gas").and_then(|v| v.as_u64()))
            .or_else(|| json["estimatedGas"].as_u64())
            .unwrap_or(300_000);

        Ok(Quote {
            to_amount: to_amount.clone(),
            price: src_amount.to_string(),
            guaranteed_price: to_amount,
            estimated_gas: est_gas,
            buy_decimals: 0, // 1inch doesn't return decimals; trader resolves from token DB
        })
    }

    async fn check_liquidity(
        &self,
        params: &SwapParams,
    ) -> Result<super::LiquidityCheck, ExecutionError> {
        match self.quote(params).await {
            Ok(q) => Ok(super::LiquidityCheck {
                available: q.to_amount != "0" && !q.to_amount.is_empty(),
                buy_tax_bps: 0,
                sell_tax_bps: 0,
                buy_amount: q.to_amount,
                balance_ok: true,
                allowance_ok: true,
                price: q.price,
            }),
            Err(_) => Ok(super::LiquidityCheck {
                available: false,
                buy_tax_bps: 0,
                sell_tax_bps: 0,
                buy_amount: "0".to_string(),
                balance_ok: false,
                allowance_ok: false,
                price: "0".to_string(),
            }),
        }
    }

    async fn build_swap_tx(&self, params: &SwapParams) -> Result<SwapTx, ExecutionError> {
        let slippage_pct = params.slippage * 100.0;
        let url = format!(
            "{}/swap?src={}&dst={}&amount={}&from={}&slippage={}",
            self.api_url(params.chain_id),
            params.src_token,
            params.dst_token,
            params.amount,
            params.from,
            slippage_pct,
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("1inch swap request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "1inch swap returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("1inch swap parse failed: {}", e)))?;

        let tx = json["tx"]
            .as_object()
            .ok_or_else(|| ExecutionError::Other("1inch response missing 'tx' object".into()))?;

        // Critical fields — transaction fails on-chain if missing
        // NOTE: `.get()` is safe on Map (returns None for missing keys)
        // whereas `map["key"]` panics on missing keys.
        let to = tx
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExecutionError::Other("1inch response missing 'tx.to'".into()))?;
        let data = tx
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExecutionError::Other("1inch response missing 'tx.data'".into()))?;
        let value = tx
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExecutionError::Other("1inch response missing 'tx.value'".into()))?;

        // Gas may be estimated by the API; fallback to reasonable default
        let gas = tx.get("gas").and_then(|v| v.as_u64()).unwrap_or(300_000);

        let gas_price = tx
            .get("gasPrice")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                tx.get("maxFeePerGas")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
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

    const TEST_API_KEY: &str = "test-1inch-key-00000000";

    fn test_params() -> SwapParams {
        SwapParams {
            src_token: "USDC".into(),
            dst_token: "ETH".into(),
            amount: "50000000".into(),
            slippage: 0.005,
            from: "0x1111111111111111111111111111111111111111".into(),
            chain_id: 42161,
            sell_entire_balance: false,
        }
    }

    fn quote_response_json() -> serde_json::Value {
        serde_json::json!({
            "dstAmount": "10000000000000000",
            "srcAmount": "50000000",
            "estimatedGas": 200000,
            "tx": {
                "gas": 300000,
                "gasPrice": "100000000"
            }
        })
    }

    fn swap_response_json() -> serde_json::Value {
        serde_json::json!({
            "dstAmount": "10000000000000000",
            "srcAmount": "50000000",
            "tx": {
                "to": "0x11111112542d85b3ef69ae05871c7c4e3145c8b8",
                "data": "0xdeadbeef",
                "value": "0",
                "gas": 300000,
                "gasPrice": "100000000"
            }
        })
    }

    fn mk_backend(mock_url: &str) -> InchBackend {
        // Override URL includes the /swap/v6.1/{chain_id} prefix so that
        // downstream URL construction ({base_url}/quote or {base_url}/swap)
        // produces the same relative paths as production.
        let base = format!("{}/swap/v6.1/42161", mock_url.trim_end_matches('/'));
        InchBackend::with_client_and_url(
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
            .and(path("/swap/v6.1/42161/quote"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .respond_with(ResponseTemplate::new(200).set_body_json(quote_response_json()))
            .mount(&mock_server)
            .await;

        let quote = backend.quote(&test_params()).await.unwrap();
        assert_eq!(quote.to_amount, "10000000000000000");
        assert_eq!(quote.price, "50000000");
        assert!(quote.estimated_gas > 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quote_returns_429_error() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        Mock::given(method("GET"))
            .and(path("/swap/v6.1/42161/quote"))
            .respond_with(ResponseTemplate::new(429).set_body_string("rate limit"))
            .mount(&mock_server)
            .await;

        let result = backend.quote(&test_params()).await;
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("429"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quote_returns_500_error() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        Mock::given(method("GET"))
            .and(path("/swap/v6.1/42161/quote"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&mock_server)
            .await;

        let result = backend.quote(&test_params()).await;
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("500"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quote_malformed_json_missing_dst_amount() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        let bad_json = serde_json::json!({
            "srcAmount": "50000000",
            "estimatedGas": 200000
        });

        Mock::given(method("GET"))
            .and(path("/swap/v6.1/42161/quote"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bad_json))
            .mount(&mock_server)
            .await;

        let result = backend.quote(&test_params()).await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("dstAmount") || err_str.contains("missing"),
            "Expected error about missing dstAmount, got: {}",
            err_str
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_swap_tx_happy_path() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        Mock::given(method("GET"))
            .and(path("/swap/v6.1/42161/swap"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .respond_with(ResponseTemplate::new(200).set_body_json(swap_response_json()))
            .mount(&mock_server)
            .await;

        let swap_tx = backend.build_swap_tx(&test_params()).await.unwrap();
        assert_eq!(swap_tx.to, "0x11111112542d85b3ef69ae05871c7c4e3145c8b8");
        assert_eq!(swap_tx.data, "0xdeadbeef");
        assert_eq!(swap_tx.value, "0");
        assert!(swap_tx.gas > 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_swap_tx_missing_tx_to_field() {
        let mock_server = MockServer::start().await;
        let backend = mk_backend(&mock_server.uri());

        let bad_json = serde_json::json!({
            "dstAmount": "10000000000000000",
            "srcAmount": "50000000",
            "tx": {
                "data": "0xdeadbeef",
                "value": "0",
                "gas": 300000
            }
        });

        Mock::given(method("GET"))
            .and(path("/swap/v6.1/42161/swap"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bad_json))
            .mount(&mock_server)
            .await;

        let result = backend.build_swap_tx(&test_params()).await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("tx.to") || err_str.contains("missing"),
            "Expected error about missing 'tx.to', got: {}",
            err_str
        );
    }
}
