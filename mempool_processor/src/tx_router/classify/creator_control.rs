use crate::function_detector::CreatorFunctionType;
use crate::mempool_fetcher::MempoolTransaction;
use crate::token_tracking::TokenTrackingCache;
/// Creator Transaction Classifier
///
/// Identifies and categorizes transactions from known token creators
use std::sync::Arc;

pub struct CreatorTransactionRouter;

impl CreatorTransactionRouter {
    pub fn new(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        let _ = token_cache;
        Self
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
                let selector_hex = format!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    selector[0], selector[1], selector[2], selector[3]
                );
                CreatorFunctionType::Other(selector_hex)
            }
        }
    }
}
