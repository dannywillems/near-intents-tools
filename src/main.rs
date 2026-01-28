use near_intents_tools::{
    rpc::SolverRelayRpcClient, types::SubscriptionType, ws::SolverRelayWsClient, SolverEvent,
};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("NEAR Intents Tools");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match mode {
        "ws" | "websocket" => run_websocket().await,
        "rpc" | "quote" => run_rpc_demo().await,
        _ => print_help(),
    }
}

fn print_help() {
    println!(
        r#"
NEAR Intents Tools

Usage: near-intents-tools <command>

Commands:
  ws, websocket    Connect to the Solver Relay WebSocket (requires solver registration)
  rpc, quote       Demo RPC quote request

Note: The WebSocket endpoint requires solver registration.
      Public connections will receive 403 Forbidden.

For more information, see:
  https://docs.near-intents.org/near-intents/market-makers/bus/solver-relay
"#
    );
}

async fn run_websocket() {
    info!("Starting WebSocket listener...");
    info!("Note: This requires solver registration. Public connections get 403.");

    loop {
        let mut client = SolverRelayWsClient::new();

        match client.connect(SubscriptionType::Quote).await {
            Ok((mut events, _response_tx)) => {
                info!("Connected and subscribed to quote events");

                while let Some(event) = events.recv().await {
                    match event {
                        SolverEvent::QuoteRequest(req) => {
                            info!(
                                "Quote request: {} -> {} (amount_in: {:?}, amount_out: {:?})",
                                req.defuse_asset_identifier_in,
                                req.defuse_asset_identifier_out,
                                req.exact_amount_in,
                                req.exact_amount_out
                            );
                        }
                        SolverEvent::QuoteStatus(status) => {
                            info!(
                                "Quote status: quote_hash={}, intent_hash={}, tx_hash={}",
                                status.quote_hash, status.intent_hash, status.tx_hash
                            );
                        }
                        SolverEvent::Subscribed { subscription_id } => {
                            info!("Subscribed with ID: {}", subscription_id);
                        }
                        SolverEvent::QuoteResponseAck => {
                            info!("Quote response acknowledged");
                        }
                        SolverEvent::Unsubscribed => {
                            info!("Unsubscribed");
                        }
                    }
                }
            }
            Err(e) => {
                error!("Connection error: {}", e);
            }
        }

        info!("Reconnecting in 5 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn run_rpc_demo() {
    info!("Running RPC demo...");

    let client = SolverRelayRpcClient::new();

    // Example: Request a quote for NEAR -> USDT swap
    info!("Requesting quote for 1 NEAR -> USDT...");

    match client
        .quote(
            "nep141:wrap.near",
            "nep141:usdt.tether-token.near",
            Some("1000000000000000000000000"), // 1 NEAR in yoctoNEAR
            None,
            Some(60000),
        )
        .await
    {
        Ok(quotes) => {
            if quotes.is_empty() {
                info!("No quotes available");
            } else {
                for quote in quotes {
                    info!(
                        "Quote: {} {} -> {} {} (expires: {})",
                        quote.amount_in,
                        quote.defuse_asset_identifier_in,
                        quote.amount_out,
                        quote.defuse_asset_identifier_out,
                        quote.expiration_time
                    );
                }
            }
        }
        Err(e) => {
            error!("Failed to get quote: {}", e);
        }
    }
}
