/// LP Token Approval Detector
///
/// Detects when LP token holders approve routers to spend their LP tokens,
/// which is typically the precursor to a rug pull (liquidity removal)
use crate::mempool_fetcher::MempoolTransaction;
use crate::tx_router::TransactionCategory;
use alloy_primitives::{Address, U256};
use chrono::Utc;
use reth_chain_query::to_checksum_address;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Custom serialization for U256
mod u256_serde {
    use alloy_primitives::U256;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<U256>().map_err(serde::de::Error::custom)
    }
}

/// Signal for LP token approval (rug pull setup)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpApprovalSignal {
    pub tx_hash: String,
    /// Backward-compatible alias for the LP token approver.
    pub creator: String,
    pub lp_token_address: String,
    pub router_address: String,
    #[serde(with = "u256_serde")]
    pub amount: U256,
    pub timestamp: i64,
    // Additional fields for database
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub denom_address: Option<String>,
    pub denom_currency: Option<String>,
    pub denom_decimals: Option<u8>,
    pub spender_address: String,
    pub approval_percentage: Option<f64>,
    pub approved_share_pct: Option<f64>,
    pub approval_model: Option<String>,
    pub lp_total_supply: Option<String>,
    pub position_manager: Option<String>,
    pub position_id: Option<String>,
    pub position_liquidity: Option<String>,
    pub pool_liquidity: Option<String>,
    pub position_share_pct: Option<f64>,
    pub previous_allowance: Option<f64>,
    pub approver_address: String,
    pub creator_address: String,
}

pub struct LpApprovalDetector {
    // Previously held a log file for per-detector logs. We now delegate
    // logging to SignalPublisher to avoid duplicate entries.
    #[allow(dead_code)]
    log_file: Option<std::fs::File>,
}

impl LpApprovalDetector {
    pub fn new(log_dir: &Path) -> Self {
        // Detector no longer writes directly to files; keep field for backward
        // compatibility but do not open or use a file here.
        let _ = log_dir; // unused
        let log_file = None;

        Self { log_file }
    }

    /// Detect LP token approval from transaction data (no simulation needed)
    pub fn detect_from_transaction(
        &mut self,
        tx: &MempoolTransaction,
        _category: &TransactionCategory,
    ) -> Option<LpApprovalSignal> {
        // Verify this is an approve() call
        if tx.input.len() < 68 || &tx.input[0..4] != &[0x09, 0x5e, 0xa7, 0xb3] {
            return None;
        }

        // Extract spender (router) from calldata
        let mut spender_bytes = [0u8; 20];
        spender_bytes.copy_from_slice(&tx.input[16..36]);
        let router_address = Address::from(spender_bytes);

        // Extract amount from calldata
        let amount = U256::from_be_slice(&tx.input[36..68]);

        let router_hex = to_checksum_address(&router_address);

        // Derive approver and LP token address directly from transaction fields.
        let approver = to_checksum_address(&Address::from_slice(&tx.from));
        let lp_token_address = tx
            .to
            .as_ref()
            .map(|t| to_checksum_address(&Address::from_slice(t)))
            .unwrap_or_else(|| "unknown".to_string());

        let signal = LpApprovalSignal {
            tx_hash: tx.hash.clone(),
            creator: approver.clone(),
            lp_token_address: lp_token_address.clone(),
            router_address: router_hex.clone(),
            amount,
            timestamp: Utc::now().timestamp(),
            // Additional fields for database
            token_address: lp_token_address.clone(), // Resolved to actual token by SignalManager
            pool_address: lp_token_address.clone(),  // LP token address is the V2 pool address
            pool_type: "UNKNOWN".to_string(),
            denom_address: None,
            denom_currency: None,
            denom_decimals: None,
            spender_address: router_hex.clone(),
            approval_percentage: None,
            approved_share_pct: None,
            approval_model: None,
            lp_total_supply: None,
            position_manager: None,
            position_id: None,
            position_liquidity: None,
            pool_liquidity: None,
            position_share_pct: None,
            previous_allowance: None,
            approver_address: approver.clone(),
            creator_address: approver.clone(),
        };

        Some(signal)
    }
    /// Log database write for tracking
    pub fn log_db_write(&mut self, _signal: &LpApprovalSignal, _success: bool) {}
}
