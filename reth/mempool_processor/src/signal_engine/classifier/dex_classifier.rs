/// DEX Classifier
/// 
/// Identifies DEX interactions and categorizes them

use crate::mempool_fetcher::MempoolTransaction;
use crate::common::address::checksum_address;
use super::{DexType, DexAction};

/// Known DEX router addresses
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const UNISWAP_V3_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
const UNISWAP_V3_ROUTER2: &str = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45";
const SUSHISWAP_ROUTER: &str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";

pub struct DexClassifier {
    router_addresses: std::collections::HashMap<String, DexType>,
}

impl DexClassifier {
    pub fn new() -> Self {
        let mut router_addresses = std::collections::HashMap::new();
        
        // Uniswap V2
        router_addresses.insert(checksum_address(UNISWAP_V2_ROUTER), DexType::UniswapV2);
        
        // Uniswap V3
        router_addresses.insert(checksum_address(UNISWAP_V3_ROUTER), DexType::UniswapV3);
        router_addresses.insert(checksum_address(UNISWAP_V3_ROUTER2), DexType::UniswapV3);
        
        // SushiSwap
        router_addresses.insert(checksum_address(SUSHISWAP_ROUTER), DexType::SushiSwap);
        
        Self { router_addresses }
    }

    /// Identify DEX action from transaction
    pub fn identify_dex_action(&self, tx: &MempoolTransaction) -> Option<(DexType, DexAction)> {
        let to_addr = checksum_address(&tx.to.trim_start_matches("0x"));
        
        // Check if it's a known DEX router
        let dex_type = self.router_addresses.get(&to_addr)?.clone();
        
        // Decode function selector
        let input_data = hex::decode(&tx.input.trim_start_matches("0x")).ok()?;
        if input_data.len() < 4 {
            return None;
        }
        
        let selector = &input_data[0..4];
        let action = match selector {
            // Add liquidity functions
            [0xe8, 0xe3, 0x37, 0x00] => DexAction::AddLiquidity,     // addLiquidity
            [0xf3, 0x05, 0xd7, 0x19] => DexAction::AddLiquidity,     // addLiquidityETH
            [0x85, 0xf8, 0xc2, 0x59] => DexAction::AddLiquidity,     // addLiquidityETHSupportingFeeOnTransferTokens
            
            // Remove liquidity functions
            [0xba, 0xa2, 0xab, 0xde] => DexAction::RemoveLiquidity,  // removeLiquidity
            [0x02, 0x75, 0x1c, 0xec] => DexAction::RemoveLiquidity,  // removeLiquidityETH
            [0xaf, 0x29, 0x79, 0xeb] => DexAction::RemoveLiquidity,  // removeLiquidityETHSupportingFeeOnTransferTokens
            [0x2e, 0x95, 0xb6, 0xc8] => DexAction::RemoveLiquidity,  // removeLiquidityWithPermit
            [0xde, 0xd9, 0x38, 0x2a] => DexAction::RemoveLiquidity,  // removeLiquidityETHWithPermit
            [0x5b, 0x0d, 0x59, 0x85] => DexAction::RemoveLiquidity,  // removeLiquidityETHWithPermitSupportingFeeOnTransferTokens
            
            // Swap functions
            [0x38, 0xed, 0x17, 0x39] => DexAction::Swap,             // swapExactTokensForTokens
            [0x8a, 0xc6, 0x97, 0xfa] => DexAction::Swap,             // swapTokensForExactTokens
            [0x7f, 0xf3, 0x6a, 0xb5] => DexAction::Swap,             // swapExactETHForTokens
            [0x4a, 0x25, 0xd9, 0x4a] => DexAction::Swap,             // swapTokensForExactETH
            [0x18, 0xcb, 0xaf, 0xe5] => DexAction::Swap,             // swapExactTokensForETH
            [0xfb, 0x3b, 0xdb, 0x41] => DexAction::Swap,             // swapETHForExactTokens
            [0xb6, 0xf9, 0xde, 0x95] => DexAction::Swap,             // swapExactETHForTokensSupportingFeeOnTransferTokens
            [0x79, 0x1a, 0xc9, 0x47] => DexAction::Swap,             // swapExactTokensForETHSupportingFeeOnTransferTokens
            [0x5c, 0x11, 0xd7, 0x95] => DexAction::Swap,             // swapExactTokensForTokensSupportingFeeOnTransferTokens
            
            // V3 specific
            [0x41, 0x4b, 0xf3, 0x89] => DexAction::Swap,             // exactInputSingle
            [0xc0, 0x4b, 0x8d, 0x59] => DexAction::Swap,             // exactInput
            [0xf2, 0x8c, 0x61, 0x57] => DexAction::Swap,             // exactOutputSingle
            [0x09, 0xb8, 0x13, 0x46] => DexAction::Swap,             // exactOutput
            
            _ => DexAction::Other,
        };
        
        Some((dex_type, action))
    }

    /// Extract token and pool addresses from calldata if possible
    pub fn extract_addresses(&self, _tx: &MempoolTransaction) -> (Option<String>, Option<String>) {
        // TODO: Implement calldata parsing to extract addresses
        // This requires understanding the ABI encoding for each function
        (None, None)
    }
}