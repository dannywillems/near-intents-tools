use near_intents_tools::{
    rpc::SolverRelayRpcClient, types::SubscriptionType, ws::SolverRelayWsClient, SolverEvent,
};
use serde::Deserialize;
use std::io::Write;
use tracing::{error, info};

const TOKENS_API_URL: &str = "https://1click.chaindefuser.com/v0/tokens";

// Common ERC20 token addresses
const USDC_ETH: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const USDT_ETH: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
const DAI_ETH: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";
const WBTC_ETH: &str = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599";

// Base-specific addresses
const USDC_BASE: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";

// Arbitrum-specific addresses
const USDC_ARB: &str = "0xaf88d065e77c8cc2239327c5edb3a432268e5831";
const USDT_ARB: &str = "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9";

// Gnosis-specific addresses
const USDC_GNOSIS: &str = "0x2a22f9c3b484c3629090feed35f17ff8f88f76f0";

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
/// Cross-chain format: nep141:{chain}-{address}.omft.near
fn resolve_chain_currency(chain: &str, currency: &str) -> String {
    match chain {
        // NEAR chain - native tokens
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

        // Ethereum mainnet via OMFT
        "eth" => match currency {
            "usdc" => format!("nep141:eth-{}.omft.near", USDC_ETH),
            "usdt" => format!("nep141:eth-{}.omft.near", USDT_ETH),
            "dai" => format!("nep141:eth-{}.omft.near", DAI_ETH),
            "wbtc" => format!("nep141:eth-{}.omft.near", WBTC_ETH),
            _ => format!("nep141:eth-{}.omft.near", currency),
        },

        // Base via OMFT
        "base" => match currency {
            "usdc" => format!("nep141:base-{}.omft.near", USDC_BASE),
            _ => format!("nep141:base-{}.omft.near", currency),
        },

        // Arbitrum via OMFT
        "arb" | "arbitrum" => match currency {
            "usdc" => format!("nep141:arb-{}.omft.near", USDC_ARB),
            "usdt" => format!("nep141:arb-{}.omft.near", USDT_ARB),
            _ => format!("nep141:arb-{}.omft.near", currency),
        },

        // Gnosis via OMFT
        "gnosis" | "xdai" => match currency {
            "usdc" => format!("nep141:gnosis-{}.omft.near", USDC_GNOSIS),
            _ => format!("nep141:gnosis-{}.omft.near", currency),
        },

        // Solana via OMFT
        "sol" | "solana" => format!("nep141:sol-{}.omft.near", currency),

        // Bitcoin
        "btc" | "bitcoin" => "nep141:btc.omft.near".to_string(),

        // Unknown chain - try OMFT format
        _ => format!("nep141:{}-{}.omft.near", chain, currency),
    }
}

/// Get decimals for a token identifier
fn token_decimals(identifier: &str) -> u8 {
    // Check for OMFT cross-chain format
    if identifier.starts_with("nep141:") && identifier.ends_with(".omft.near") {
        let inner = identifier
            .trim_start_matches("nep141:")
            .trim_end_matches(".omft.near");
        if let Some((_, addr)) = inner.split_once('-') {
            return match addr.to_lowercase().as_str() {
                // USDC/USDT have 6 decimals on all chains
                a if a == USDC_ETH
                    || a == USDC_BASE
                    || a == USDC_ARB
                    || a == USDC_GNOSIS
                    || a == USDT_ETH
                    || a == USDT_ARB =>
                {
                    6
                }
                // DAI has 18 decimals
                a if a == DAI_ETH => 18,
                // WBTC has 8 decimals
                a if a == WBTC_ETH => 8,
                // Default to 18 for unknown tokens
                _ => 18,
            };
        }
    }

    // NEAR native tokens
    match identifier {
        "nep141:wrap.near" => 24,             // NEAR
        "nep141:usdt.tether-token.near" => 6, // USDT
        "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1" => 6, // USDC
        "nep141:aurora" => 18,                // ETH
        "nep141:btc.omft.near" => 8,          // BTC
        _ => 18,                              // Default
    }
}

/// Format a raw amount with decimals for human readability
fn format_amount(raw: &str, decimals: u8) -> String {
    if decimals == 0 {
        return raw.to_string();
    }

    let raw = raw.trim_start_matches('0');
    if raw.is_empty() {
        return "0".to_string();
    }

    let decimals = decimals as usize;
    let len = raw.len();

    if len <= decimals {
        // Need to add leading zeros after decimal point
        let zeros = decimals - len;
        format!("0.{}{}", "0".repeat(zeros), raw.trim_end_matches('0'))
    } else {
        // Insert decimal point
        let (integer, fraction) = raw.split_at(len - decimals);
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            integer.to_string()
        } else {
            format!("{}.{}", integer, fraction)
        }
    }
}

/// Get currency display name from identifier
fn currency_name(identifier: &str) -> String {
    // Check for OMFT cross-chain format: nep141:{chain}-{address}.omft.near
    if identifier.starts_with("nep141:") && identifier.ends_with(".omft.near") {
        let inner = identifier
            .trim_start_matches("nep141:")
            .trim_end_matches(".omft.near");
        if let Some((chain, addr)) = inner.split_once('-') {
            let token = match addr.to_lowercase().as_str() {
                a if a == USDC_ETH || a == USDC_BASE || a == USDC_ARB || a == USDC_GNOSIS => "USDC",
                a if a == USDT_ETH || a == USDT_ARB => "USDT",
                a if a == DAI_ETH => "DAI",
                a if a == WBTC_ETH => "WBTC",
                _ => return format!("{}:{}", chain, &addr[..8.min(addr.len())]),
            };
            return format!("{}:{}", chain, token);
        }
        return identifier.to_string();
    }

    // NEAR native tokens
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
            "nep141:btc.omft.near" => "BTC".to_string(),
            _ => identifier.to_string(),
        };
    }

    identifier.to_string()
}

#[derive(Debug, Deserialize)]
struct Token {
    #[serde(rename = "assetId")]
    asset_id: Option<String>,
    symbol: String,
    blockchain: String,
    decimals: Option<u8>,
}

async fn run_tokens(args: &[String]) {
    let filter = args.first().map(|s| s.to_lowercase());

    info!("Fetching tokens from {}", TOKENS_API_URL);

    let client = reqwest::Client::new();
    match client.get(TOKENS_API_URL).send().await {
        Ok(response) => match response.json::<Vec<Token>>().await {
            Ok(tokens) => {
                let filtered: Vec<&Token> = tokens
                    .iter()
                    .filter(|t| {
                        if let Some(ref f) = filter {
                            t.symbol.to_lowercase().contains(f)
                                || t.blockchain.to_lowercase().contains(f)
                        } else {
                            true
                        }
                    })
                    .collect();

                println!();
                println!("Tokens: {} (showing {})", tokens.len(), filtered.len());
                println!("{:-<100}", "");
                println!(
                    "{:<10} {:<8} {:<10} {:<60}",
                    "CHAIN", "SYMBOL", "DECIMALS", "ASSET ID"
                );
                println!("{:-<100}", "");

                for token in filtered.iter().take(50) {
                    let asset_id = token.asset_id.as_deref().unwrap_or("-");
                    let decimals = token
                        .decimals
                        .map(|d| d.to_string())
                        .unwrap_or("-".to_string());
                    println!(
                        "{:<10} {:<8} {:<10} {:<60}",
                        token.blockchain,
                        token.symbol,
                        decimals,
                        if asset_id.len() > 60 {
                            format!("{}...", &asset_id[..57])
                        } else {
                            asset_id.to_string()
                        }
                    );
                }

                if filtered.len() > 50 {
                    println!();
                    println!("... and {} more tokens", filtered.len() - 50);
                }

                println!();
                println!("Filter: near-intents-tools tokens <symbol|chain>");
            }
            Err(e) => {
                error!("Failed to parse tokens: {}", e);
            }
        },
        Err(e) => {
            error!("Failed to fetch tokens: {}", e);
        }
    }
}

/// Collected quote data for display
struct QuoteData {
    pair: String,
    quotes: Vec<QuoteInfo>,
}

struct QuoteInfo {
    amount_in_raw: String,
    amount_out_raw: String,
    amount_out_fmt: String,
    quote_hash: String,
    expires: String,
    solver_id: Option<String>,
    extra_fields: Vec<(String, String)>,
}

async fn run_monitor(args: &[String]) {
    let interval_secs: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(3);

    let client = SolverRelayRpcClient::new();

    // Use 1 unit of each token as base amount
    let amounts: Vec<(&str, &str, &str)> = vec![
        ("near", "usdt", "1000000000000000000000000"), // 1 NEAR
        ("near", "usdc", "1000000000000000000000000"), // 1 NEAR
        ("usdt", "usdc", "1000000"),                   // 1 USDT
        ("usdc", "usdt", "1000000"),                   // 1 USDC
        ("usdt", "near", "1000000"),                   // 1 USDT
        ("usdc", "near", "1000000"),                   // 1 USDC
        ("near", "eth:usdc", "1000000000000000000000000"), // 1 NEAR
        ("near", "arb:usdc", "1000000000000000000000000"), // 1 NEAR
    ];

    loop {
        // Print immediately to show we're working
        println!(
            "\n[{}] Fetching quotes...",
            chrono::Utc::now().format("%H:%M:%S")
        );
        let _ = std::io::stdout().flush();

        // Collect all quotes first
        let mut all_quotes: Vec<QuoteData> = Vec::new();
        let mut total_quoters = 0;
        let mut unique_hashes: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (from, to, amount) in &amounts {
            let from_resolved = resolve_currency(from);
            let to_resolved = resolve_currency(to);
            let from_name = currency_name(&from_resolved);
            let to_name = currency_name(&to_resolved);
            let pair = format!("{} → {}", from_name, to_name);

            match client
                .quote(
                    &from_resolved,
                    &to_resolved,
                    Some(amount),
                    None,
                    Some(60000),
                )
                .await
            {
                Ok(quotes) if !quotes.is_empty() => {
                    let out_decimals = token_decimals(&quotes[0].defuse_asset_identifier_out);
                    let mut quote_infos: Vec<QuoteInfo> = quotes
                        .iter()
                        .map(|q| {
                            unique_hashes.insert(q.quote_hash.clone());
                            let out_human = format_amount(&q.amount_out, out_decimals);
                            let out_display = if out_human.len() > 14 {
                                format!("{}...", &out_human[..11])
                            } else {
                                out_human
                            };

                            // Collect extra fields
                            let extra: Vec<(String, String)> = q
                                .extra
                                .iter()
                                .map(|(k, v)| (k.clone(), v.to_string()))
                                .collect();

                            QuoteInfo {
                                amount_in_raw: q.amount_in.clone(),
                                amount_out_raw: q.amount_out.clone(),
                                amount_out_fmt: out_display,
                                quote_hash: q.quote_hash.clone(),
                                expires: q.expiration_time.clone(),
                                solver_id: q.solver_id.clone(),
                                extra_fields: extra,
                            }
                        })
                        .collect();

                    // Sort by amount (best first - highest output)
                    quote_infos.sort_by(|a, b| {
                        b.amount_out_raw
                            .parse::<u128>()
                            .unwrap_or(0)
                            .cmp(&a.amount_out_raw.parse::<u128>().unwrap_or(0))
                    });

                    total_quoters += quotes.len();
                    all_quotes.push(QuoteData {
                        pair,
                        quotes: quote_infos,
                    });
                }
                Ok(_) => {
                    all_quotes.push(QuoteData {
                        pair,
                        quotes: vec![],
                    });
                }
                Err(_) => {
                    all_quotes.push(QuoteData {
                        pair,
                        quotes: vec![],
                    });
                }
            }
        }

        // Print separator between refreshes
        println!("\n{}", "=".repeat(80));
        println!(
            "╔══════════════════════════════════════════════════════════════════════════════╗"
        );
        println!(
            "║  NEAR Intents Quote Monitor | {} quotes from {} unique sources | {}s refresh  ║",
            total_quoters,
            unique_hashes.len(),
            interval_secs
        );
        println!(
            "╚══════════════════════════════════════════════════════════════════════════════╝"
        );
        println!(
            "Time: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!();

        // Summary table
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!(
            "│ {:<20} │ {:<5} │ {:<14} │ {:<30} │",
            "PAIR", "QTY", "BEST RATE", "BEST QUOTE HASH"
        );
        println!("├─────────────────────────────────────────────────────────────────────────────┤");

        for data in &all_quotes {
            if data.quotes.is_empty() {
                println!(
                    "│ {:<20} │ {:<5} │ {:<14} │ {:<30} │",
                    data.pair, "0", "-", "-"
                );
            } else {
                let best = &data.quotes[0];
                println!(
                    "│ {:<20} │ {:<5} │ {:<14} │ {:<30} │",
                    data.pair,
                    data.quotes.len(),
                    format!("1→{}", best.amount_out_fmt),
                    &best.quote_hash[..30.min(best.quote_hash.len())]
                );
            }
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");

        // Detailed quoter breakdown
        println!();
        println!("═══════════════════════════════════════════════════════════════════════════════");
        println!("                           DETAILED QUOTER DATA");
        println!("═══════════════════════════════════════════════════════════════════════════════");

        for data in &all_quotes {
            if !data.quotes.is_empty() {
                println!();
                println!("▶ {} ({} quoters)", data.pair, data.quotes.len());
                println!(
                    "  ─────────────────────────────────────────────────────────────────────────"
                );

                for (i, q) in data.quotes.iter().enumerate() {
                    let rank = if i == 0 { "★ BEST" } else { "       " };
                    let solver = q
                        .solver_id
                        .as_ref()
                        .map(|s| format!(" [solver: {}]", s))
                        .unwrap_or_default();

                    println!("  {} Quote #{}", rank, i + 1);
                    println!("       Hash:       {}", q.quote_hash);
                    println!(
                        "       Amount Out: {} (raw: {})",
                        q.amount_out_fmt, q.amount_out_raw
                    );
                    println!("       Amount In:  {}", q.amount_in_raw);
                    println!("       Expires:    {}", q.expires);

                    if !solver.is_empty() {
                        println!("       Solver:    {}", solver);
                    }

                    if !q.extra_fields.is_empty() {
                        println!("       Extra data:");
                        for (key, val) in &q.extra_fields {
                            println!("         {}: {}", key, val);
                        }
                    }
                    println!();
                }
            }
        }

        println!("───────────────────────────────────────────────────────────────────────────────");
        println!("Press Ctrl+C to stop");

        // Flush stdout to ensure output is visible
        let _ = std::io::stdout().flush();

        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match mode {
        "ws" | "websocket" => run_websocket().await,
        "quote" => run_quote(&args[2..], false).await,
        "watch" => run_quote(&args[2..], true).await,
        "monitor" => run_monitor(&args[2..]).await,
        "status" => run_status(&args[2..]).await,
        "tokens" => run_tokens(&args[2..]).await,
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
  monitor [interval]                     Live dashboard of all major pairs (default: 3s)
  tokens [filter]                        List available tokens
  status <intent_hash>                   Check the status of an intent
  ws, websocket                          Connect to Solver Relay WebSocket (solvers only)

Currency Format:
  Simple:         near, usdt, usdc, eth, btc
  Chain-prefixed: near:usdt, eth:usdc, base:usdc, arb:usdt

Supported Chains:
  near              NEAR Protocol (native)
  eth               Ethereum Mainnet (via OMFT)
  base              Base (via OMFT)
  arb, arbitrum     Arbitrum One (via OMFT)
  gnosis            Gnosis Chain (via OMFT)
  sol, solana       Solana (via OMFT)
  btc               Bitcoin (via OMFT)

Examples:
  # List all tokens
  near-intents-tools tokens

  # List USDC on all chains
  near-intents-tools tokens usdc

  # NEAR to USDT on NEAR (1 NEAR = 10^24 yoctoNEAR)
  near-intents-tools quote near usdt 1000000000000000000000000

  # NEAR to USDC on Ethereum
  near-intents-tools quote near eth:usdc 1000000000000000000000000

  # Watch quotes continuously
  near-intents-tools watch near usdt 1000000000000000000000000

  # Monitor all major pairs (live dashboard)
  near-intents-tools monitor

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
            "  near-intents-tools {} near eth:usdc 1000000000000000000000000",
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
                        let in_decimals = token_decimals(&quote.defuse_asset_identifier_in);
                        let out_decimals = token_decimals(&quote.defuse_asset_identifier_out);
                        let in_human = format_amount(&quote.amount_in, in_decimals);
                        let out_human = format_amount(&quote.amount_out, out_decimals);
                        println!("Quote #{}:", i + 1);
                        println!(
                            "  From:      {} {} (raw: {})",
                            in_human, in_name, quote.amount_in
                        );
                        println!(
                            "  To:        {} {} (raw: {})",
                            out_human, out_name, quote.amount_out
                        );
                        println!("  Hash:      {}", quote.quote_hash);
                        println!("  Expires:   {}", quote.expiration_time);
                        println!("  Asset In:  {}", quote.defuse_asset_identifier_in);
                        println!("  Asset Out: {}", quote.defuse_asset_identifier_out);
                        if let Some(ref solver) = quote.solver_id {
                            println!("  Solver:    {}", solver);
                        }
                        if !quote.extra.is_empty() {
                            println!("  Extra fields:");
                            for (key, val) in &quote.extra {
                                println!("    {}: {}", key, val);
                            }
                        }
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
