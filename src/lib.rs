//! # NEAR Intents Tools
//!
//! A Rust library for interacting with the NEAR Intents Solver Relay protocol.
//!
//! ## Overview
//!
//! NEAR Intents is a multichain transaction protocol where users specify desired
//! outcomes and solvers compete to provide the best solution. This library provides:
//!
//! - **RPC Client**: HTTP client for requesting quotes and publishing intents
//! - **WebSocket Client**: Real-time event streaming for solvers
//! - **Type Definitions**: Strongly-typed structs for all API interactions
//!
//! ## Quick Start
//!
//! ### Requesting a Quote (RPC)
//!
//! ```rust,no_run
//! use near_intents_tools::rpc::SolverRelayRpcClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = SolverRelayRpcClient::new();
//!
//!     let quotes = client.quote(
//!         "nep141:wrap.near",
//!         "nep141:usdt.tether-token.near",
//!         Some("1000000000000000000000000"), // 1 NEAR in yoctoNEAR
//!         None,
//!         Some(60000), // 1 minute deadline
//!     ).await?;
//!
//!     for quote in quotes {
//!         println!("Quote: {} -> {}", quote.amount_in, quote.amount_out);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Listening for Quote Requests (WebSocket - Solver Only)
//!
//! ```rust,no_run
//! use near_intents_tools::ws::SolverRelayWsClient;
//! use near_intents_tools::types::{SolverEvent, SubscriptionType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = SolverRelayWsClient::new();
//!     let (mut events, _response_tx) = client.connect(SubscriptionType::Quote).await?;
//!
//!     while let Some(event) = events.recv().await {
//!         match event {
//!             SolverEvent::QuoteRequest(req) => {
//!                 println!("Quote request: {} -> {}",
//!                     req.defuse_asset_identifier_in,
//!                     req.defuse_asset_identifier_out);
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## API Endpoints
//!
//! - **RPC**: `https://solver-relay-v2.chaindefuser.com/rpc`
//! - **WebSocket**: `wss://solver-relay-v2.chaindefuser.com/ws`
//!
//! Note: The WebSocket endpoint requires solver registration. Public connections
//! will receive 403 Forbidden.
//!
//! ## References
//!
//! - [NEAR Intents Documentation](https://docs.near-intents.org)
//! - [Solver Relay API](https://docs.near-intents.org/near-intents/market-makers/bus/solver-relay)

pub mod error;
pub mod rpc;
pub mod types;
pub mod ws;

pub use error::{Result, SolverRelayError};
pub use rpc::SolverRelayRpcClient;
pub use types::*;
pub use ws::{QuoteResponseBuilder, SolverRelayWsClient};
