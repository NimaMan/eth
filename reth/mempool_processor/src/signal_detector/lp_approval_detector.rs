use crate::function_detector::CreatorFunctionType;
/// LP Token Approval Detector
///
/// Detects when token creators approve routers to spend their LP tokens,
/// which is typically the precursor to a rug pull (liquidity removal)
use crate::mempool_fetcher::MempoolTransaction;
use crate::tx_router::TransactionCategory;
use alloy_primitives::{Address, U256};
use chrono::Utc;
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
    pub creator: String,
    pub lp_token_address: String,
    pub router_address: String,
    #[serde(with = "u256_serde")]
    pub amount: U256,
    pub timestamp: i64,
    // Additional fields for database
    pub token_address: String,
    pub pool_address: String,
    pub spender_address: String,
    pub amount_approved: Option<f64>,
    pub previous_allowance: Option<f64>,
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

        // Known routers (extend as needed)
        const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
        const SUSHISWAP_ROUTER: &str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
        let router_hex = format!("0x{}", hex::encode(router_address));
        let is_known_router = router_hex.eq_ignore_ascii_case(UNISWAP_V2_ROUTER)
            || router_hex.eq_ignore_ascii_case(SUSHISWAP_ROUTER);

        if !is_known_router {
            return None;
        }

        // Derive creator and LP token address directly from transaction fields
        let creator = format!("0x{}", hex::encode(&tx.from));
        let lp_token_address = tx
            .to
            .as_ref()
            .map(|t| format!("0x{}", hex::encode(t)))
            .unwrap_or_else(|| "unknown".to_string());

        let signal = LpApprovalSignal {
            tx_hash: tx.hash.clone(),
            creator: creator.clone(),
            lp_token_address: lp_token_address.clone(),
            router_address: router_hex.clone(),
            amount,
            timestamp: Utc::now().timestamp(),
            // Additional fields for database
            token_address: lp_token_address.clone(), // Will be resolved to actual token via LP token
            pool_address: lp_token_address.clone(),  // LP token address IS the pool address
            spender_address: router_hex.clone(),
            amount_approved: Some(amount.to_string().parse::<f64>().unwrap_or(0.0)),
            previous_allowance: None,
            creator_address: creator.clone(),
        };

        Some(signal)
    }

    // No-op: per-detector file logging removed in favor of centralized logging
    fn log_approval_warning(&mut self, _signal: &LpApprovalSignal) {}

    /// Log database write for tracking
    pub fn log_db_write(&mut self, _signal: &LpApprovalSignal, _success: bool) {}
}
