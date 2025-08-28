use super::data_models::ProcessedTransaction;
use alloy_primitives::{Address, U256};

/// Transaction types based on what the transaction does
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionType {
    Transfer,
    Swap,
    AddLiquidity,
    RemoveLiquidity,
    Approval,
    ContractCreation,
    ContractInteraction,
    Failed,
    Unknown,
}

impl ToString for TransactionType {
    fn to_string(&self) -> String {
        match self {
            TransactionType::Transfer => "transfer".to_string(),
            TransactionType::Swap => "swap".to_string(),
            TransactionType::AddLiquidity => "add_liquidity".to_string(),
            TransactionType::RemoveLiquidity => "remove_liquidity".to_string(),
            TransactionType::Approval => "approval".to_string(),
            TransactionType::ContractCreation => "contract_creation".to_string(),
            TransactionType::ContractInteraction => "contract_interaction".to_string(),
            TransactionType::Failed => "failed".to_string(),
            TransactionType::Unknown => "unknown".to_string(),
        }
    }
}

pub struct TransactionClassifier;

impl TransactionClassifier {
    pub fn new() -> Self {
        Self
    }
    
    /// Classify a transaction based on its content and events
    pub fn classify(&self, tx: &ProcessedTransaction) -> TransactionType {
        // Failed transactions
        if tx.status != "success" && tx.status != "1" {
            return TransactionType::Failed;
        }
        
        // Contract creation
        if tx.to_address.is_none() || tx.contract_address.is_some() {
            return TransactionType::ContractCreation;
        }
        
        // Check for swaps
        if self.is_swap(tx) {
            return TransactionType::Swap;
        }
        
        // Check for liquidity operations
        if self.is_add_liquidity(tx) {
            return TransactionType::AddLiquidity;
        }
        
        if self.is_remove_liquidity(tx) {
            return TransactionType::RemoveLiquidity;
        }
        
        // Check for transfers
        if self.is_transfer(tx) {
            return TransactionType::Transfer;
        }
        
        // Check for approvals
        if self.is_approval(tx) {
            return TransactionType::Approval;
        }
        
        // Default to contract interaction if there's a to address and input data
        if tx.to_address.is_some() && !tx.input.is_empty() && tx.input.len() > 4 {
            return TransactionType::ContractInteraction;
        }
        
        TransactionType::Unknown
    }
    
    /// Identify specific actions taken in the transaction
    pub fn identify_actions(&self, tx: &ProcessedTransaction) -> Vec<String> {
        let mut actions = Vec::new();
        
        // Token transfers
        if !tx.erc20_transfers.is_empty() {
            actions.push("erc20_transfer".to_string());
        }
        
        if !tx.erc721_transfers.is_empty() {
            actions.push("nft_transfer".to_string());
        }
        
        if !tx.erc1155_transfers.is_empty() {
            actions.push("multi_token_transfer".to_string());
        }
        
        // ETH transfers
        if tx.value > U256::ZERO || !tx.eth_transfers.is_empty() {
            actions.push("eth_transfer".to_string());
        }
        
        // DEX operations
        if !tx.uniswap_v2_swaps.is_empty() || !tx.uniswap_v3_swaps.is_empty() || !tx.uniswap_v4_swaps.is_empty() {
            actions.push("dex_swap".to_string());
        }
        
        if !tx.mints.is_empty() || !tx.uniswap_v3_mints.is_empty() {
            actions.push("liquidity_add".to_string());
        }
        
        if !tx.burns.is_empty() || !tx.uniswap_v3_burns.is_empty() {
            actions.push("liquidity_remove".to_string());
        }
        
        // Approvals
        if !tx.approvals.is_empty() {
            actions.push("token_approval".to_string());
        }
        
        // Contract events
        if !tx.contract_creation_events.is_empty() {
            actions.push("contract_deployed".to_string());
        }
        
        if !tx.owner_events.is_empty() {
            actions.push("ownership_change".to_string());
        }
        
        if !tx.trading_enabled_events.is_empty() {
            actions.push("trading_enabled".to_string());
        }
        
        if !tx.trading_disabled_events.is_empty() {
            actions.push("trading_disabled".to_string());
        }
        
        // Position management
        if !tx.uniswap_v3_increases.is_empty() {
            actions.push("position_increased".to_string());
        }
        
        if !tx.uniswap_v3_decreases.is_empty() {
            actions.push("position_decreased".to_string());
        }
        
        actions
    }
    
    fn is_swap(&self, tx: &ProcessedTransaction) -> bool {
        !tx.uniswap_v2_swaps.is_empty() ||
        !tx.uniswap_v3_swaps.is_empty() ||
        !tx.uniswap_v4_swaps.is_empty() ||
        // Check for simultaneous token in/out transfers that indicate a swap
        (tx.erc20_transfers.len() >= 2 && self.has_reciprocal_transfers(tx))
    }
    
    fn is_add_liquidity(&self, tx: &ProcessedTransaction) -> bool {
        !tx.mints.is_empty() ||
        !tx.uniswap_v3_mints.is_empty() ||
        !tx.deposits.is_empty() ||
        // Check for multiple token transfers to same contract
        self.has_multiple_tokens_to_same_address(tx)
    }
    
    fn is_remove_liquidity(&self, tx: &ProcessedTransaction) -> bool {
        !tx.burns.is_empty() ||
        !tx.uniswap_v3_burns.is_empty() ||
        !tx.withdraws.is_empty() ||
        !tx.uniswap_v3_decreases.is_empty()
    }
    
    fn is_transfer(&self, tx: &ProcessedTransaction) -> bool {
        // Simple ETH transfer
        if tx.value > U256::ZERO && tx.input.is_empty() {
            return true;
        }
        
        // Single token transfer
        if tx.erc20_transfers.len() == 1 && tx.approvals.is_empty() {
            return true;
        }
        
        // NFT transfer
        if !tx.erc721_transfers.is_empty() || !tx.erc1155_transfers.is_empty() {
            return true;
        }
        
        false
    }
    
    fn is_approval(&self, tx: &ProcessedTransaction) -> bool {
        !tx.approvals.is_empty() && tx.erc20_transfers.is_empty()
    }
    
    fn has_reciprocal_transfers(&self, tx: &ProcessedTransaction) -> bool {
        // Check if there are transfers going in opposite directions
        // indicating a swap pattern
        if tx.erc20_transfers.len() < 2 {
            return false;
        }
        
        // Look for pattern where tokens flow: A->Contract->B
        for i in 0..tx.erc20_transfers.len() {
            for j in i+1..tx.erc20_transfers.len() {
                let t1 = &tx.erc20_transfers[i];
                let t2 = &tx.erc20_transfers[j];
                
                // Different tokens and opposite flow directions
                if t1.token_address != t2.token_address &&
                   (t1.to_address == t2.from_address || t1.from_address == t2.to_address) {
                    return true;
                }
            }
        }
        
        false
    }
    
    fn has_multiple_tokens_to_same_address(&self, tx: &ProcessedTransaction) -> bool {
        if tx.erc20_transfers.len() < 2 {
            return false;
        }
        
        // Group transfers by recipient
        let mut recipients: std::collections::HashMap<Address, std::collections::HashSet<Address>> = std::collections::HashMap::new();
        
        for transfer in &tx.erc20_transfers {
            recipients.entry(transfer.to_address)
                .or_insert_with(std::collections::HashSet::new)
                .insert(transfer.token_address);
        }
        
        // Check if any recipient received multiple different tokens
        recipients.values().any(|tokens| tokens.len() >= 2)
    }
}