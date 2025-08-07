/// Creator Transaction Classifier
/// 
/// Identifies and categorizes transactions from known token creators

use std::sync::Arc;
use crate::mempool_fetcher::MempoolTransaction;
use crate::token_tracking::TokenTrackingCache;
use crate::common::address::checksum_address;
use super::CreatorFunctionType;
use hex;

pub struct CreatorTransactionRouter {
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl CreatorTransactionRouter {
    pub fn new(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        Self { token_cache }
    }

    /// Identify the type of function being called
    pub fn identify_function(&self, tx: &MempoolTransaction) -> CreatorFunctionType {
        // Check if this is a simple ETH transfer (no input data or empty input)
        if tx.input.is_empty() || tx.input.len() < 4 {
            return CreatorFunctionType::Other("eth_transfer".to_string());
        }

        let selector = &tx.input[0..4];
        
        // Special handling for approve - check if it's LP token approval
        if selector == &[0x09, 0x5e, 0xa7, 0xb3] {
            return self.classify_approve(tx);
        }
        
        match selector {
            // Tax modification functions
            [0x03, 0x2d, 0xc6, 0xa2] => CreatorFunctionType::TaxModification, // setTaxes
            [0x65, 0x8d, 0x45, 0x81] => CreatorFunctionType::TaxModification, // setTax
            [0x2f, 0x2f, 0xf1, 0x5d] => CreatorFunctionType::TaxModification, // setBuyTax
            [0x6d, 0x4e, 0x21, 0xf5] => CreatorFunctionType::TaxModification, // setSellTax
            [0x9d, 0x00, 0x14, 0xb1] => CreatorFunctionType::TaxModification, // setBuyAndSellTax
            [0x08, 0x3c, 0x63, 0x23] => CreatorFunctionType::TaxModification, // setFees
            [0x66, 0xcf, 0xee, 0x39] => CreatorFunctionType::TaxModification, // updateFees
            
            // Trading control functions
            [0x8a, 0x8c, 0x52, 0x3c] => CreatorFunctionType::TradingControl, // setTradingEnabled
            [0x8e, 0xe8, 0x8c, 0x53] => CreatorFunctionType::TradingControl, // enableTrading
            [0xc9, 0x56, 0x7b, 0xf9] => CreatorFunctionType::TradingControl, // openTrading
            [0xfb, 0x20, 0x1b, 0x1d] => CreatorFunctionType::TradingControl, // startTrading
            [0x1c, 0x83, 0x87, 0xfa] => CreatorFunctionType::TradingControl, // pauseTrading
            [0x0f, 0xb5, 0xa6, 0xec] => CreatorFunctionType::TradingControl, // disableTrading
            
            // Ownership functions
            [0xf2, 0xfd, 0xe3, 0x8b] => CreatorFunctionType::OwnershipChange, // transferOwnership
            [0x71, 0x5a, 0x1e, 0x08] => CreatorFunctionType::OwnershipChange, // renounceOwnership
            
            // Liquidity management
            [0xe8, 0xe3, 0x37, 0x00] => CreatorFunctionType::LiquidityManagement, // addLiquidity
            [0xf3, 0x05, 0xd7, 0x19] => CreatorFunctionType::LiquidityManagement, // addLiquidityETH
            [0x02, 0x75, 0x1c, 0xec] => CreatorFunctionType::LiquidityManagement, // removeLiquidity
            [0xaf, 0x29, 0x79, 0xeb] => CreatorFunctionType::LiquidityManagement, // removeLiquidityETH
            
            // Max wallet/tx limits
            [0xc8, 0x60, 0xe1, 0x4a] => CreatorFunctionType::MaxWalletLimit, // setMaxWallet
            [0x4a, 0x74, 0xbb, 0x02] => CreatorFunctionType::MaxWalletLimit, // setMaxTx
            [0x36, 0x0c, 0x42, 0xf8] => CreatorFunctionType::MaxWalletLimit, // setLimits
            
            _ => {
                let selector_hex = format!("{:02x}{:02x}{:02x}{:02x}", 
                    selector[0], selector[1], selector[2], selector[3]);
                CreatorFunctionType::Other(selector_hex)
            }
        }
    }
    
    /// Classify approve() calls - determine if it's LP token approval for rug pull
    fn classify_approve(&self, tx: &MempoolTransaction) -> CreatorFunctionType {
        // Check if we have enough data for approve(address,uint256)
        if tx.input.len() < 68 {
            return CreatorFunctionType::Other("approve".to_string());
        }
        
        // Extract spender address from input data (bytes 4-36)
        let spender_bytes = &tx.input[16..36]; // Skip 12 bytes of padding
        let spender_hex = hex::encode(spender_bytes);
        
        // Known DEX routers that handle liquidity removal
        const UNISWAP_V2_ROUTER: &str = "7a250d5630b4cf539739df2c5dacb4c659f2488d";
        const SUSHISWAP_ROUTER: &str = "d9e1ce17f2641f24ae83637ab66a2cca9c378b9f";
        
        // Check if spender is a known router
        let is_router_approval = spender_hex.eq_ignore_ascii_case(UNISWAP_V2_ROUTER) ||
                                 spender_hex.eq_ignore_ascii_case(SUSHISWAP_ROUTER);
        
        // Only check pool if router is being approved
        if !is_router_approval {
            return CreatorFunctionType::Other("approve".to_string());
        }
        
        // Check if the approve is being called on an LP token contract
        if let Some(to_bytes) = &tx.to {
            let to_addr = hex::encode(to_bytes);
            
            if let Some(ref cache) = self.token_cache {
                let from_addr = checksum_address(&hex::encode(&tx.from));
                
                // Get the token created by this address
                if let Some(token_info) = futures::executor::block_on(cache.get_token_for_creator(&from_addr)) {
                    // Get all pools for this token
                    let pools = futures::executor::block_on(cache.get_pools_for_token(&token_info.token_address));
                    
                    // Check if the 'to' address is one of the pool addresses
                    for (pool_addr, _pool_state) in pools {
                        if to_addr.eq_ignore_ascii_case(&pool_addr) {
                            // Creator is approving router to spend LP tokens = rug pull setup
                            return CreatorFunctionType::LiquidityManagement;
                        }
                    }
                }
            }
        }
        
        // Regular approval (not LP token or not to router)
        CreatorFunctionType::Other("approve".to_string())
    }
}