//! HTTP wire types for the token server API and observation payloads.
//!
//! These types mirror the JSON responses from `eth_chain_server` and are used
//! by both the live trader (`eth_alpha_trader`) and the backtest replay logic
//! to convert wire representations into `eth_alpha_core` domain types.

use std::str::FromStr;

use alloy_primitives::{Address, B256};
use eth_alpha_core::{
    ids::TokenPoolId,
    market::{PoolProtocol, PoolSnapshot, UniswapV4PoolKeySnapshot},
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
    #[serde(default)]
    pub pool_id: Option<String>,
    #[serde(default)]
    pub pool_manager_address: Option<String>,
    #[serde(default)]
    pub currency0: Option<String>,
    #[serde(default)]
    pub currency1: Option<String>,
    #[serde(default)]
    pub fee_tier: Option<u32>,
    #[serde(default)]
    pub tick_spacing: Option<i32>,
    #[serde(default)]
    pub hooks: Option<String>,
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
    #[serde(default)]
    pub last_update_block: Option<u64>,
    #[serde(default)]
    pub last_sync_block: Option<u64>,
    #[serde(default)]
    pub can_buy: Option<bool>,
    #[serde(default)]
    pub can_sell: Option<bool>,
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
        let protocol = parse_protocol(&self.protocol);
        let Some(latest_block) = self.latest_block_number() else {
            return Err(eyre!(
                "pool {} for token {} has no latest block",
                self.pool_address,
                self.token_address
            ));
        };
        let runtime_can_buy = self.runtime_state.as_ref().and_then(|state| state.can_buy);
        let runtime_can_sell = self.runtime_state.as_ref().and_then(|state| state.can_sell);
        let requires_runtime_trading_flags = matches!(protocol, PoolProtocol::UniswapV4);
        let can_buy = runtime_can_buy.unwrap_or_else(|| {
            if requires_runtime_trading_flags {
                false
            } else {
                self.can_buy
            }
        });
        let can_sell = runtime_can_sell.unwrap_or_else(|| {
            if requires_runtime_trading_flags {
                false
            } else {
                self.can_sell
            }
        });
        Ok(PoolSnapshot {
            address: pool_id,
            token_address,
            protocol,
            denom_address: self
                .denom_address
                .as_deref()
                .and_then(parse_optional_address),
            denom_symbol: self.denom_symbol(),
            denom_reserve: decimal_from_f64(denom_reserve),
            token_reserve: decimal_from_f64(token_reserve),
            price_denom_per_token: self.price.map(decimal_from_f64),
            token_decimals: None,
            fee_tier: self.fee_tier,
            uniswap_v4: self.uniswap_v4_pool_key()?,
            latest_block,
            can_buy,
            can_sell,
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

    fn uniswap_v4_pool_key(&self) -> Result<Option<UniswapV4PoolKeySnapshot>> {
        if !matches!(parse_protocol(&self.protocol), PoolProtocol::UniswapV4) {
            return Ok(None);
        }

        let pool_manager = self
            .pool_manager_address
            .as_deref()
            .or_else(|| self.pool_address.split('#').next())
            .ok_or_else(|| eyre!("v4 pool {} has no pool manager", self.pool_address))
            .and_then(parse_address)?;
        let pool_id = self
            .pool_id
            .as_deref()
            .or_else(|| self.pool_address.split('#').nth(1))
            .ok_or_else(|| eyre!("v4 pool {} has no pool id", self.pool_address))
            .and_then(parse_hash)?;
        let currency0 = self
            .currency0
            .as_deref()
            .ok_or_else(|| eyre!("v4 pool {} has no currency0", self.pool_address))
            .and_then(parse_address)?;
        let currency1 = self
            .currency1
            .as_deref()
            .ok_or_else(|| eyre!("v4 pool {} has no currency1", self.pool_address))
            .and_then(parse_address)?;
        let fee = self
            .fee_tier
            .ok_or_else(|| eyre!("v4 pool {} has no fee tier", self.pool_address))?;
        let tick_spacing = self
            .tick_spacing
            .ok_or_else(|| eyre!("v4 pool {} has no tick spacing", self.pool_address))?;
        let hooks = self
            .hooks
            .as_deref()
            .ok_or_else(|| eyre!("v4 pool {} has no hooks address", self.pool_address))
            .and_then(parse_address)?;

        Ok(Some(UniswapV4PoolKeySnapshot {
            pool_manager,
            pool_id,
            currency0,
            currency1,
            fee,
            tick_spacing,
            hooks,
        }))
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
        "lp_approval" | "lp_position_approval" => (
            RiskKind::LpApproval,
            if signal_flag_is_true(signal) {
                RiskSeverity::Critical
            } else {
                RiskSeverity::Warning
            },
        ),
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

fn signal_flag_is_true(signal: &MempoolSignalWire) -> bool {
    signal
        .flag
        .as_deref()
        .map(|flag| {
            matches!(
                flag.trim().to_ascii_lowercase().as_str(),
                "true" | "critical"
            )
        })
        .unwrap_or(false)
}

pub fn is_critical_tax_bucket(bucket: &str) -> bool {
    matches!(
        bucket.trim().to_ascii_lowercase().as_str(),
        "high_tax" | "extreme_tax" | "high" | "extreme"
    )
}

pub fn parse_protocol(value: &str) -> PoolProtocol {
    match value.to_ascii_lowercase().as_str() {
        "uniswapv2" | "uniswap_v2" | "uniswap-v2" | "v2" => PoolProtocol::UniswapV2,
        "uniswapv3" | "uniswap_v3" | "uniswap-v3" | "v3" => PoolProtocol::UniswapV3,
        "uniswapv4" | "uniswap_v4" | "uniswap-v4" | "v4" => PoolProtocol::UniswapV4,
        "sushi" | "sushiswap" | "sushi_v2" | "sushi-v2" => {
            PoolProtocol::Unknown("sushi".to_string())
        }
        "pancake" | "pancakeswap" | "pancake_v2" | "pancake-v2" | "pancakeswap_v2"
        | "pancakeswap-v2" => PoolProtocol::PancakeSwapV2,
        other => PoolProtocol::Unknown(other.to_string()),
    }
}

pub fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|error| eyre!("invalid address {value}: {error}"))
}

pub fn parse_hash(value: &str) -> Result<B256> {
    B256::from_str(value).map_err(|error| eyre!("invalid hash {value}: {error}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_wire_with_flags(top_level_can_buy: bool, top_level_can_sell: bool) -> PoolWire {
        PoolWire {
            token_address: "0x1111111111111111111111111111111111111111".to_string(),
            pool_address: "0x2222222222222222222222222222222222222222".to_string(),
            protocol: "UNISWAP-V2".to_string(),
            pool_id: None,
            pool_manager_address: None,
            currency0: None,
            currency1: None,
            fee_tier: None,
            tick_spacing: None,
            hooks: None,
            denom_address: Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string()),
            denom_symbol: Some("WETH".to_string()),
            currency: None,
            denom_reserve: Some(1.0),
            token_reserve: Some(100.0),
            price: Some(0.01),
            creation_block: Some(10),
            latest_block_number: Some(12),
            runtime_state: None,
            can_buy: top_level_can_buy,
            can_sell: top_level_can_sell,
            is_scam: false,
        }
    }

    #[test]
    fn pool_snapshot_prefers_runtime_state_trading_flags() {
        let mut wire = pool_wire_with_flags(true, true);
        wire.runtime_state = Some(PoolRuntimeStateWire {
            last_update_block: Some(12),
            last_sync_block: Some(12),
            can_buy: Some(false),
            can_sell: Some(false),
        });

        let snapshot = wire.to_pool_snapshot().expect("pool snapshot");

        assert!(!snapshot.can_buy);
        assert!(!snapshot.can_sell);
    }

    #[test]
    fn pool_snapshot_falls_back_to_top_level_trading_flags() {
        let snapshot = pool_wire_with_flags(true, false)
            .to_pool_snapshot()
            .expect("pool snapshot");

        assert!(snapshot.can_buy);
        assert!(!snapshot.can_sell);
    }

    #[test]
    fn v4_pool_snapshot_requires_runtime_trading_flags() {
        let mut wire = pool_wire_with_flags(true, true);
        wire.protocol = "UNISWAP-V4".to_string();
        wire.pool_id = Some(format!("0x{:064x}", 1));
        wire.pool_manager_address = Some("0x3333333333333333333333333333333333333333".to_string());
        wire.currency0 = Some("0x1111111111111111111111111111111111111111".to_string());
        wire.currency1 = Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string());
        wire.fee_tier = Some(3000);
        wire.tick_spacing = Some(60);
        wire.hooks = Some("0x0000000000000000000000000000000000000000".to_string());

        let snapshot = wire.to_pool_snapshot().expect("pool snapshot");

        assert!(!snapshot.can_buy);
        assert!(!snapshot.can_sell);
    }

    #[test]
    fn v4_pool_snapshot_uses_runtime_trading_flags_when_present() {
        let mut wire = pool_wire_with_flags(false, false);
        wire.protocol = "UNISWAP-V4".to_string();
        wire.pool_id = Some(format!("0x{:064x}", 1));
        wire.pool_manager_address = Some("0x3333333333333333333333333333333333333333".to_string());
        wire.currency0 = Some("0x1111111111111111111111111111111111111111".to_string());
        wire.currency1 = Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string());
        wire.fee_tier = Some(3000);
        wire.tick_spacing = Some(60);
        wire.hooks = Some("0x0000000000000000000000000000000000000000".to_string());
        wire.runtime_state = Some(PoolRuntimeStateWire {
            last_update_block: Some(12),
            last_sync_block: Some(12),
            can_buy: Some(true),
            can_sell: Some(true),
        });

        let snapshot = wire.to_pool_snapshot().expect("pool snapshot");

        assert!(snapshot.can_buy);
        assert!(snapshot.can_sell);
    }

    #[test]
    fn parse_protocol_accepts_pancakeswap_v2_spellings() {
        assert_eq!(
            parse_protocol("pancakeswap-v2"),
            PoolProtocol::PancakeSwapV2
        );
        assert_eq!(
            parse_protocol("pancakeswap_v2"),
            PoolProtocol::PancakeSwapV2
        );
    }

    #[test]
    fn lp_position_approval_flag_true_maps_to_critical_lp_approval() {
        let signal = MempoolSignalWire {
            signal_id: "1".to_string(),
            signal_type: "lp_position_approval".to_string(),
            detection_timestamp: None,
            detection_tx_hash: None,
            token_address: Some("0x1111111111111111111111111111111111111111".to_string()),
            pool_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            headline: None,
            flag: Some("true".to_string()),
        };

        let event = signal
            .to_risk_event()
            .expect("risk event")
            .expect("non-empty event");

        assert_eq!(event.kind, RiskKind::LpApproval);
        assert_eq!(event.severity, RiskSeverity::Critical);
    }
}
