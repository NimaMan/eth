/// Unified Signal Writer
///
/// A single writer that orchestrates all signal types and routes them to the appropriate
/// database writers. This keeps the signal publisher clean and makes it easy to add new
/// signal types without modifying multiple places.
use alloy_primitives::U256;
use eyre::Result;
use std::time::Duration;
use tracing::{debug, error, info};

use super::{
    LiquidityRemovalSignalRecord, LiquidityRemovalSignalWriter, LpApprovalSignalRecord,
    LpApprovalSignalWriter, SignalWriterConfig, TaxSignalRecord, TaxSignalWriter,
    TradingSignalRecord, TradingSignalWriter,
};
use crate::signal_detector::types::TaxSignalRecord as TaxSignal;
use crate::signal_detector::{
    LiquidityRemovalSignal, LpApprovalSignal, Signal, TradingEnabledSignal,
};

/// Unified writer that handles all signal types
pub struct UnifiedSignalWriter {
    trading_writer: Option<TradingSignalWriter>,
    tax_writer: Option<TaxSignalWriter>,
    liquidity_removal_writer: Option<LiquidityRemovalSignalWriter>,
    lp_approval_writer: Option<LpApprovalSignalWriter>,
}

impl UnifiedSignalWriter {
    /// Create a new unified signal writer with all sub-writers
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Initializing unified signal writer...");

        // Initialize trading signal writer
        let trading_writer =
            match TradingSignalWriter::new_with_defaults(SignalWriterConfig::default()).await {
                Ok(w) => {
                    info!("✅ Trading signal writer initialized");
                    Some(w)
                }
                Err(e) => {
                    error!("Failed to create trading signal writer: {}", e);
                    None
                }
            };

        // Initialize tax signal writer
        let tax_writer = match TaxSignalWriter::new(database_url, 50, Duration::from_secs(5)).await
        {
            Ok(w) => {
                info!("✅ Tax signal writer initialized");
                Some(w)
            }
            Err(e) => {
                error!("Failed to create tax signal writer: {}", e);
                None
            }
        };

        // Initialize liquidity removal signal writer
        let liquidity_removal_writer = match LiquidityRemovalSignalWriter::new().await {
            Ok(w) => {
                info!("✅ Liquidity removal signal writer initialized");
                Some(w)
            }
            Err(e) => {
                error!("Failed to create liquidity removal signal writer: {}", e);
                None
            }
        };

        // Initialize LP approval signal writer
        let lp_approval_writer = match LpApprovalSignalWriter::new().await {
            Ok(w) => {
                info!("✅ LP approval signal writer initialized");
                Some(w)
            }
            Err(e) => {
                error!("Failed to create LP approval signal writer: {}", e);
                None
            }
        };

        // Check if at least one writer was initialized
        if trading_writer.is_none()
            && tax_writer.is_none()
            && liquidity_removal_writer.is_none()
            && lp_approval_writer.is_none()
        {
            return Err(eyre::eyre!("Failed to initialize any database writers"));
        }

        info!("✅ Unified signal writer ready");

        Ok(Self {
            trading_writer,
            tax_writer,
            liquidity_removal_writer,
            lp_approval_writer,
        })
    }

    /// Write any signal type to the appropriate database
    pub async fn write_signal(&self, signal: Signal) -> Result<()> {
        match signal {
            Signal::TradingEnabled(s) => {
                if let Some(ref writer) = self.trading_writer {
                    let record = TradingSignalRecord::from_trading_enabled_signal(&s);
                    writer.write_signal(record).await;
                    debug!("Written TradingEnabled signal to database");
                } else {
                    debug!("Trading signal writer not available");
                }
            }

            Signal::TaxSignal(s) => {
                if let Some(ref writer) = self.tax_writer {
                    let record = TaxSignalRecord::from_tax_signal(&s);
                    writer.write_signal(record)?;
                    debug!("Written TaxSignal to database");
                } else {
                    debug!("Tax signal writer not available");
                }
            }

            Signal::LiquidityRemoval(s) => {
                if let Some(ref writer) = self.liquidity_removal_writer {
                    // Convert to LiquiditySignal for the writer
                    let liquidity_signal = crate::signal_detector::LiquiditySignal {
                        signal_type: crate::signal_detector::liquidity_detector::SignalType::LiquidityRemoval,
                        pool_address: s.pool_address.clone(),
                        token_address: s.token_address.clone().unwrap_or_default(),
                        pool_type: s.pool_type.clone(),
                        change_type: crate::signal_detector::liquidity_detector::LiquidityChangeType::MajorRemoval,
                        eth_change: s.estimated_eth_removed.unwrap_or(0.0),
                        percentage_change: 0.0, // We don't have this from LiquidityRemovalSignal
                        remaining_liquidity: 0.0, // We don't have this from LiquidityRemovalSignal
                        from_address: s.remover_address.clone(),
                        tx_hash: s.tx_hash.clone(),
                        details: format!("Function: {}", s.function_name),
                        eth_removed: s.estimated_eth_removed,
                        token_removed: None,
                        remaining_eth: None,
                        remaining_token: None,
                        removal_percentage: None,
                        creator_address: s.remover_address.clone(),
                    };
                    writer.write_signal(&liquidity_signal)?;
                    debug!("Written LiquidityRemoval signal to database");
                } else {
                    debug!("Liquidity removal signal writer not available");
                }
            }

            Signal::LpApproval(s) => {
                if let Some(ref writer) = self.lp_approval_writer {
                    // Write the signal directly since writer expects &LpApprovalSignal
                    writer.write_signal(&s)?;
                    debug!("Written LpApproval signal to database");
                } else {
                    debug!("LP approval signal writer not available");
                }
            }

            // ScamDetection signals are no longer written to DB
            Signal::ScamDetection(_s) => {
                debug!("Skipping ScamDetection signal (DB disabled)");
            }
        }

        Ok(())
    }

    /// Check if any writers are available
    pub fn has_writers(&self) -> bool {
        self.trading_writer.is_some()
            || self.tax_writer.is_some()
            || self.liquidity_removal_writer.is_some()
            || self.lp_approval_writer.is_some()
    }

    /// Get status of individual writers
    pub fn get_status(&self) -> String {
        format!(
            "Writers status - Trading: {}, Tax: {}, LiquidityRemoval: {}, LpApproval: {}",
            if self.trading_writer.is_some() {
                "✓"
            } else {
                "✗"
            },
            if self.tax_writer.is_some() {
                "✓"
            } else {
                "✗"
            },
            if self.liquidity_removal_writer.is_some() {
                "✓"
            } else {
                "✗"
            },
            if self.lp_approval_writer.is_some() {
                "✓"
            } else {
                "✗"
            },
        )
    }
}

// Conversion implementations for each signal type to its record type

impl TradingSignalRecord {
    /// Convert from TradingEnabledSignal
    pub fn from_trading_enabled_signal(signal: &TradingEnabledSignal) -> Self {
        Self {
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            pool_type: signal.pool_type.clone(),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: chrono::Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            price_ratio: None,
            denom_reserve_at_signal: None,
            token_reserve_at_signal: None,
            buy_tax_at_signal: Some(signal.buy_tax),
            sell_tax_at_signal: Some(signal.sell_tax),
            total_supply: None,
            owner_address: None,
            creator_address: signal.creator_address.clone(),
            signal_source: "mempool".to_string(),
        }
    }
}

impl TaxSignalRecord {
    /// Convert from TaxSignal
    pub fn from_tax_signal(signal: &TaxSignal) -> Self {
        Self {
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            pool_type: signal.pool_type.clone(),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: chrono::Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            signal_type: signal.signal_type.clone(),
            signal_details: signal.signal_details.clone(),
            confidence: Some(signal.confidence),
            buy_tax_at_signal: signal.buy_tax,
            sell_tax_at_signal: signal.sell_tax,
            buy_tax_exceeds_threshold: signal.buy_tax_exceeds_threshold,
            sell_tax_exceeds_threshold: signal.sell_tax_exceeds_threshold,
            cant_sell: signal.cant_sell,
            creator_address: signal.creator_address.clone(),
            signal_source: "mempool".to_string(),
        }
    }
}

impl LiquidityRemovalSignalRecord {
    /// Convert from LiquidityRemovalSignal  
    pub fn from_liquidity_removal_signal(signal: &LiquidityRemovalSignal) -> Self {
        Self {
            token_address: signal.token_address.clone().unwrap_or_default(),
            pool_address: signal.pool_address.clone(),
            pool_type: signal.pool_type.clone(),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: chrono::Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            liquidity_removed_denom: signal.estimated_eth_removed,
            remaining_liquidity_denom: None,
            pool_drain_risk_level: "HIGH".to_string(),
            creator_address: signal.remover_address.clone(),
            signal_source: "mempool".to_string(),
        }
    }
}

impl LpApprovalSignalRecord {
    /// Convert from LpApprovalSignal
    pub fn from_lp_approval_signal(signal: &LpApprovalSignal) -> Self {
        let is_unlimited_amount = signal.amount == U256::MAX;
        let approval_pct = signal.approval_percentage.or_else(|| {
            if is_unlimited_amount {
                Some(100.0)
            } else {
                None
            }
        });
        let unlimited = approval_pct
            .map(|pct| pct >= 99.99)
            .unwrap_or(is_unlimited_amount);

        Self {
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            pool_type: "V2".to_string(),
            denom_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            denom_currency: Some("WETH".to_string()),
            detection_timestamp: chrono::Utc::now(),
            detection_tx_hash: signal.tx_hash.clone(),
            approved_spender: signal.router_address.clone(),
            approval_percentage: approval_pct,
            is_unlimited_approval: unlimited,
            approval_type: "LP_TOKEN".to_string(),
            previous_allowance: signal.previous_allowance,
            creator_address: signal.creator.clone(),
            signal_source: "mempool".to_string(),
        }
    }
}
