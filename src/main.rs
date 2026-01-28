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

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match mode {
        "ws" | "websocket" => run_websocket().await,
        "quote" => run_quote(&args[2..]).await,
        "status" => run_status(&args[2..]).await,
        _ => print_help(),
    }
}

fn print_help() {
    println!(
        r#"
NEAR Intents Tools

Usage: near-intents-tools <command> [options]

Commands:
  quote <from> <to> <amount>   Request a quote for a token swap
  status <intent_hash>         Check the status of an intent
  ws, websocket                Connect to Solver Relay WebSocket (solvers only)

Arguments:
  <from>          Source asset (e.g., nep141:wrap.near, nep141:usdt.tether-token.near)
  <to>            Target asset (e.g., nep141:usdc.tether-token.near)
  <amount>        Amount in smallest units (e.g., 1000000000000000000000000 for 1 NEAR)
  <intent_hash>   Intent hash to query

Examples:
  # Quote 1 NEAR to USDT
  near-intents-tools quote nep141:wrap.near nep141:usdt.tether-token.near 1000000000000000000000000

  # Quote 1 USDC to NEAR
  near-intents-tools quote nep141:usdc nep141:wrap.near 1000000

  # Check intent status
  near-intents-tools status abc123...

Common tokens:
  nep141:wrap.near                  - Wrapped NEAR
  nep141:usdt.tether-token.near     - USDT on NEAR
  nep141:usdc                       - USDC on NEAR
  nep141:aurora                     - AURORA token

Note: WebSocket endpoint requires solver registration.
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

async fn run_quote(args: &[String]) {
    if args.len() < 3 {
        eprintln!("Usage: near-intents-tools quote <from> <to> <amount>");
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  near-intents-tools quote nep141:wrap.near nep141:usdt.tether-token.near \
             1000000000000000000000000"
        );
        std::process::exit(1);
    }

    let from = &args[0];
    let to = &args[1];
    let amount = &args[2];

    info!("Requesting quote: {} {} -> {}", amount, from, to);

    let client = SolverRelayRpcClient::new();

    match client
        .quote(from, to, Some(amount), None, Some(60000))
        .await
    {
        Ok(quotes) => {
            if quotes.is_empty() {
                println!("No quotes available for this pair");
            } else {
                println!();
                println!("Quotes received: {}", quotes.len());
                println!("{:-<80}", "");
                for (i, quote) in quotes.iter().enumerate() {
                    println!("Quote #{}:", i + 1);
                    println!(
                        "  From:    {} {}",
                        quote.amount_in, quote.defuse_asset_identifier_in
                    );
                    println!(
                        "  To:      {} {}",
                        quote.amount_out, quote.defuse_asset_identifier_out
                    );
                    println!("  Hash:    {}", quote.quote_hash);
                    println!("  Expires: {}", quote.expiration_time);
                    println!();
                }
            }
        }
        Err(e) => {
            error!("Failed to get quote: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_status(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: near-intents-tools status <intent_hash>");
        std::process::exit(1);
    }

    let intent_hash = &args[0];

    info!("Checking status for intent: {}", intent_hash);

    let client = SolverRelayRpcClient::new();

    match client.get_status(intent_hash).await {
        Ok(status) => {
            println!();
            println!("Intent Status");
            println!("{:-<80}", "");
            println!("  Hash:   {}", status.intent_hash);
            println!("  Status: {:?}", status.status);
            if let Some(data) = status.data {
                println!("  TX:     {}", data.hash);
            }
        }
        Err(e) => {
            error!("Failed to get status: {}", e);
            std::process::exit(1);
        }
    }
}
