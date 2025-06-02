use ethers::prelude::*;
use eyre::Result;
use std::sync::Arc;
use tracing::{info, debug};
use serde::{Serialize, Deserialize};
use std::str::FromStr;

/// Configuration for transaction fetching
#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub blocks_to_scan: u64,
    pub max_transactions_per_block: usize,
    pub min_gas_used: u64,
    pub require_logs: bool,
    pub require_internal_txns: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            blocks_to_scan: 5,
            max_transactions_per_block: 10,
            min_gas_used: 100_000, // Skip simple transfers
            require_logs: true,     // Focus on DeFi transactions
            require_internal_txns: false,
        }
    }
}

/// Transaction candidate for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCandidate {
    pub hash: String,
    pub block_number: u64,
    pub transaction_index: u64,
    pub from: String,
    pub to: Option<String>,
    pub value: String,
    pub gas_used: u64,
    pub log_count: usize,
    pub has_internal_txns: bool,
    pub transaction_type: String,
}

/// Fetches transactions from recent blocks for validation testing
pub struct TransactionFetcher {
    provider: Arc<Provider<Http>>,
    config: FetchConfig,
}

impl TransactionFetcher {
    pub fn new(provider: Arc<Provider<Http>>, config: FetchConfig) -> Self {
        Self { provider, config }
    }

    /// Fetch recent transactions suitable for testing
    pub async fn fetch_test_transactions(&self) -> Result<Vec<TransactionCandidate>> {
        let latest_block = self.provider.get_block_number().await?;
        info!("Fetching test transactions from latest block: {}", latest_block);

        let mut candidates = Vec::new();
        let start_block = latest_block.saturating_sub(U64::from(self.config.blocks_to_scan));

        // Convert U64 to u64 for iteration
        let start_block_u64 = start_block.as_u64();
        let latest_block_u64 = latest_block.as_u64();

        for block_num in start_block_u64..=latest_block_u64 {
            let block_candidates = self.fetch_from_block(block_num).await?;
            candidates.extend(block_candidates);
            
            if candidates.len() >= self.config.max_transactions_per_block * self.config.blocks_to_scan as usize {
                break;
            }
        }

        info!("Found {} transaction candidates across {} blocks", 
              candidates.len(), self.config.blocks_to_scan);
        
        Ok(candidates)
    }

    /// Fetch transactions from a specific block
    async fn fetch_from_block(&self, block_number: u64) -> Result<Vec<TransactionCandidate>> {
        let block = self.provider
            .get_block_with_txs(BlockNumber::Number(block_number.into()))
            .await?;

        let mut candidates = Vec::new();
        
        if let Some(block) = block {
            debug!("Scanning block {} with {} transactions", 
                   block_number, block.transactions.len());

            for (tx_index, tx) in block.transactions.iter().enumerate() {
                if candidates.len() >= self.config.max_transactions_per_block {
                    break;
                }

                // Get transaction receipt for additional filtering
                if let Ok(Some(receipt)) = self.provider.get_transaction_receipt(tx.hash).await {
                    if self.is_suitable_for_testing(tx, &receipt).await {
                        let candidate = self.create_candidate(tx, &receipt, block_number, tx_index).await?;
                        candidates.push(candidate);
                    }
                }
            }
        }

        Ok(candidates)
    }

    /// Check if transaction is suitable for testing
    async fn is_suitable_for_testing(&self, tx: &Transaction, receipt: &TransactionReceipt) -> bool {
        // Check gas usage threshold
        if receipt.gas_used.unwrap_or_default().as_u64() < self.config.min_gas_used {
            return false;
        }

        // Check for logs if required
        if self.config.require_logs && receipt.logs.is_empty() {
            return false;
        }

        // Check transaction status
        if receipt.status != Some(1.into()) {
            return false;
        }

        // Skip contract creation if we want to focus on interactions
        if tx.to.is_none() && self.config.require_logs {
            return false;
        }

        true
    }

    /// Create a transaction candidate from transaction and receipt
    async fn create_candidate(
        &self,
        tx: &Transaction,
        receipt: &TransactionReceipt,
        block_number: u64,
        tx_index: usize,
    ) -> Result<TransactionCandidate> {
        let tx_type = self.classify_transaction_type(tx, receipt);
        let has_internal_txns = self.check_for_internal_transactions(tx).await;

        Ok(TransactionCandidate {
            hash: format!("0x{:x}", tx.hash),
            block_number,
            transaction_index: tx_index as u64,
            from: format!("0x{:x}", tx.from),
            to: tx.to.map(|addr| format!("0x{:x}", addr)),
            value: tx.value.to_string(),
            gas_used: receipt.gas_used.unwrap_or_default().as_u64(),
            log_count: receipt.logs.len(),
            has_internal_txns,
            transaction_type: tx_type,
        })
    }

    /// Classify transaction type for better test categorization
    fn classify_transaction_type(&self, tx: &Transaction, receipt: &TransactionReceipt) -> String {
        // Contract creation
        if tx.to.is_none() {
            return "Contract Creation".to_string();
        }

        // Check for common DeFi patterns in logs
        for log in &receipt.logs {
            // ERC20 Transfer
            if log.topics.len() >= 3 {
                if let Ok(transfer_topic) = H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef") {
                    if log.topics[0] == transfer_topic {
                        return "Token Transfer".to_string();
                    }
                }
            }
            
            // Uniswap V2 Swap
            if log.topics.len() >= 1 {
                if let Ok(swap_v2_topic) = H256::from_str("0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822") {
                    if log.topics[0] == swap_v2_topic {
                        return "Uniswap V2 Swap".to_string();
                    }
                }
            }
            
            // Uniswap V3 Swap
            if log.topics.len() >= 1 {
                if let Ok(swap_v3_topic) = H256::from_str("0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67") {
                    if log.topics[0] == swap_v3_topic {
                        return "Uniswap V3 Swap".to_string();
                    }
                }
            }
        }

        // Simple ETH transfer
        if tx.input.is_empty() || tx.input == Bytes::from_static(&[0]) {
            return "ETH Transfer".to_string();
        }

        "Contract Interaction".to_string()
    }

    /// Check if transaction has internal transactions (simplified check)
    async fn check_for_internal_transactions(&self, tx: &Transaction) -> bool {
        // For now, assume complex transactions with significant gas usage have internal txns
        // This could be enhanced with actual trace checking
        tx.gas.as_u64() > 200_000
    }

    /// Get diverse transaction types for comprehensive testing
    pub async fn fetch_diverse_test_set(&self) -> Result<Vec<TransactionCandidate>> {
        let all_candidates = self.fetch_test_transactions().await?;
        
        // Group by transaction type
        let mut by_type: std::collections::HashMap<String, Vec<TransactionCandidate>> = 
            std::collections::HashMap::new();
        
        for candidate in all_candidates {
            by_type.entry(candidate.transaction_type.clone())
                   .or_default()
                   .push(candidate);
        }

        // Take a few from each type for diversity
        let mut diverse_set = Vec::new();
        for (tx_type, mut candidates) in by_type {
            candidates.sort_by(|a, b| b.gas_used.cmp(&a.gas_used)); // Prefer higher gas usage
            let take_count = std::cmp::min(3, candidates.len()); // Max 3 per type
            diverse_set.extend(candidates.into_iter().take(take_count));
            
            info!("Selected {} {} transactions", take_count, tx_type);
        }

        info!("Created diverse test set with {} transactions", diverse_set.len());
        Ok(diverse_set)
    }
} 