# NEAR Intents Tools

A Rust library and CLI for interacting with the NEAR Intents Solver Relay
protocol.

## Features

- **RPC Client**: HTTP client for requesting quotes, publishing intents, and
  checking status
- **WebSocket Client**: Real-time event streaming for solvers (quote requests,
  status updates)
- **Type Definitions**: Strongly-typed structs for all API interactions
- **CLI**: Command-line interface for testing and debugging

## Quick Start

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
near-intents-tools = { git = "https://github.com/dannywillems/near-intents-tools" }
```

### Request a Quote

```rust
use near_intents_tools::rpc::SolverRelayRpcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SolverRelayRpcClient::new();

    let quotes = client.quote(
        "nep141:wrap.near",                    // Source asset
        "nep141:usdt.tether-token.near",       // Target asset
        Some("1000000000000000000000000"),     // 1 NEAR in yoctoNEAR
        None,
        Some(60000),                           // 1 minute deadline
    ).await?;

    for quote in quotes {
        println!("Quote: {} -> {} (expires: {})",
            quote.amount_in, quote.amount_out, quote.expiration_time);
    }

    Ok(())
}
```

### Check Intent Status

```rust
use near_intents_tools::rpc::SolverRelayRpcClient;

let client = SolverRelayRpcClient::new();
let status = client.get_status("intent-hash-here").await?;

println!("Status: {:?}", status.status);
```

### CLI Usage

```bash
# Build the project
make build

# Request a quote (RPC)
cargo run -- rpc

# Connect to WebSocket (requires solver registration)
cargo run -- ws
```

## API Endpoints

| Endpoint | URL | Access |
|----------|-----|--------|
| RPC | `https://solver-relay-v2.chaindefuser.com/rpc` | Public |
| WebSocket | `wss://solver-relay-v2.chaindefuser.com/ws` | Solvers only |

### RPC Methods

| Method | Description |
|--------|-------------|
| `quote` | Request quotes for token swaps |
| `publish_intent` | Submit signed intent for execution |
| `get_status` | Query intent execution status |

### WebSocket Events (Solvers)

| Event | Description |
|-------|-------------|
| `QuoteRequest` | New quote request from a user |
| `QuoteStatus` | Quote has been executed |

## Requirements

- Rust 1.75+
- Nightly Rust (for formatting)
- taplo (for TOML formatting)

### Setup

```bash
make setup
```

## Development

```bash
# Format code
make format

# Run linter
make lint

# Run tests
make test

# Check formatting
make check-format
```

## API Access Note

The Solver Relay WebSocket endpoint is restricted to registered solvers.
Connecting without proper registration will result in 403 Forbidden.

To become a market maker/solver, refer to the
[Market Makers documentation](https://docs.near-intents.org/near-intents/market-makers).

For public intent monitoring, consider using the
[Intents Explorer API](https://docs.near-intents.org/near-intents/integration/distribution-channels/intents-explorer-api)
which provides historical intent data with JWT authentication.

## References

- [NEAR Intents Documentation](https://docs.near-intents.org)
- [Solver Relay API](https://docs.near-intents.org/near-intents/market-makers/bus/solver-relay)
- [Market Makers](https://docs.near-intents.org/near-intents/market-makers)
- [Intents Explorer API](https://docs.near-intents.org/near-intents/integration/distribution-channels/intents-explorer-api)

## License

MIT
