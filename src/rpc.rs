use crate::{
    error::{Result, SolverRelayError},
    types::{
        GetStatusParams, IntentStatus, JsonRpcRequest, JsonRpcResponse, PublishIntentParams,
        PublishIntentResult, PublishStatus, Quote, QuoteParams,
    },
};
use tracing::{debug, error};

const DEFAULT_RPC_URL: &str = "https://solver-relay-v2.chaindefuser.com/rpc";

/// HTTP RPC client for the NEAR Intents Solver Relay
#[derive(Debug, Clone)]
pub struct SolverRelayRpcClient {
    client: reqwest::Client,
    url: String,
}

impl Default for SolverRelayRpcClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SolverRelayRpcClient {
    /// Create a new client with the default endpoint
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            url: DEFAULT_RPC_URL.to_string(),
        }
    }

    /// Create a new client with a custom endpoint
    pub fn with_url(url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
        }
    }

    /// Get the RPC endpoint URL
    pub fn url(&self) -> &str {
        &self.url
    }

    async fn call<P, R>(&self, method: &str, params: Option<Vec<P>>) -> Result<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let request = JsonRpcRequest::new(method, params);
        debug!("RPC request: {}", serde_json::to_string(&request)?);

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await?
            .json::<JsonRpcResponse<R>>()
            .await?;

        if let Some(error) = response.error {
            error!("RPC error: {:?}", error);
            return Err(SolverRelayError::RpcError {
                code: error.code,
                message: error.message,
                data: error.data,
            });
        }

        response
            .result
            .ok_or_else(|| SolverRelayError::InvalidResponse("missing result".to_string()))
    }

    /// Request quotes for a token swap
    ///
    /// # Arguments
    /// * `asset_in` - Source asset identifier (e.g., "nep141:wrap.near")
    /// * `asset_out` - Target asset identifier (e.g., "nep141:usdt.tether-token.near")
    /// * `exact_amount_in` - Exact input amount (mutually exclusive with exact_amount_out)
    /// * `exact_amount_out` - Exact output amount (mutually exclusive with exact_amount_in)
    /// * `min_deadline_ms` - Minimum validity window in milliseconds (default: 60000)
    pub async fn quote(
        &self,
        asset_in: &str,
        asset_out: &str,
        exact_amount_in: Option<&str>,
        exact_amount_out: Option<&str>,
        min_deadline_ms: Option<u64>,
    ) -> Result<Vec<Quote>> {
        let params = QuoteParams {
            defuse_asset_identifier_in: asset_in.to_string(),
            defuse_asset_identifier_out: asset_out.to_string(),
            exact_amount_in: exact_amount_in.map(String::from),
            exact_amount_out: exact_amount_out.map(String::from),
            min_deadline_ms,
        };

        self.call("quote", Some(vec![params])).await
    }

    /// Publish a signed intent for execution
    ///
    /// # Arguments
    /// * `params` - The publish intent parameters including quote hashes and signed data
    pub async fn publish_intent(&self, params: PublishIntentParams) -> Result<PublishIntentResult> {
        let result: PublishIntentResult = self.call("publish_intent", Some(vec![params])).await?;

        if result.status == PublishStatus::Failed {
            return Err(SolverRelayError::PublishFailed {
                reason: result.reason.clone().unwrap_or_default(),
            });
        }

        Ok(result)
    }

    /// Get the status of an intent
    ///
    /// # Arguments
    /// * `intent_hash` - The intent hash to query
    pub async fn get_status(&self, intent_hash: &str) -> Result<IntentStatus> {
        let params = GetStatusParams {
            intent_hash: intent_hash.to_string(),
        };

        self.call("get_status", Some(vec![params])).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SolverRelayRpcClient::new();
        assert_eq!(client.url(), DEFAULT_RPC_URL);

        let custom_client = SolverRelayRpcClient::with_url("https://custom.endpoint/rpc");
        assert_eq!(custom_client.url(), "https://custom.endpoint/rpc");
    }
}
