use crate::{
    error::{Result, SolverRelayError},
    types::{
        JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, QuoteRequestEvent,
        QuoteResponseParams, QuoteStatusEvent, SolverEvent, SubscriptionType,
    },
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

const DEFAULT_WS_URL: &str = "wss://solver-relay-v2.chaindefuser.com/ws";

/// WebSocket client for the NEAR Intents Solver Relay
///
/// This client is used by solvers to receive quote requests and respond with quotes.
/// Note: The WebSocket endpoint requires solver registration - public connections
/// will receive 403 Forbidden.
pub struct SolverRelayWsClient {
    url: String,
    subscription_id: Option<String>,
}

impl Default for SolverRelayWsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SolverRelayWsClient {
    /// Create a new WebSocket client with the default endpoint
    pub fn new() -> Self {
        Self {
            url: DEFAULT_WS_URL.to_string(),
            subscription_id: None,
        }
    }

    /// Create a new WebSocket client with a custom endpoint
    pub fn with_url(url: &str) -> Self {
        Self {
            url: url.to_string(),
            subscription_id: None,
        }
    }

    /// Get the WebSocket endpoint URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Connect and start listening for events
    ///
    /// Returns a channel receiver for incoming events and a sender for outgoing
    /// quote responses.
    ///
    /// # Arguments
    /// * `subscription_type` - Type of events to subscribe to (Quote or QuoteStatus)
    pub async fn connect(
        &mut self,
        subscription_type: SubscriptionType,
    ) -> Result<(
        mpsc::Receiver<SolverEvent>,
        mpsc::Sender<QuoteResponseParams>,
    )> {
        info!("Connecting to Solver Relay WebSocket: {}", self.url);

        let (ws_stream, response) = connect_async(&self.url).await?;
        info!("WebSocket connected. Status: {:?}", response.status());

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to events
        let sub_type_str = match subscription_type {
            SubscriptionType::Quote => "quote",
            SubscriptionType::QuoteStatus => "quote_status",
        };
        let subscribe_req: JsonRpcRequest<Vec<&str>> =
            JsonRpcRequest::new("subscribe", Some(vec![sub_type_str]));
        let subscribe_json = serde_json::to_string(&subscribe_req)?;
        debug!("Sending subscribe: {}", subscribe_json);
        write.send(Message::Text(subscribe_json.into())).await?;

        // Wait for subscription confirmation
        if let Some(Ok(Message::Text(text))) = read.next().await {
            let response: JsonRpcResponse<String> = serde_json::from_str(&text)?;
            if let Some(subscription_id) = response.result {
                info!("Subscribed with ID: {}", subscription_id);
                self.subscription_id = Some(subscription_id.clone());
            } else if let Some(error) = response.error {
                return Err(SolverRelayError::RpcError {
                    code: error.code,
                    message: error.message,
                    data: error.data,
                });
            }
        }

        let (event_tx, event_rx) = mpsc::channel::<SolverEvent>(100);
        let (response_tx, mut response_rx) = mpsc::channel::<QuoteResponseParams>(100);

        // Spawn task to handle incoming messages
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = handle_message(&text, &event_tx_clone).await {
                            error!("Error handling message: {}", e);
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        debug!("Received ping");
                        // Note: pong is handled by the write task
                        let _ = data;
                    }
                    Ok(Message::Close(frame)) => {
                        info!("Connection closed: {:?}", frame);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Spawn task to handle outgoing quote responses
        tokio::spawn(async move {
            while let Some(params) = response_rx.recv().await {
                let request: JsonRpcRequest<Vec<QuoteResponseParams>> =
                    JsonRpcRequest::new("quote_response", Some(vec![params]));
                match serde_json::to_string(&request) {
                    Ok(json) => {
                        debug!("Sending quote_response: {}", json);
                        if let Err(e) = write.send(Message::Text(json.into())).await {
                            error!("Failed to send quote response: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize quote response: {}", e);
                    }
                }
            }
        });

        Ok((event_rx, response_tx))
    }
}

async fn handle_message(text: &str, event_tx: &mpsc::Sender<SolverEvent>) -> Result<()> {
    debug!("Received message: {}", text);

    // Try to parse as a notification (incoming event)
    if let Ok(notification) = serde_json::from_str::<JsonRpcNotification<serde_json::Value>>(text) {
        match notification.method.as_str() {
            "subscribe" => {
                // This is a quote request or quote status event
                if let Ok(event) =
                    serde_json::from_value::<QuoteRequestEvent>(notification.params.clone())
                {
                    let _ = event_tx.send(SolverEvent::QuoteRequest(event)).await;
                } else if let Ok(event) =
                    serde_json::from_value::<QuoteStatusEvent>(notification.params)
                {
                    let _ = event_tx.send(SolverEvent::QuoteStatus(event)).await;
                } else {
                    warn!("Unknown subscribe event params");
                }
            }
            _ => {
                warn!("Unknown notification method: {}", notification.method);
            }
        }
        return Ok(());
    }

    // Try to parse as a response (ack for our requests)
    if let Ok(response) = serde_json::from_str::<JsonRpcResponse<String>>(text) {
        if response.error.is_some() {
            let error = response.error.unwrap();
            warn!("Received error response: {:?}", error);
            return Err(SolverRelayError::RpcError {
                code: error.code,
                message: error.message,
                data: error.data,
            });
        }
        if let Some(result) = response.result {
            if result == "OK" {
                let _ = event_tx.send(SolverEvent::QuoteResponseAck).await;
            }
        }
    }

    Ok(())
}

/// Builder for creating quote responses
pub struct QuoteResponseBuilder {
    quote_id: String,
    amount_out: Option<String>,
    amount_in: Option<String>,
    other_quote_hashes: Option<Vec<String>>,
}

impl QuoteResponseBuilder {
    /// Create a new quote response builder
    pub fn new(quote_id: &str) -> Self {
        Self {
            quote_id: quote_id.to_string(),
            amount_out: None,
            amount_in: None,
            other_quote_hashes: None,
        }
    }

    /// Set the output amount (for exact_amount_in requests)
    pub fn amount_out(mut self, amount: &str) -> Self {
        self.amount_out = Some(amount.to_string());
        self
    }

    /// Set the input amount (for exact_amount_out requests)
    pub fn amount_in(mut self, amount: &str) -> Self {
        self.amount_in = Some(amount.to_string());
        self
    }

    /// Add other quote hashes for multi-hop swaps
    pub fn other_quote_hashes(mut self, hashes: Vec<String>) -> Self {
        self.other_quote_hashes = Some(hashes);
        self
    }

    /// Get the quote ID
    pub fn quote_id(&self) -> &str {
        &self.quote_id
    }

    /// Get the configured amount_out
    pub fn get_amount_out(&self) -> Option<&str> {
        self.amount_out.as_deref()
    }

    /// Get the configured amount_in
    pub fn get_amount_in(&self) -> Option<&str> {
        self.amount_in.as_deref()
    }

    /// Get the other quote hashes
    pub fn get_other_quote_hashes(&self) -> Option<&[String]> {
        self.other_quote_hashes.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SolverRelayWsClient::new();
        assert_eq!(client.url(), DEFAULT_WS_URL);

        let custom_client = SolverRelayWsClient::with_url("wss://custom.endpoint/ws");
        assert_eq!(custom_client.url(), "wss://custom.endpoint/ws");
    }

    #[test]
    fn test_quote_response_builder() {
        let builder = QuoteResponseBuilder::new("test-quote-id")
            .amount_out("1000")
            .other_quote_hashes(vec!["hash1".to_string(), "hash2".to_string()]);

        assert_eq!(builder.quote_id(), "test-quote-id");
        assert_eq!(builder.get_amount_out(), Some("1000"));
        assert_eq!(builder.get_amount_in(), None);
        assert_eq!(
            builder.get_other_quote_hashes(),
            Some(vec!["hash1".to_string(), "hash2".to_string()].as_slice())
        );
    }
}
