use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// JSON-RPC Base Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl<T> JsonRpcRequest<T> {
    pub fn new(method: &str, params: Option<T>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification<T> {
    pub jsonrpc: String,
    pub method: String,
    pub params: T,
}

// =============================================================================
// Signature Standards
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SignatureStandard {
    Nep413,
    Erc191,
    #[serde(rename = "raw_ed25519")]
    RawEd25519,
}

// =============================================================================
// Intent Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "intent")]
pub enum Intent {
    #[serde(rename = "token_diff")]
    TokenDiff {
        /// Map of asset identifier to amount
        /// Positive values: tokens to receive
        /// Negative values: tokens to transfer (prefixed with "-")
        diff: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMessage {
    pub signer_id: String,
    pub intents: Vec<Intent>,
    /// ISO-8601 formatted deadline
    pub deadline: String,
}

// =============================================================================
// NEP-413 Payload
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nep413Payload {
    /// Contract address (typically "intents.near")
    pub recipient: String,
    /// Base64-encoded unique nonce
    pub nonce: String,
    /// JSON-encoded IntentMessage or raw string
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "callbackUrl")]
    pub callback_url: Option<String>,
}

// =============================================================================
// Signed Data
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedData {
    pub standard: SignatureStandard,
    pub payload: Nep413Payload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    pub signature: String,
}

// =============================================================================
// Quote Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteParams {
    pub defuse_asset_identifier_in: String,
    pub defuse_asset_identifier_out: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_amount_in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_amount_out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_deadline_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub quote_hash: String,
    pub defuse_asset_identifier_in: String,
    pub defuse_asset_identifier_out: String,
    pub amount_in: String,
    pub amount_out: String,
    /// ISO-8601 formatted expiration time
    pub expiration_time: String,
}

// =============================================================================
// Publish Intent Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishIntentParams {
    pub quote_hashes: Vec<String>,
    pub signed_data: SignedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishIntentResult {
    pub status: PublishStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub intent_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PublishStatus {
    Ok,
    Failed,
}

// =============================================================================
// Get Status Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusParams {
    pub intent_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStatus {
    pub intent_hash: String,
    pub status: IntentStatusType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<IntentStatusData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntentStatusType {
    /// Intent was successfully received and is pending execution
    Pending,
    /// Transaction has been sent to the NEAR Intents contract
    TxBroadcasted,
    /// Intent has been successfully settled on chain
    Settled,
    /// Intent wasn't received, has expired, or execution failed
    NotFoundOrNotValid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStatusData {
    /// NEAR transaction hash
    pub hash: String,
}

// =============================================================================
// WebSocket Subscription Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionType {
    Quote,
    QuoteStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequestEvent {
    pub subscription: String,
    pub quote_id: String,
    pub defuse_asset_identifier_in: String,
    pub defuse_asset_identifier_out: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_amount_in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_amount_out: Option<String>,
    pub min_deadline_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_in: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponseParams {
    pub quote_id: String,
    pub quote_output: QuoteOutput,
    pub signed_data: SignedData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_quote_hashes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteStatusEvent {
    pub quote_hash: String,
    pub intent_hash: String,
    pub tx_hash: String,
}

// =============================================================================
// WebSocket Event Enum
// =============================================================================

#[derive(Debug, Clone)]
pub enum SolverEvent {
    /// New quote request from a user
    QuoteRequest(QuoteRequestEvent),
    /// Quote has been executed
    QuoteStatus(QuoteStatusEvent),
    /// Subscription confirmed
    Subscribed { subscription_id: String },
    /// Unsubscription confirmed
    Unsubscribed,
    /// Quote response acknowledged
    QuoteResponseAck,
}

// =============================================================================
// Asset Identifier Helper
// =============================================================================

/// Represents a NEAR Intents asset identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetIdentifier {
    pub chain_type: String,
    pub token_address: String,
}

impl AssetIdentifier {
    /// Create a NEP-141 token identifier
    pub fn nep141(token_address: &str) -> Self {
        Self {
            chain_type: "nep141".to_string(),
            token_address: token_address.to_string(),
        }
    }

    /// Create an ERC-20 token identifier
    pub fn erc20(chain_id: u64, token_address: &str) -> Self {
        Self {
            chain_type: format!("erc20:{}", chain_id),
            token_address: token_address.to_string(),
        }
    }

    /// Parse from string format "chain_type:token_address"
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() == 2 {
            Some(Self {
                chain_type: parts[0].to_string(),
                token_address: parts[1].to_string(),
            })
        } else {
            None
        }
    }
}

impl std::fmt::Display for AssetIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.chain_type, self.token_address)
    }
}

impl Serialize for AssetIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AssetIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom("invalid asset identifier"))
    }
}
