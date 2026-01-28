# NEAR Intents Tools

A collection of tools for monitoring and interacting with the NEAR Intents
protocol.

## Features

- **WebSocket Listener**: Real-time monitoring of intents on the NEAR Intents
  Solver Relay
- **Intent Parsing**: Structured parsing of intent messages and quotes

## Quick Start

```bash
# Build the project
make build

# Run the WebSocket listener
make run
```

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

## Architecture

The tool connects to the NEAR Intents Solver Relay WebSocket endpoint
(`wss://solver-relay-v2.chaindefuser.com/ws`) and listens for:

- Quote requests and responses
- Published intents
- Intent status updates

### API Access Note

The Solver Relay WebSocket endpoint (`wss://solver-relay-v2.chaindefuser.com/ws`)
is restricted to registered solvers. Connecting without proper registration will
result in a 403 Forbidden error.

To become a solver, refer to the [Create a Solver](https://docs.near.org/chain-abstraction/intents/solvers)
documentation.

For public intent monitoring, consider using the [Intents Explorer API](https://docs.near-intents.org/near-intents/integration/distribution-channels/intents-explorer-api)
which provides historical intent data with JWT authentication.

## References

- [NEAR Intents Documentation](https://docs.near-intents.org)
- [NEAR Intents Overview](https://docs.near.org/chain-abstraction/intents/overview)
- [Solver Relay API](https://docs.near-intents.org/near-intents/market-makers/bus/solver-relay)
- [Intents Explorer API](https://docs.near-intents.org/near-intents/integration/distribution-channels/intents-explorer-api)
- [Create a Solver](https://docs.near.org/chain-abstraction/intents/solvers)

## License

MIT
