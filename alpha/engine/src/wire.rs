//! HTTP wire types for the token server API and observation payloads.
//!
//! These types mirror the JSON responses from `eth_token_server` and are used
//! by both the live trader (`eth_alpha_trader`) and the backtest replay logic
//! to convert wire representations into `eth_alpha_core` domain types.

use std::str::FromStr;

use alloy_primitives::{Address, B256};
use eth_alpha_core::{
    ids::TokenPoolId,
    market::{PoolProtocol, PoolSnapshot},
    risk::{RiskEvent, RiskKind, RiskSeverity},
};
use eyre::{eyre, Result};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LiveStatusResponse {
    pub progress: LiveProgressWire,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LivePoolListResponse {
    pub count: usize,
    pub pools: Vec<PoolWire>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LiveProgressWire {
    pub status: String,
    pub current_block: Option<u64>,
    pub blocks_processed: u64,
    pub warmup_total_blocks: u64,
    pub tracked_tokens: usize,
    #[serde(default)]
    pub tracked_pools: usize,
    #[serde(default)]
    pub tracked_v2_pools: usize,
    #[serde(default)]
    pub tracked_v3_pools: usize,
    #[serde(default)]
    pub tracked_v4_pools: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolWire {
    pub token_address: String,
    pub pool_address: String,
    pub protocol: String,
    pub denom_address: Option<String>,
    pub denom_symbol: Option<String>,
    pub currency: Option<String>,
    pub denom_reserve: Option<f64>,
    pub token_reserve: Option<f64>,
    pub price: Option<f64>,
    pub creation_block: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub runtime_state: Option<PoolRuntimeStateWire>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub is_scam: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolRuntimeStateWire {
    pub last_update_block: Option<u64>,
    pub last_sync_block: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MempoolSignalsResponse {
    pub count: usize,
    pub signals: Vec<MempoolSignalWire>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MempoolSignalWire {
    pub signal_id: String,
    pub signal_type: String,
    pub detection_timestamp: Option<String>,
    pub detection_tx_hash: Option<String>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub headline: Option<String>,
    pub flag: Option<String>,
}

impl PoolWire {
    pub fn latest_block_number(&self) -> Option<u64> {
        self.latest_block_number
            .filter(|block| *block > 0)
            .or_else(|| {
                self.runtime_state.as_ref().and_then(|state| {
                    state
                        .last_update_block
                        .filter(|block| *block > 0)
                        .or_else(|| state.last_sync_block.filter(|block| *block > 0))
                })
            })
            .or_else(|| self.creation_block.filter(|block| *block > 0))
    }

    pub fn pool_identity(&self) -> String {
        self.pool_address.clone()
    }

    pub fn to_pool_snapshot(&self) -> Result<PoolSnapshot> {
        let token_address = parse_address(&self.token_address)?;
        let pool_id = TokenPoolId::new(token_address, self.pool_identity());
        let denom_reserve = required_pool_float(self.denom_reserve, "denom_reserve", self)?;
        let token_reserve = self.token_reserve.unwrap_or_default();
        let Some(latest_block) = self.latest_block_number() else {
            return Err(eyre!(
                "pool {} for token {} has no latest block",
                self.pool_address,
                self.token_address
            ));
        };
        Ok(PoolSnapshot {
            address: pool_id,
            token_address,
            protocol: parse_protocol(&self.protocol),
            denom_address: self
                .denom_address
                .as_deref()
                .and_then(parse_optional_address),
            denom_symbol: self.denom_symbol(),
            denom_reserve: decimal_from_f64(denom_reserve),
            token_reserve: decimal_from_f64(token_reserve),
            price_denom_per_token: self.price.map(decimal_from_f64),
            token_decimals: None,
            latest_block,
            can_buy: self.can_buy,
            can_sell: self.can_sell,
            is_scam: self.is_scam,
        })
    }

    pub fn denom_symbol(&self) -> Option<String> {
        self.denom_symbol
            .as_deref()
            .or(self.currency.as_deref())
            .map(str::trim)
            .filter(|symbol| !symbol.is_empty() && !symbol.starts_with("0x"))
            .map(str::to_ascii_uppercase)
    }
}

impl LiveProgressWire {
    pub fn tracked_pool_count(&self) -> usize {
        if self.tracked_pools > 0 {
            self.tracked_pools
        } else {
            self.tracked_v2_pools + self.tracked_v3_pools + self.tracked_v4_pools
        }
    }
}

impl MempoolSignalWire {
    pub fn to_risk_event(&self) -> Result<Option<RiskEvent>> {
        let Some(token_address) = self.token_address.as_ref() else {
            return Ok(None);
        };
        let token_address = parse_address(token_address)?;
        let pool_address = self
            .pool_address
            .as_ref()
            .map(|value| TokenPoolId::new(token_address, value));
        let pending_tx_hash = self
            .detection_tx_hash
            .as_ref()
            .map(|value| B256::from_str(value))
            .transpose()
            .map_err(|error| eyre!("invalid tx hash: {error}"))?;
        let (kind, severity) = signal_kind_and_severity(self);
        Ok(Some(RiskEvent {
            kind,
            severity,
            token_address,
            pool_address,
            pending_tx_hash,
            observed_block: None,
            message: self.message(),
        }))
    }

    pub fn message(&self) -> String {
        match (&self.detection_timestamp, &self.headline) {
            (Some(ts), Some(headline)) => format!("{headline} at {ts}"),
            (Some(ts), None) => format!("{} at {ts}", self.signal_type),
            (None, Some(headline)) => headline.clone(),
            (None, None) => self.signal_type.clone(),
        }
    }
}

pub fn signal_kind_and_severity(signal: &MempoolSignalWire) -> (RiskKind, RiskSeverity) {
    match signal.signal_type.as_str() {
        "trading_enabled" => (RiskKind::TradingEnabled, RiskSeverity::Info),
        "liquidity_removal" => (RiskKind::LiquidityRemoval, RiskSeverity::Critical),
        "lp_approval" => (RiskKind::LpApproval, RiskSeverity::Warning),
        "honeypot_signal" | "sell_blocked_signal" => (RiskKind::Honeypot, RiskSeverity::Critical),
        "tax_signal" => {
            let critical = signal
                .flag
                .as_deref()
                .map(is_critical_tax_bucket)
                .unwrap_or(false);
            (
                RiskKind::TaxChange,
                if critical {
                    RiskSeverity::Critical
                } else {
                    RiskSeverity::Warning
                },
            )
        }
        other => (RiskKind::Custom(other.to_string()), RiskSeverity::Warning),
    }
}

pub fn is_critical_tax_bucket(bucket: &str) -> bool {
    matches!(
        bucket.trim().to_ascii_lowercase().as_str(),
        "high_tax" | "extreme_tax" | "high" | "extreme"
    )
}

pub fn parse_protocol(value: &str) -> PoolProtocol {
    match value.to_ascii_lowercase().as_str() {
        "uniswapv2" | "uniswap_v2" | "v2" => PoolProtocol::UniswapV2,
        "uniswapv3" | "uniswap_v3" | "v3" => PoolProtocol::UniswapV3,
        "uniswapv4" | "uniswap_v4" | "v4" => PoolProtocol::UniswapV4,
        other => PoolProtocol::Unknown(other.to_string()),
    }
}

pub fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|error| eyre!("invalid address {value}: {error}"))
}

pub fn parse_optional_address(value: &str) -> Option<Address> {
    Address::from_str(value).ok()
}

pub fn required_pool_float(value: Option<f64>, field: &str, pool: &PoolWire) -> Result<f64> {
    value.ok_or_else(|| {
        eyre!(
            "pool {} for token {} is missing {field}",
            pool.pool_address,
            pool.token_address
        )
    })
}

pub fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or(Decimal::ZERO)
}
