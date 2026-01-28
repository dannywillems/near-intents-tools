use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

const SOLVER_RELAY_WS: &str = "wss://solver-relay-v2.chaindefuser.com/ws";

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    fn new(method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            method: method.to_string(),
            params,
        }
    }

    fn subscribe() -> Self {
        Self::new("subscribe", None)
    }
}

async fn connect_and_listen() -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Connecting to NEAR Intents Solver Relay: {}",
        SOLVER_RELAY_WS
    );

    let (ws_stream, response) = connect_async(SOLVER_RELAY_WS).await?;
    info!("WebSocket connected. Response: {:?}", response.status());

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to receive intent events
    let subscribe_msg = JsonRpcRequest::subscribe();
    let subscribe_json = serde_json::to_string(&subscribe_msg)?;
    info!("Sending subscribe request: {}", subscribe_json);
    write.send(Message::Text(subscribe_json.into())).await?;

    info!("Listening for intents...");

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => match serde_json::from_str::<JsonRpcResponse>(&text) {
                Ok(response) => {
                    if let Some(method) = &response.method {
                        info!("Received notification: method={}", method);
                        if let Some(params) = &response.params {
                            info!("Params: {}", serde_json::to_string_pretty(params)?);
                        }
                    } else if let Some(result) = &response.result {
                        info!("Received result: {}", serde_json::to_string_pretty(result)?);
                    } else if let Some(error) = &response.error {
                        warn!("Received error: {}", serde_json::to_string_pretty(error)?);
                    }
                }
                Err(e) => {
                    warn!("Failed to parse JSON-RPC response: {}", e);
                    info!("Raw message: {}", text);
                }
            },
            Ok(Message::Binary(data)) => {
                info!("Received binary message: {} bytes", data.len());
            }
            Ok(Message::Ping(data)) => {
                write.send(Message::Pong(data)).await?;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(frame)) => {
                info!("Connection closed: {:?}", frame);
                break;
            }
            Ok(Message::Frame(_)) => {}
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("NEAR Intents Tools - WebSocket Listener");

    loop {
        match connect_and_listen().await {
            Ok(()) => {
                info!("Connection closed normally");
            }
            Err(e) => {
                error!("Connection error: {}", e);
            }
        }

        info!("Reconnecting in 5 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
