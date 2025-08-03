/// Creator Transaction Classifier
/// 
/// Identifies and categorizes transactions from known token creators

use std::sync::Arc;
use crate::mempool_fetcher::MempoolTransaction;
use crate::token_tracking::TokenTrackingCache;
use super::CreatorFunctionType;

pub struct CreatorTransactionRouter {
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl CreatorTransactionRouter {
    pub fn new(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        Self { token_cache }
    }

    /// Identify the type of function being called
    pub fn identify_function(&self, tx: &MempoolTransaction) -> CreatorFunctionType {
        if tx.input.len() < 4 {
            return CreatorFunctionType::Other("unknown".to_string());
        }

        let selector = &tx.input[0..4];
        
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
}