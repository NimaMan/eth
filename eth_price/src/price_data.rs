//! Normalized Price Data Structures.
//!
//! Defines the high-level containers for reporting price observations.
//!
//! # Key Structures
//!
//! *   **`PriceData`:** The primary output struct containing the price, timestamp, block number, and source metadata.
//! *   **`PricePair`:** Canonical descriptor of a trading pair (Base/Quote).
//! *   **`PriceSource`:** Enum detailing where the price came from (AMM, Oracle, etc.) and specific metadata (pool address, route).

use crate::price_data_models::{
    LiquidityMetrics, Pool, PoolState, PriceId, Protocol, RawChainPrice, Token,
};
use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// Canonical description of a token pair being priced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PricePair {
    pub label: String,
    pub base: Token,
    pub quote: Token,
}

impl PricePair {
    pub fn new(label: impl Into<String>, base: Token, quote: Token) -> Self {
        Self {
            label: label.into(),
            base,
            quote,
        }
    }
}

/// Precision-safe price record emitted by every reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub pair: PricePair,
    pub price: RawChainPrice,
    pub inverse_price: RawChainPrice,
    pub block_number: u64,
    pub timestamp: u64,
    pub source: PriceSource,
}

impl PriceData {
    pub fn new(
        pair: PricePair,
        price: RawChainPrice,
        block_number: u64,
        timestamp: u64,
        source: PriceSource,
    ) -> Self {
        let inverse_price = price.inverse();
        Self {
            pair,
            price,
            inverse_price,
            block_number,
            timestamp,
            source,
        }
    }

    /// Convert quote-per-base price to f64 for display/logging only.
    pub fn price_as_f64(&self) -> f64 {
        self.price
            .to_scaled_f64(self.pair.base.decimals, self.pair.quote.decimals)
    }

    /// Convert base-per-quote price to f64 for display/logging only.
    pub fn inverse_price_as_f64(&self) -> f64 {
        self.inverse_price
            .to_scaled_f64(self.pair.quote.decimals, self.pair.base.decimals)
    }

    pub fn protocol(&self) -> Protocol {
        self.source.protocol()
    }
}

/// Provenance metadata for a price quote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceSource {
    Amm(AmmPriceSource),
    Oracle(OraclePriceSource),
    Aggregator(AggregatorPriceSource),
    Simulation(SimulationPriceSource),
}

impl PriceSource {
    pub fn protocol(&self) -> Protocol {
        match self {
            PriceSource::Amm(data) => data.protocol,
            PriceSource::Oracle(data) => data.price_id.protocol,
            PriceSource::Aggregator(data) => data.protocol,
            PriceSource::Simulation(data) => data.price_id.protocol,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmPriceSource {
    pub protocol: Protocol,
    pub price_id: PriceId,
    pub pool: Pool,
    pub pool_state: PoolState,
    pub liquidity: LiquidityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OraclePriceSource {
    pub price_id: PriceId,
    pub feed_address: Address,
    pub answer: U256,
    pub answer_decimals: u8,
    pub round_id: Option<U256>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorPriceSource {
    pub protocol: Protocol,
    pub routes: Vec<String>,
    pub amount_in: U256,
    pub amount_out: U256,
    pub estimated_gas: Option<U256>,
    pub from_token: Token,
    pub to_token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationDirection {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationPriceSource {
    pub price_id: PriceId,
    pub direction: SimulationDirection,
    pub route: String,
    pub account: Address,
    pub input_token: Token,
    pub output_token: Token,
    pub input_amount: U256,
    pub output_amount: U256,
    pub tax_percent: Option<f64>,
    pub liquidity: Option<LiquidityMetrics>,
}
