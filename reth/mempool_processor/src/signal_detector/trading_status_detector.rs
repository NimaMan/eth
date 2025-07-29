/// Trading Status Detector
/// 
/// Detects changes in token trading status (enabled/disabled)

use crate::simulator::{BuySellResult, SimulationResult};
use crate::tx_router::CreatorFunctionType;
use tracing::info;

#[derive(Debug, Clone)]
pub struct TradingStatusSignal {
    pub token_address: String,
    pub status_change: TradingStatusChange,
    pub executor: String,
    pub can_trade_after: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub confidence: f64,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradingStatusChange {
    /// Trading has been enabled
    TradingEnabled,
    /// Trading has been disabled
    TradingDisabled,
    /// Trading was paused
    TradingPaused,
    /// Trading status unchanged
    NoChange,
}

pub struct TradingStatusDetector {
    /// Whether to check buy/sell after trading enable
    verify_trading_works: bool,
}

impl TradingStatusDetector {
    pub fn new() -> Self {
        Self {
            verify_trading_works: true,
        }
    }

    /// Detect trading status changes
    pub fn detect(&self, sim_result: &SimulationResult) -> Option<TradingStatusSignal> {
        // Check if this is a trading control function
        let (executor, function_type, target_token) = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { 
                creator, 
                function_type, 
                target_token,
                .. 
            } => {
                if !matches!(function_type, CreatorFunctionType::TradingControl) {
                    return None;
                }
                (creator, function_type, target_token.as_ref()?)
            }
            _ => return None,
        };

        // Determine status change from function name
        let status_change = match function_type {
            CreatorFunctionType::TradingControl => {
                // In real implementation, would parse actual function name
                TradingStatusChange::TradingEnabled
            }
            _ => TradingStatusChange::NoChange,
        };

        if status_change == TradingStatusChange::NoChange {
            return None;
        }

        // Check if trading actually works
        let (can_trade, buy_tax, sell_tax) = if self.verify_trading_works {
            if let Some(buy_sell) = &sim_result.buy_sell_result {
                (
                    buy_sell.can_buy && buy_sell.can_sell,
                    buy_sell.buy_tax,
                    buy_sell.sell_tax,
                )
            } else {
                (false, None, None)
            }
        } else {
            (true, None, None) // Assume it works if not verified
        };

        let details = match status_change {
            TradingStatusChange::TradingEnabled => {
                if can_trade {
                    if let (Some(buy), Some(sell)) = (buy_tax, sell_tax) {
                        format!("Trading enabled successfully. Buy tax: {:.1}%, Sell tax: {:.1}%", buy, sell)
                    } else {
                        "Trading enabled successfully".to_string()
                    }
                } else {
                    "Trading enabled but verification failed - may be honeypot".to_string()
                }
            }
            TradingStatusChange::TradingDisabled => {
                "Trading has been disabled".to_string()
            }
            TradingStatusChange::TradingPaused => {
                "Trading has been paused".to_string()
            }
            _ => "Trading status unchanged".to_string(),
        };

        info!("🚦 {} for token {}", details, target_token);

        Some(TradingStatusSignal {
            token_address: target_token.clone(),
            status_change,
            executor: executor.clone(),
            can_trade_after: can_trade,
            buy_tax,
            sell_tax,
            confidence: if can_trade { 0.95 } else { 0.7 },
            details,
        })
    }
}