/// Python bindings for eth_prices - Ethereum price reading from DEX sources
///
/// This module provides a Python interface for reading prices from:
/// - On-chain DEX sources (Uniswap, Curve, Balancer, etc.)
/// - Chainlink oracles
///
/// Core functionality is exposed through the PyEthPriceClient class.
/// Additional modules (dex_reader, cex_reader, arbitrage) are disabled
/// to focus on rust-side implementation first.
pub mod client;
pub mod price_data;
// pub mod dex_reader;  // Disabled - focus on rust side first
// pub mod cex_reader;   // Disabled - focus on rust side first
// pub mod arbitrage;    // Disabled - focus on rust side first

pub use client::PyEthPriceClient;
pub use price_data::PyPriceData;
// pub use dex_reader::PyDexReader;
// pub use cex_reader::PyCexReader;
// pub use arbitrage::PyArbitrageDetector;
