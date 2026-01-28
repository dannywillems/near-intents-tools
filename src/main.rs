use near_intents_tools::{
    rpc::SolverRelayRpcClient, types::SubscriptionType, ws::SolverRelayWsClient, SolverEvent,
};
use tracing::{error, info};

// EVM Chain IDs
const CHAIN_ETH: u64 = 1;
const CHAIN_ARB: u64 = 42161;
const CHAIN_BASE: u64 = 8453;
const CHAIN_OP: u64 = 10;
const CHAIN_BSC: u64 = 56;
const CHAIN_POL: u64 = 137;
const CHAIN_AVAX: u64 = 43114;
const CHAIN_GNOSIS: u64 = 100;

// Common ERC20 token addresses (same across most EVM chains)
const USDC_ETH: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const USDT_ETH: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
const DAI_ETH: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";
const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const WBTC_ETH: &str = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599";

// Base-specific addresses
const USDC_BASE: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";

// Arbitrum-specific addresses
const USDC_ARB: &str = "0xaf88d065e77c8cc2239327c5edb3a432268e5831";
const USDT_ARB: &str = "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9";

/// Resolve currency alias to full asset identifier
/// Supports formats:
///   - Simple: `near`, `usdt`, `usdc`
///   - Chain-prefixed: `near:usdt`, `eth:usdc`, `base:usdc`
fn resolve_currency(input: &str) -> String {
    let input_lower = input.to_lowercase();

    // Check for chain:currency format
    if let Some((chain, currency)) = input_lower.split_once(':') {
        return resolve_chain_currency(chain, currency);
    }

    // Simple aliases (default to NEAR chain)
    match input_lower.as_str() {
        // Native tokens
        "near" | "wnear" => "nep141:wrap.near".to_string(),
        "eth" | "weth" => "nep141:aurora".to_string(),
        "btc" | "wbtc" => format!(
            "nep141:{}.factory.bridge.near",
            WBTC_ETH.trim_start_matches("0x")
        ),

        // Stablecoins (default to NEAR versions)
        "usdt" => "nep141:usdt.tether-token.near".to_string(),
        "usdc" => {
            "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".to_string()
        }
        "dai" => format!(
            "nep141:{}.factory.bridge.near",
            DAI_ETH.trim_start_matches("0x")
        ),

        // Other tokens
        "aurora" => {
            "nep141:aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near".to_string()
        }

        // If no alias found, return as-is
        _ => input.to_string(),
    }
}

/// Resolve chain:currency format to full asset identifier
fn resolve_chain_currency(chain: &str, currency: &str) -> String {
    match chain {
        // NEAR chain
        "near" => match currency {
            "near" | "wnear" => "nep141:wrap.near".to_string(),
            "usdt" => "nep141:usdt.tether-token.near".to_string(),
            "usdc" => "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"
                .to_string(),
            "eth" | "weth" => "nep141:aurora".to_string(),
            "aurora" => {
                "nep141:aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near".to_string()
            }
            _ => format!("nep141:{}", currency),
        },

        // Ethereum mainnet
        "eth" => match currency {
            "eth" | "native" => format!("eth:{}:native", CHAIN_ETH),
            "weth" => format!("eth:{}:{}", CHAIN_ETH, WETH),
            "usdc" => format!("eth:{}:{}", CHAIN_ETH, USDC_ETH),
            "usdt" => format!("eth:{}:{}", CHAIN_ETH, USDT_ETH),
            "dai" => format!("eth:{}:{}", CHAIN_ETH, DAI_ETH),
            "wbtc" => format!("eth:{}:{}", CHAIN_ETH, WBTC_ETH),
            _ => format!("eth:{}:{}", CHAIN_ETH, currency),
        },

        // Base
        "base" => match currency {
            "eth" | "native" => format!("eth:{}:native", CHAIN_BASE),
            "usdc" => format!("eth:{}:{}", CHAIN_BASE, USDC_BASE),
            _ => format!("eth:{}:{}", CHAIN_BASE, currency),
        },

        // Arbitrum
        "arb" | "arbitrum" => match currency {
            "eth" | "native" => format!("eth:{}:native", CHAIN_ARB),
            "usdc" => format!("eth:{}:{}", CHAIN_ARB, USDC_ARB),
            "usdt" => format!("eth:{}:{}", CHAIN_ARB, USDT_ARB),
            _ => format!("eth:{}:{}", CHAIN_ARB, currency),
        },

        // Optimism
        "op" | "optimism" => match currency {
            "eth" | "native" => format!("eth:{}:native", CHAIN_OP),
            _ => format!("eth:{}:{}", CHAIN_OP, currency),
        },

        // BSC
        "bsc" | "bnb" => match currency {
            "bnb" | "native" => format!("eth:{}:native", CHAIN_BSC),
            _ => format!("eth:{}:{}", CHAIN_BSC, currency),
        },

        // Polygon
        "pol" | "polygon" | "matic" => match currency {
            "matic" | "pol" | "native" => format!("eth:{}:native", CHAIN_POL),
            _ => format!("eth:{}:{}", CHAIN_POL, currency),
        },

        // Avalanche
        "avax" | "avalanche" => match currency {
            "avax" | "native" => format!("eth:{}:native", CHAIN_AVAX),
            _ => format!("eth:{}:{}", CHAIN_AVAX, currency),
        },

        // Gnosis
        "gnosis" | "xdai" => match currency {
            "xdai" | "native" => format!("eth:{}:native", CHAIN_GNOSIS),
            _ => format!("eth:{}:{}", CHAIN_GNOSIS, currency),
        },

        // Bitcoin
        "btc" | "bitcoin" => "btc:mainnet".to_string(),

        // Solana
        "sol" | "solana" => match currency {
            "sol" | "native" => "solana:mainnet:native".to_string(),
            _ => format!("solana:mainnet:{}", currency),
        },

        // Unknown chain - pass through
        _ => format!("{}:{}", chain, currency),
    }
}

/// Get currency display name from identifier
fn currency_name(identifier: &str) -> String {
    // NEAR tokens
    if identifier.starts_with("nep141:") {
        return match identifier {
            "nep141:wrap.near" => "NEAR".to_string(),
            "nep141:usdt.tether-token.near" => "near:USDT".to_string(),
            "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1" => {
                "near:USDC".to_string()
            }
            "nep141:aurora" => "near:ETH".to_string(),
            "nep141:aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near" => {
                "AURORA".to_string()
            }
            _ => identifier.to_string(),
        };
    }

    // EVM tokens
    if identifier.starts_with("eth:") {
        let parts: Vec<&str> = identifier.split(':').collect();
        if parts.len() >= 3 {
            let chain_id: u64 = parts[1].parse().unwrap_or(0);
            let chain_name = match chain_id {
                1 => "eth",
                42161 => "arb",
                8453 => "base",
                10 => "op",
                56 => "bsc",
                137 => "pol",
                43114 => "avax",
                100 => "gnosis",
                _ => "evm",
            };
            let token = parts[2];
            if token == "native" {
                return format!("{}:native", chain_name);
            }
            // Try to identify common tokens
            let token_lower = token.to_lowercase();
            if token_lower.contains("usdc") || token_lower == USDC_ETH || token_lower == USDC_BASE {
                return format!("{}:USDC", chain_name);
            }
            if token_lower.contains("usdt") || token_lower == USDT_ETH {
                return format!("{}:USDT", chain_name);
            }
            return format!("{}:{}", chain_name, &token[..8.min(token.len())]);
        }
    }

    // BTC
    if identifier == "btc:mainnet" {
        return "BTC".to_string();
    }

    // Solana
    if identifier.starts_with("solana:") {
        let parts: Vec<&str> = identifier.split(':').collect();
        if parts.len() >= 3 {
            if parts[2] == "native" {
                return "SOL".to_string();
            }
            return format!("sol:{}", &parts[2][..8.min(parts[2].len())]);
        }
    }

    identifier.to_string()
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
  quote <from> <to> <amount>             Request a quote for a token swap
  watch <from> <to> <amount> [interval]  Continuously watch quotes (default: 5s)
  status <intent_hash>                   Check the status of an intent
  ws, websocket                          Connect to Solver Relay WebSocket (solvers only)

Currency Format:
  Simple:         near, usdt, usdc, eth, btc
  Chain-prefixed: near:usdt, eth:usdc, base:usdc, arb:usdt

Supported Chains:
  near              NEAR Protocol
  eth               Ethereum Mainnet
  base              Base
  arb, arbitrum     Arbitrum One
  op, optimism      Optimism
  bsc, bnb          BNB Chain
  pol, polygon      Polygon
  avax              Avalanche
  gnosis            Gnosis Chain
  btc               Bitcoin
  sol, solana       Solana

Examples:
  # NEAR to USDT on NEAR (1 NEAR = 10^24 yoctoNEAR)
  near-intents-tools quote near usdt 1000000000000000000000000

  # NEAR to USDC on Base
  near-intents-tools quote near base:usdc 1000000000000000000000000

  # ETH on Ethereum to USDC on Arbitrum
  near-intents-tools quote eth:eth arb:usdc 1000000000000000000

  # Watch quotes continuously
  near-intents-tools watch near usdt 1000000000000000000000000

  # Check intent status
  near-intents-tools status <intent_hash>

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
        eprintln!(
            "  near-intents-tools {} near base:usdc 1000000000000000000000000",
            cmd
        );
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
