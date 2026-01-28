use near_intents_tools::{
    rpc::SolverRelayRpcClient, types::SubscriptionType, ws::SolverRelayWsClient, SolverEvent,
};
use tracing::{error, info};

/// Resolve currency alias to full asset identifier
fn resolve_currency(input: &str) -> String {
    match input.to_lowercase().as_str() {
        // NEAR
        "near" | "wnear" => "nep141:wrap.near".to_string(),
        // Stablecoins
        "usdt" => "nep141:usdt.tether-token.near".to_string(),
        "usdc" => {
            "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".to_string()
        }
        "dai" => "nep141:6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near".to_string(),
        "frax" => "nep141:853d955acef822db058eb8505911ed77f175b99e.factory.bridge.near".to_string(),
        // ETH
        "eth" | "weth" => "nep141:aurora".to_string(),
        // BTC
        "btc" | "wbtc" => {
            "nep141:2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near".to_string()
        }
        // Other tokens
        "aurora" => {
            "nep141:aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near".to_string()
        }
        // If no alias found, return as-is (assume it's a full identifier)
        _ => input.to_string(),
    }
}

/// Get currency display name from identifier
fn currency_name(identifier: &str) -> &str {
    match identifier {
        "nep141:wrap.near" => "NEAR",
        "nep141:usdt.tether-token.near" => "USDT",
        "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1" => "USDC",
        "nep141:6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near" => "DAI",
        "nep141:853d955acef822db058eb8505911ed77f175b99e.factory.bridge.near" => "FRAX",
        "nep141:aurora" => "ETH",
        "nep141:2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near" => "WBTC",
        "nep141:aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near" => "AURORA",
        _ => identifier,
    }
}

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
        "quote" => run_quote(&args[2..], false).await,
        "watch" => run_quote(&args[2..], true).await,
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
  quote <from> <to> <amount>         Request a quote for a token swap
  watch <from> <to> <amount> [interval]  Continuously watch quotes (default: 5s)
  status <intent_hash>               Check the status of an intent
  ws, websocket                      Connect to Solver Relay WebSocket (solvers only)

Arguments:
  <from>          Source currency (alias or full identifier)
  <to>            Target currency (alias or full identifier)
  <amount>        Amount in smallest units
  <interval>      Polling interval in seconds (default: 5)
  <intent_hash>   Intent hash to query

Currency Aliases:
  near, wnear     Wrapped NEAR
  usdt            USDT (Tether)
  usdc            USDC
  dai             DAI
  frax            FRAX
  eth, weth       Wrapped ETH (Aurora)
  btc, wbtc       Wrapped BTC
  aurora          AURORA token

Examples:
  # Quote 1 NEAR to USDT (1 NEAR = 10^24 yoctoNEAR)
  near-intents-tools quote near usdt 1000000000000000000000000

  # Watch quotes continuously (every 5 seconds)
  near-intents-tools watch near usdt 1000000000000000000000000

  # Watch with custom interval (every 10 seconds)
  near-intents-tools watch near usdt 1000000000000000000000000 10

  # Quote 1 USDT to NEAR (1 USDT = 10^6 units)
  near-intents-tools quote usdt near 1000000

  # Check intent status
  near-intents-tools status abc123...

Note: WebSocket endpoint requires solver registration.

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

async fn run_quote(args: &[String], watch: bool) {
    if args.len() < 3 {
        let cmd = if watch { "watch" } else { "quote" };
        eprintln!(
            "Usage: near-intents-tools {} <from> <to> <amount> [interval]",
            cmd
        );
        eprintln!();
        eprintln!("Examples:");
        eprintln!(
            "  near-intents-tools {} near usdt 1000000000000000000000000",
            cmd
        );
        eprintln!("  near-intents-tools {} usdt near 1000000", cmd);
        std::process::exit(1);
    }

    let from = resolve_currency(&args[0]);
    let to = resolve_currency(&args[1]);
    let amount = &args[2];
    let interval_secs: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(5);

    let from_name = currency_name(&from);
    let to_name = currency_name(&to);

    let client = SolverRelayRpcClient::new();

    loop {
        info!("Requesting quote: {} {} -> {}", amount, from_name, to_name);

        match client
            .quote(&from, &to, Some(amount), None, Some(60000))
            .await
        {
            Ok(quotes) => {
                if quotes.is_empty() {
                    println!("No quotes available for this pair");
                } else {
                    // Clear screen for watch mode
                    if watch {
                        print!("\x1B[2J\x1B[1;1H");
                        println!(
                            "Watching {} -> {} (every {}s) | Press Ctrl+C to stop\n",
                            from_name, to_name, interval_secs
                        );
                    }

                    println!("Quotes received: {}", quotes.len());
                    println!("{:-<80}", "");
                    for (i, quote) in quotes.iter().enumerate() {
                        let in_name = currency_name(&quote.defuse_asset_identifier_in);
                        let out_name = currency_name(&quote.defuse_asset_identifier_out);
                        println!("Quote #{}:", i + 1);
                        println!("  From:    {} {}", quote.amount_in, in_name);
                        println!("  To:      {} {}", quote.amount_out, out_name);
                        println!("  Hash:    {}", quote.quote_hash);
                        println!("  Expires: {}", quote.expiration_time);
                        println!();
                    }
                }
            }
            Err(e) => {
                error!("Failed to get quote: {}", e);
                if !watch {
                    std::process::exit(1);
                }
            }
        }

        if !watch {
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
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
