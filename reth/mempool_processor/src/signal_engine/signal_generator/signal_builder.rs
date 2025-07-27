/// Signal Builder
/// 
/// Builds unified signals from detector outputs

use crate::signal_engine::types::{EventType, Severity};
use crate::signal_engine::detectors::{
    HoneypotSignal, LiquiditySignal, TaxChangeSignal, TradingStatusSignal,
    LiquidityChangeType,
};
use crate::signal_engine::detectors::tax_change_detector::TaxRiskLevel;
use crate::signal_engine::detectors::trading_status_detector::TradingStatusChange;
use ethers::types::H256;
use serde::{Serialize, Deserialize};

/// Unified signal structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSignal {
    pub signal_id: String,
    pub signal_type: SignalType,
    pub severity: Severity,
    pub confidence: f64,
    pub risk_score: u8, // 0-100
    pub tx_hash: H256,
    pub timestamp: u64,
    pub data: SignalData,
    pub metadata: SignalMetadata,
}

/// Types of signals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SignalType {
    Honeypot,
    LiquidityChange,
    TaxChange,
    TradingStatus,
    CreatorAction,
    Scam,
}

/// Signal-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalData {
    Honeypot(HoneypotData),
    Liquidity(LiquidityData),
    TaxChange(TaxChangeData),
    TradingStatus(TradingStatusData),
    CreatorAction(CreatorActionData),
}

/// Honeypot signal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoneypotData {
    pub token_address: String,
    pub detection_method: String,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub can_sell: bool,
}

/// Liquidity signal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityData {
    pub pool_address: String,
    pub token_address: String,
    pub change_type: String,
    pub eth_change: f64,
    pub percentage_change: f64,
    pub remaining_liquidity: f64,
}

/// Tax change signal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxChangeData {
    pub token_address: String,
    pub buy_tax_before: Option<f64>,
    pub buy_tax_after: Option<f64>,
    pub sell_tax_before: Option<f64>,
    pub sell_tax_after: Option<f64>,
    pub changer_address: String,
    pub is_honeypot_after: bool,
}

/// Trading status signal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStatusData {
    pub token_address: String,
    pub status_change: String,
    pub executor: String,
    pub can_trade_after: bool,
    pub current_taxes: Option<(f64, f64)>, // (buy, sell)
}

/// Creator action signal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorActionData {
    pub creator_address: String,
    pub target_address: String,
    pub action_type: String,
    pub function_called: String,
}

/// Signal metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMetadata {
    pub source: String,
    pub chain_id: u64,
    pub block_number: Option<u64>,
    pub gas_price: Option<u64>,
    pub details: String,
}

/// Signal builder
pub struct SignalBuilder {
    chain_id: u64,
    source: String,
}

impl SignalBuilder {
    pub fn new(chain_id: u64) -> Self {
        Self {
            chain_id,
            source: "mempool_processor".to_string(),
        }
    }

    /// Build signal from honeypot detection
    pub fn from_honeypot(
        &self,
        signal: HoneypotSignal,
        tx_hash: H256,
        risk_score: u8,
    ) -> UnifiedSignal {
        UnifiedSignal {
            signal_id: Self::generate_id(&tx_hash, "honeypot"),
            signal_type: SignalType::Honeypot,
            severity: Severity::Critical,
            confidence: signal.confidence,
            risk_score,
            tx_hash,
            timestamp: chrono::Utc::now().timestamp() as u64,
            data: SignalData::Honeypot(HoneypotData {
                token_address: signal.token_address,
                detection_method: format!("{:?}", signal.detection_method),
                buy_tax: signal.buy_tax,
                sell_tax: signal.sell_tax,
                can_sell: signal.can_sell,
            }),
            metadata: SignalMetadata {
                source: self.source.clone(),
                chain_id: self.chain_id,
                block_number: None,
                gas_price: None,
                details: signal.details,
            },
        }
    }

    /// Build signal from liquidity change
    pub fn from_liquidity(
        &self,
        signal: LiquiditySignal,
        tx_hash: H256,
        risk_score: u8,
    ) -> UnifiedSignal {
        let severity = match signal.change_type {
            LiquidityChangeType::CompleteDrain => Severity::Critical,
            LiquidityChangeType::MajorRemoval => Severity::High,
            LiquidityChangeType::SignificantRemoval => Severity::Medium,
            _ => Severity::Low,
        };

        UnifiedSignal {
            signal_id: Self::generate_id(&tx_hash, "liquidity"),
            signal_type: SignalType::LiquidityChange,
            severity,
            confidence: signal.confidence,
            risk_score,
            tx_hash,
            timestamp: chrono::Utc::now().timestamp() as u64,
            data: SignalData::Liquidity(LiquidityData {
                pool_address: signal.pool_address,
                token_address: signal.token_address,
                change_type: format!("{:?}", signal.change_type),
                eth_change: signal.eth_change,
                percentage_change: signal.percentage_change,
                remaining_liquidity: signal.remaining_liquidity,
            }),
            metadata: SignalMetadata {
                source: self.source.clone(),
                chain_id: self.chain_id,
                block_number: None,
                gas_price: None,
                details: signal.details,
            },
        }
    }

    /// Build signal from tax change
    pub fn from_tax_change(
        &self,
        signal: TaxChangeSignal,
        tx_hash: H256,
        risk_score: u8,
    ) -> UnifiedSignal {
        let severity = match signal.risk_level {
            TaxRiskLevel::Critical => Severity::Critical,
            TaxRiskLevel::High => Severity::High,
            TaxRiskLevel::Medium => Severity::Medium,
            TaxRiskLevel::Low => Severity::Low,
        };

        UnifiedSignal {
            signal_id: Self::generate_id(&tx_hash, "tax"),
            signal_type: SignalType::TaxChange,
            severity,
            confidence: signal.confidence,
            risk_score,
            tx_hash,
            timestamp: chrono::Utc::now().timestamp() as u64,
            data: SignalData::TaxChange(TaxChangeData {
                token_address: signal.token_address,
                buy_tax_before: signal.before.buy_tax,
                buy_tax_after: signal.after.buy_tax,
                sell_tax_before: signal.before.sell_tax,
                sell_tax_after: signal.after.sell_tax,
                changer_address: signal.changer_address,
                is_honeypot_after: signal.is_honeypot_after,
            }),
            metadata: SignalMetadata {
                source: self.source.clone(),
                chain_id: self.chain_id,
                block_number: None,
                gas_price: None,
                details: signal.details,
            },
        }
    }

    /// Build signal from trading status change
    pub fn from_trading_status(
        &self,
        signal: TradingStatusSignal,
        tx_hash: H256,
        risk_score: u8,
    ) -> UnifiedSignal {
        let severity = match signal.status_change {
            TradingStatusChange::TradingDisabled => Severity::High,
            TradingStatusChange::TradingPaused => Severity::Medium,
            _ => Severity::Low,
        };

        let current_taxes = match (signal.buy_tax, signal.sell_tax) {
            (Some(buy), Some(sell)) => Some((buy, sell)),
            _ => None,
        };

        UnifiedSignal {
            signal_id: Self::generate_id(&tx_hash, "trading"),
            signal_type: SignalType::TradingStatus,
            severity,
            confidence: signal.confidence,
            risk_score,
            tx_hash,
            timestamp: chrono::Utc::now().timestamp() as u64,
            data: SignalData::TradingStatus(TradingStatusData {
                token_address: signal.token_address,
                status_change: format!("{:?}", signal.status_change),
                executor: signal.executor,
                can_trade_after: signal.can_trade_after,
                current_taxes,
            }),
            metadata: SignalMetadata {
                source: self.source.clone(),
                chain_id: self.chain_id,
                block_number: None,
                gas_price: None,
                details: signal.details,
            },
        }
    }

    /// Generate unique signal ID
    fn generate_id(tx_hash: &H256, signal_type: &str) -> String {
        let timestamp = chrono::Utc::now().timestamp_nanos();
        format!("{}-{}-{}", tx_hash, signal_type, timestamp)
    }
}