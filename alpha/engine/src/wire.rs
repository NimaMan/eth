//! HTTP wire types for the token server API and observation payloads.
//!
//! These types mirror the JSON responses from `eth_chain_server` and are used
//! by the live trader entrypoints and the backtest replay logic
//! to convert wire representations into `eth_alpha_core` domain types.

use std::str::FromStr;

use alloy_primitives::{Address, B256};
use eth_alpha_core::{
    ids::TokenPoolId,
    market::{PoolProtocol, PoolSnapshot, UniswapV4PoolKeySnapshot},
    mempool_entry::MEMPOOL_ENTRY_EVIDENCE_KEY,
    risk::{RiskEvent, RiskKind, RiskSeverity, RISK_SOURCE_MEMPOOL_SIGNAL},
};
use eyre::{eyre, Result};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Deserialize, Serialize)]
pub struct LiveStatusResponse {
    pub progress: LiveProgressWire,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GasRankSamplesResponse {
    pub source: String,
    pub requested_blocks: usize,
    pub available_recent_blocks: usize,
    pub latest_block: Option<u64>,
    pub latest_block_hash: Option<String>,
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
    #[serde(default)]
    pub token_decimals: Option<u8>,
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
    pub initial_price: Option<f64>,
    pub price_ratio_to_initial: Option<f64>,
    pub creation_block: Option<u64>,
    #[serde(default)]
    pub can_buy_block: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub runtime_state: Option<PoolRuntimeStateWire>,
    #[serde(default)]
    pub lp_last_approval_block: Option<u64>,
    #[serde(default)]
    pub lp_last_approval: Option<Value>,
    #[serde(default)]
    pub lp_approval_count: Option<u64>,
    #[serde(default)]
    pub lp_approved_percentage: Option<f64>,
    #[serde(default)]
    pub liquidity_removal: bool,
    #[serde(default)]
    pub liquidity_removal_block: Option<u64>,
    #[serde(default)]
    pub liquidity_removal_tx_hash: Option<String>,
    #[serde(default)]
    pub liquidity_removal_label: Option<String>,
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
    #[serde(default)]
    pub signal_source: Option<String>,
    #[serde(default)]
    pub signal_created_at: Option<String>,
    #[serde(default)]
    pub mempool_first_seen_at: Option<String>,
    #[serde(default)]
    pub mempool_first_seen_ms: Option<i64>,
    pub detection_timestamp: Option<String>,
    pub detection_tx_hash: Option<String>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    #[serde(default)]
    pub pool_type: Option<String>,
    #[serde(default)]
    pub creator_address: Option<String>,
    #[serde(default)]
    pub subject_address: Option<String>,
    pub headline: Option<String>,
    #[serde(default)]
    pub value_1: Option<String>,
    #[serde(default)]
    pub value_2: Option<String>,
    pub flag: Option<String>,
    #[serde(default)]
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mempool_entry_evidence: Option<Value>,
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
            initial_price_denom_per_token: self.initial_price.map(decimal_from_f64),
            price_ratio_to_initial: self.price_ratio_to_initial.map(decimal_from_f64),
            creation_block: self.creation_block.filter(|block| *block > 0),
            token_decimals: self.token_decimals,
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
            source: Some(RISK_SOURCE_MEMPOOL_SIGNAL.to_string()),
            token_address,
            pool_address,
            pending_tx_hash,
            observed_block: None,
            message: self.message(),
            evidence: Some(self.risk_evidence()),
        }))
    }

    pub fn risk_evidence(&self) -> Value {
        let mut evidence = Map::new();
        evidence.insert("signal_id".to_string(), json!(self.signal_id));
        evidence.insert("signal_type".to_string(), json!(self.signal_type));
        if let Some(value) = self.signal_source.as_ref() {
            evidence.insert("signal_source".to_string(), json!(value));
        }
        if let Some(value) = self.signal_created_at.as_ref() {
            evidence.insert("signal_created_at".to_string(), json!(value));
        }
        if let Some(value) = self.mempool_first_seen_at.as_ref() {
            evidence.insert("mempool_first_seen_at".to_string(), json!(value));
        }
        if let Some(value) = self.mempool_first_seen_ms {
            evidence.insert("mempool_first_seen_ms".to_string(), json!(value));
        }
        if let Some(value) = self.detection_timestamp.as_ref() {
            evidence.insert("detection_timestamp".to_string(), json!(value));
        }
        if let Some(value) = self.detection_tx_hash.as_ref() {
            evidence.insert("detection_tx_hash".to_string(), json!(value));
        }
        if let Some(value) = self.pool_type.as_ref() {
            evidence.insert("pool_type".to_string(), json!(value));
        }
        if let Some(value) = self.creator_address.as_ref() {
            evidence.insert("creator_address".to_string(), json!(value));
        }
        if let Some(value) = self.subject_address.as_ref() {
            evidence.insert("subject_address".to_string(), json!(value));
        }
        if let Some(value) = self.lp_approval_pct_value() {
            evidence.insert("approved_share_pct".to_string(), value);
        }
        if !self.payload.is_null() {
            evidence.insert("signal_payload".to_string(), self.payload.clone());
        }
        if let Some(value) = self.mempool_entry_evidence.as_ref() {
            evidence.insert(MEMPOOL_ENTRY_EVIDENCE_KEY.to_string(), value.clone());
        }
        Value::Object(evidence)
    }

    pub fn message(&self) -> String {
        let base = match (&self.detection_timestamp, &self.headline) {
            (Some(ts), Some(headline)) => format!("{headline} at {ts}"),
            (Some(ts), None) => format!("{} at {ts}", self.signal_type),
            (None, Some(headline)) => headline.clone(),
            (None, None) => self.signal_type.clone(),
        };

        if self.is_lp_approval_signal() {
            if let Some(approved_pct) = self.lp_approval_pct() {
                return format!("{base} approved_pct={approved_pct}%");
            }
        }

        base
    }

    fn is_lp_approval_signal(&self) -> bool {
        matches!(
            self.signal_type.as_str(),
            "lp_approval" | "lp_position_approval"
        )
    }

    fn lp_approval_pct(&self) -> Option<String> {
        self.lp_approval_pct_value().and_then(|value| match value {
            Value::Number(number) => Some(number.to_string()),
            Value::String(text) => {
                let trimmed = text.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }
            _ => None,
        })
    }

    fn lp_approval_pct_value(&self) -> Option<Value> {
        [
            "approved_share_pct",
            "approval_percentage",
            "position_share_pct",
        ]
        .into_iter()
        .find_map(|key| self.payload.get(key).cloned())
        .or_else(|| {
            self.value_1
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| json!(value))
        })
    }
}

pub fn signal_kind_and_severity(signal: &MempoolSignalWire) -> (RiskKind, RiskSeverity) {
    match signal.signal_type.as_str() {
        "trading_enabled" => (RiskKind::TradingEnabled, RiskSeverity::Info),
        "liquidity_removal" => (RiskKind::MempoolLiquidityRemoval, RiskSeverity::Critical),
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
    PoolProtocol::from_label(value)
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
            token_decimals: Some(9),
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
            initial_price: Some(0.005),
            price_ratio_to_initial: Some(2.0),
            creation_block: Some(10),
            can_buy_block: Some(10),
            latest_block_number: Some(12),
            runtime_state: None,
            lp_last_approval_block: None,
            lp_last_approval: None,
            lp_approval_count: None,
            lp_approved_percentage: None,
            liquidity_removal: false,
            liquidity_removal_block: None,
            liquidity_removal_tx_hash: None,
            liquidity_removal_label: None,
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
        assert_eq!(
            snapshot.initial_price_denom_per_token,
            Some(decimal_from_f64(0.005))
        );
        assert_eq!(snapshot.price_ratio_to_initial, Some(decimal_from_f64(2.0)));
        assert_eq!(snapshot.token_decimals, Some(9));
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
            signal_source: None,
            signal_created_at: None,
            mempool_first_seen_at: None,
            mempool_first_seen_ms: None,
            detection_timestamp: None,
            detection_tx_hash: None,
            token_address: Some("0x1111111111111111111111111111111111111111".to_string()),
            pool_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            pool_type: None,
            creator_address: None,
            subject_address: None,
            headline: None,
            value_1: None,
            value_2: None,
            flag: Some("true".to_string()),
            payload: Value::Null,
            mempool_entry_evidence: None,
        };

        let event = signal
            .to_risk_event()
            .expect("risk event")
            .expect("non-empty event");

        assert_eq!(event.kind, RiskKind::LpApproval);
        assert_eq!(event.severity, RiskSeverity::Critical);
        assert_eq!(event.source.as_deref(), Some(RISK_SOURCE_MEMPOOL_SIGNAL));
    }

    #[test]
    fn liquidity_removal_signal_maps_to_mempool_specific_risk_kind() {
        let signal = MempoolSignalWire {
            signal_id: "1".to_string(),
            signal_type: "liquidity_removal".to_string(),
            signal_source: None,
            signal_created_at: None,
            mempool_first_seen_at: None,
            mempool_first_seen_ms: None,
            detection_timestamp: None,
            detection_tx_hash: Some(
                "0x3333333333333333333333333333333333333333333333333333333333333333".to_string(),
            ),
            token_address: Some("0x1111111111111111111111111111111111111111".to_string()),
            pool_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            pool_type: None,
            creator_address: None,
            subject_address: None,
            headline: None,
            value_1: None,
            value_2: None,
            flag: Some("true".to_string()),
            payload: Value::Null,
            mempool_entry_evidence: None,
        };

        let event = signal
            .to_risk_event()
            .expect("risk event")
            .expect("non-empty event");

        assert_eq!(event.kind, RiskKind::MempoolLiquidityRemoval);
        assert_eq!(event.severity, RiskSeverity::Critical);
        assert_eq!(event.source.as_deref(), Some(RISK_SOURCE_MEMPOOL_SIGNAL));
        assert!(event.pending_tx_hash.is_some());
    }

    #[test]
    fn trading_enabled_signal_carries_mempool_entry_evidence() {
        let entry_evidence = serde_json::json!({
            "evidence_version": "mempool_entry_evidence_v1",
            "base_block": 12
        });
        let signal = MempoolSignalWire {
            signal_id: "1".to_string(),
            signal_type: "trading_enabled".to_string(),
            signal_source: None,
            signal_created_at: None,
            mempool_first_seen_at: None,
            mempool_first_seen_ms: None,
            detection_timestamp: None,
            detection_tx_hash: None,
            token_address: Some("0x1111111111111111111111111111111111111111".to_string()),
            pool_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            pool_type: None,
            creator_address: None,
            subject_address: None,
            headline: None,
            value_1: None,
            value_2: None,
            flag: None,
            payload: Value::Null,
            mempool_entry_evidence: Some(entry_evidence.clone()),
        };

        let event = signal
            .to_risk_event()
            .expect("risk event")
            .expect("non-empty event");

        assert_eq!(event.kind, RiskKind::TradingEnabled);
        assert_eq!(
            event.evidence.unwrap()[MEMPOOL_ENTRY_EVIDENCE_KEY],
            entry_evidence
        );
    }

    #[test]
    fn lp_position_approval_message_includes_approved_pct_from_payload() {
        let signal = MempoolSignalWire {
            signal_id: "1".to_string(),
            signal_type: "lp_position_approval".to_string(),
            signal_source: None,
            signal_created_at: None,
            mempool_first_seen_at: None,
            mempool_first_seen_ms: None,
            detection_timestamp: Some("2026-05-19 08:59:54+00".to_string()),
            detection_tx_hash: None,
            token_address: Some("0x1111111111111111111111111111111111111111".to_string()),
            pool_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            pool_type: None,
            creator_address: None,
            subject_address: None,
            headline: Some("LP position approval".to_string()),
            value_1: None,
            value_2: None,
            flag: Some("true".to_string()),
            payload: serde_json::json!({ "approved_share_pct": 100.0 }),
            mempool_entry_evidence: None,
        };

        let event = signal
            .to_risk_event()
            .expect("risk event")
            .expect("non-empty event");

        assert!(event.message.contains("approved_pct=100.0%"));
    }
}
