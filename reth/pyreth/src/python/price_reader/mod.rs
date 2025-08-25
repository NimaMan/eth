/// Python bindings for eth_prices - Ethereum price reading from DEX and CEX sources
/// 
/// This module provides a unified Python interface for reading prices from:
/// - On-chain DEX sources (Uniswap, Curve, Balancer, etc.)
/// - CEX WebSocket feeds (Kraken, Binance, etc.)
/// - Chainlink oracles
/// 
/// All functionality is exposed through the PyEthPriceClient class.

pub mod client;
pub mod price_data;
pub mod dex_reader;
pub mod cex_reader;
pub mod arbitrage;

pub use client::PyEthPriceClient;
pub use price_data::PyPriceData;
pub use dex_reader::PyDexReader;
pub use cex_reader::PyCexReader;
pub use arbitrage::PyArbitrageDetector;