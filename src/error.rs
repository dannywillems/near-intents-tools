use thiserror::Error;

#[derive(Error, Debug)]
pub enum SolverRelayError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("RPC error: code={code}, message={message}")]
    RpcError {
        code: i64,
        message: String,
        data: Option<String>,
    },

    #[error("Intent publication failed: {reason}")]
    PublishFailed { reason: String },

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Not subscribed")]
    NotSubscribed,
}

pub type Result<T> = std::result::Result<T, SolverRelayError>;
