//! Heuristics for optimizing transaction data retrieval
//! 
//! This module contains logic to intelligently determine when simulation is needed
//! vs when database-only access is sufficient.

use super::types::{TransactionType, TransactionDataOptions};
use revm_primitives::{Address as RevmAddress, U256 as RevmU256};
// Removed unused import

/// Well-known contract addresses that commonly generate internal transfers
const DEX_CONTRACTS: &[&str] = &[
    "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", // Uniswap V2 Router
    "0xE592427A0AEce92De3Edee1F18E0157C05861564", // Uniswap V3 Router
    "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45", // Uniswap V3 Router 2
    "0xEf1c6E67703c7BD7107eed8303Fbe6EC2554BF6B", // Uniswap Universal Router
    "0x881D40237659C251811CEC9c364ef91dC08D300C", // Metamask Swap Router
    "0x1111111254EEB25477B68fb85Ed929f73A960582", // 1inch Router v5
    "0x1111111254fb6c44bAC0beD2854e76F90643097d", // 1inch Router v4
    "0xDef1C0ded9bec7F1a1670819833240f027b25EfF", // 0x Exchange
];

const BRIDGE_CONTRACTS: &[&str] = &[
    "0xA0b86991c6218b36c1d19D4a2e9Eb0Ce3606eB48", // Circle Bridge
    "0x3154Cf16ccdb4C6d922629664174b904d80F2C35", // Polygon Bridge
    "0x4Dbd4fc535Ac27206064B68FfCf827b0A60BAB3f", // Arbitrum Bridge
];

const MULTI_SIG_CONTRACTS: &[&str] = &[
    "0x34cfae9a5947fa3d7c7df3a9526b0adbb6b30945", // Popular Gnosis Safe
    "0x0DA0C3e52C977Ed3cBc641fF02DD271c3ED55aFe", // Another common multi-sig
];

/// Detect the type of transaction for optimization heuristics
pub fn detect_transaction_type(
    to_address: Option<RevmAddress>,
    value: RevmU256,
    input_data: &[u8],
    gas_limit: u64,
) -> TransactionType {
    // Contract deployment
    if to_address.is_none() {
        return TransactionType::ContractDeployment;
    }
    
    let to = to_address.unwrap();
    let to_hex = format!("{:x}", to).to_lowercase();
    
    // Check if it's a known DEX
    if DEX_CONTRACTS.iter().any(|&addr| addr.to_lowercase().contains(&to_hex)) {
        return TransactionType::DexInteraction;
    }
    
    // Check if it's a known bridge or complex DeFi
    if BRIDGE_CONTRACTS.iter().any(|&addr| addr.to_lowercase().contains(&to_hex)) ||
       MULTI_SIG_CONTRACTS.iter().any(|&addr| addr.to_lowercase().contains(&to_hex)) {
        return TransactionType::ComplexDeFi;
    }
    
    // Simple transfer heuristics
    if input_data.is_empty() && value > RevmU256::ZERO {
        // Could be EOA transfer or simple contract call
        // For now, assume it's simple if gas limit is low
        if gas_limit < 50_000 {
            return TransactionType::SimpleTransfer;
        }
    }
    
    // Check for complex input data patterns
    if input_data.len() > 4 {
        // Check for common function selectors that indicate complex operations
        let function_selector = &input_data[0..4];
        
        // Common DEX function selectors
        let dex_selectors = [
            [0xa9, 0x05, 0x9c, 0xbb], // swapExactTokensForTokens
            [0x38, 0xed, 0x17, 0x39], // swapExactETHForTokens
            [0x18, 0xcb, 0xaf, 0xe5], // swapExactTokensForETH
            [0x8a, 0x65, 0x7b, 0x34], // swapTokensForExactETH
            [0x42, 0x71, 0x2a, 0x67], // swapExactTokensForTokensSupportingFeeOnTransferTokens
        ];
        
        if dex_selectors.iter().any(|&sel| sel == function_selector) {
            return TransactionType::DexInteraction;
        }
        
        // Multi-call or batch operation selectors
        let complex_selectors = [
            [0xac, 0x96, 0x50, 0xd8], // multicall
            [0x59, 0x45, 0x42, 0x04], // batchSwap
            [0x25, 0x2d, 0xcd, 0x11], // aggregate
        ];
        
        if complex_selectors.iter().any(|&sel| sel == function_selector) {
            return TransactionType::ComplexDeFi;
        }
    }
    
    // Default to contract call if it has input data or high gas limit
    if !input_data.is_empty() || gas_limit > 100_000 {
        TransactionType::ContractCall
    } else {
        TransactionType::SimpleTransfer
    }
}

/// Determine if internal transfers are likely needed based on transaction characteristics
pub fn needs_internal_transfers(
    transaction_type: &TransactionType,
    options: &TransactionDataOptions,
) -> bool {
    // Always simulate if explicitly requested
    if options.need_internal_transfers {
        return true;
    }
    
    // Always simulate if other simulation features are needed
    if options.need_call_trace || options.need_state_changes {
        return true;
    }
    
    // Override if force level is specified
    if let Some(level) = options.force_level {
        return match level {
            super::types::DataLevel::Basic => false,
            super::types::DataLevel::Smart => {
                // Smart mode: use heuristics
                match transaction_type {
                    TransactionType::SimpleTransfer => false,
                    TransactionType::ContractCall => false, // Conservative: most contract calls don't have internal transfers
                    TransactionType::DexInteraction => true,
                    TransactionType::ComplexDeFi => true,
                    TransactionType::ContractDeployment => false,
                }
            }
            super::types::DataLevel::Complete => true,
        };
    }
    
    // Default smart heuristics
    match transaction_type {
        TransactionType::SimpleTransfer => false,
        TransactionType::ContractCall => false, // Conservative approach
        TransactionType::DexInteraction => true, // DEX swaps often have internal transfers
        TransactionType::ComplexDeFi => true,    // Bridges, multi-sig likely have internal transfers
        TransactionType::ContractDeployment => false,
    }
}

/// Determine if simulation should be used based on all factors
pub fn should_use_simulation(
    transaction_type: &TransactionType,
    options: &TransactionDataOptions,
) -> bool {
    needs_internal_transfers(transaction_type, options)
}

/// Analyze transaction complexity for optimization hints
pub fn analyze_transaction_complexity(
    _to_address: Option<RevmAddress>,
    input_data: &[u8],
    gas_limit: u64,
    gas_used: u64,
) -> (bool, Vec<String>) {
    let mut complexity_indicators = Vec::new();
    let mut is_complex = false;
    
    // High gas usage indicates complexity
    if gas_used > 200_000 {
        complexity_indicators.push("High gas usage".to_string());
        is_complex = true;
    }
    
    // Large input data indicates complex operations
    if input_data.len() > 1000 {
        complexity_indicators.push("Large input data".to_string());
        is_complex = true;
    }
    
    // Multiple function calls (heuristic based on gas limit vs usage ratio)
    if gas_limit > 500_000 {
        complexity_indicators.push("High gas limit (likely complex operations)".to_string());
        is_complex = true;
    }
    
    // Check for specific patterns in input data
    if input_data.len() >= 4 {
        let function_selector = &input_data[0..4];
        
        // Multicall patterns
        if function_selector == [0xac, 0x96, 0x50, 0xd8] { // multicall
            complexity_indicators.push("Multicall detected".to_string());
            is_complex = true;
        }
        
        // Batch operations
        if function_selector == [0x25, 0x2d, 0xcd, 0x11] { // aggregate
            complexity_indicators.push("Batch operation detected".to_string());
            is_complex = true;
        }
    }
    
    (is_complex, complexity_indicators)
}

/// Get optimization recommendation for a transaction
pub fn get_optimization_recommendation(
    transaction_type: &TransactionType,
    gas_used: u64,
    log_count: usize,
) -> String {
    match transaction_type {
        TransactionType::SimpleTransfer => {
            "✅ Database-only: Simple transfer, no simulation needed".to_string()
        }
        TransactionType::ContractCall => {
            if gas_used < 100_000 && log_count < 5 {
                "✅ Database-only: Simple contract call, unlikely to have internal transfers".to_string()
            } else {
                "⚠️ Consider simulation: Complex contract call may have internal transfers".to_string()
            }
        }
        TransactionType::DexInteraction => {
            "🔄 Simulation recommended: DEX interactions commonly have internal transfers".to_string()
        }
        TransactionType::ComplexDeFi => {
            "🔄 Simulation required: Complex DeFi operations likely have internal transfers".to_string()
        }
        TransactionType::ContractDeployment => {
            "✅ Database-only: Contract deployment, no internal transfers expected".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_transfer_detection() {
        let tx_type = detect_transaction_type(
            Some("0x742d35cc6571c4c8d8e9c8e3a6d3b8a2b4c4d5e6".parse().unwrap()),
            RevmU256::from(1000000000000000000u64), // 1 ETH
            &[], // Empty input
            21000, // Standard gas limit
        );
        assert_eq!(tx_type, TransactionType::SimpleTransfer);
    }
    
    #[test]
    fn test_dex_interaction_detection() {
        let uniswap_router: RevmAddress = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap();
        let tx_type = detect_transaction_type(
            Some(uniswap_router),
            RevmU256::ZERO,
            &[0xa9, 0x05, 0x9c, 0xbb], // swapExactTokensForTokens selector
            200000,
        );
        assert_eq!(tx_type, TransactionType::DexInteraction);
    }
    
    #[test]
    fn test_internal_transfers_heuristics() {
        let options = TransactionDataOptions::smart();
        
        // Simple transfer shouldn't need simulation
        assert!(!needs_internal_transfers(&TransactionType::SimpleTransfer, &options));
        
        // DEX interaction should need simulation
        assert!(needs_internal_transfers(&TransactionType::DexInteraction, &options));
        
        // Force internal transfers should always need simulation
        let force_options = TransactionDataOptions::with_internal_transfers();
        assert!(needs_internal_transfers(&TransactionType::SimpleTransfer, &force_options));
    }
}