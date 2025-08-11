/// LP Token Approval Detector
/// 
/// Detects when token creators approve routers to spend their LP tokens,
/// which is typically the precursor to a rug pull (liquidity removal)

use crate::mempool_fetcher::MempoolTransaction;
use crate::tx_router::{TransactionCategory, CreatorFunctionType};
use tracing::{info, warn};
use alloy_primitives::{Address, U256};
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

/// Signal for LP token approval (rug pull setup)
#[derive(Debug, Clone)]
pub struct LpApprovalSignal {
    pub tx_hash: String,
    pub creator: String,
    pub lp_token_address: String,
    pub router_address: String,
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
    log_file: Option<std::fs::File>,
}

impl LpApprovalDetector {
    pub fn new() -> Self {
        // Create log file for LP approval warnings
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/lp_approval_warnings.log")
            .ok();
            
        Self { log_file }
    }
    
    /// Detect LP token approval from transaction data (no simulation needed)
    pub fn detect_from_transaction(
        &mut self,
        tx: &MempoolTransaction,
        category: &TransactionCategory,
    ) -> Option<LpApprovalSignal> {
        // Only process creator transactions with liquidity management
        if let TransactionCategory::CreatorTransaction { 
            creator, 
            target_address, 
            function_type: CreatorFunctionType::LiquidityManagement,
            .. 
        } = category {
            // Verify this is an approve function
            if tx.input.len() >= 68 && &tx.input[0..4] == &[0x09, 0x5e, 0xa7, 0xb3] {
                // Extract spender address (router) from bytes 16-36
                let mut spender_bytes = [0u8; 20];
                spender_bytes.copy_from_slice(&tx.input[16..36]);
                let router_address = Address::from(spender_bytes);
                
                // Extract amount from bytes 36-68
                let amount = U256::from_be_slice(&tx.input[36..68]);
                
                // Known routers
                const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
                const SUSHISWAP_ROUTER: &str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
                
                let router_hex = format!("0x{}", hex::encode(router_address));
                let is_known_router = router_hex.eq_ignore_ascii_case(UNISWAP_V2_ROUTER) ||
                                     router_hex.eq_ignore_ascii_case(SUSHISWAP_ROUTER);
                
                if is_known_router {
                    let signal = LpApprovalSignal {
                        tx_hash: tx.hash.clone(),
                        creator: creator.clone(),
                        lp_token_address: target_address.clone(),
                        router_address: router_hex.clone(),
                        amount,
                        timestamp: Utc::now().timestamp(),
                        // Additional fields for database
                        token_address: target_address.clone(), // LP token for now
                        pool_address: target_address.clone(), // LP token acts as pool identifier
                        spender_address: router_hex.clone(),
                        amount_approved: Some(amount.to_string().parse::<f64>().unwrap_or(0.0)),
                        previous_allowance: None, // TODO: Get from state changes
                        creator_address: creator.clone(),
                    };
                    
                    // Log the warning
                    self.log_approval_warning(&signal);
                    
                    warn!("🚨 RUG PULL SETUP DETECTED!");
                    warn!("  Creator: {}", creator);
                    warn!("  LP Token: {}", target_address);
                    warn!("  Router: {}", router_hex);
                    warn!("  Amount: {}", amount);
                    warn!("  TX: {}", tx.hash);
                    
                    return Some(signal);
                }
            }
        }
        
        None
    }
    
    fn log_approval_warning(&mut self, signal: &LpApprovalSignal) {
        if let Some(ref mut file) = self.log_file {
            let log_entry = format!(
                "[{}] WARN: Creator {} approved router {} to spend {} LP tokens of pool {} (tx: {})\n",
                Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                signal.creator,
                signal.router_address,
                signal.amount,
                signal.lp_token_address,
                signal.tx_hash
            );
            
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }
}