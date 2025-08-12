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

    /// Get the function type from pre-categorized transaction
    pub fn get_function_type(&self, tx: &MempoolTransaction) -> CreatorFunctionType {
        // Use the pre-categorized function type from the function detector
        if let Some(ref category) = tx.function_category {
            category.clone()
        } else {
            // Fallback: no function category detected
            if tx.input.is_empty() || tx.input.len() < 4 {
                CreatorFunctionType::Other("eth_transfer".to_string())
            } else {
                let selector = &tx.input[0..4];
                let selector_hex = format!("{:02x}{:02x}{:02x}{:02x}", 
                    selector[0], selector[1], selector[2], selector[3]);
                CreatorFunctionType::Other(selector_hex)
            }
        }
    }
}